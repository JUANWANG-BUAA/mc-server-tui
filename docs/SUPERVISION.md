# Server lifecycle and process identity contract

> API protocol 6 · implemented in `mc-supervisor` and wired through `mc-daemon`

## API operations

| Request | Response | Behavior |
| --- | --- | --- |
| `GetInstanceDetail { instance_id }` | `InstanceDetail` | Returns immutable instance summary plus observed server state. |
| `CreateWorldBackup { instance_id }` | `TaskQueued` | Queues a durable task, or returns the active backup task for the same instance. The task reserves the stopped instance and holds its lifecycle lease while the backup is copied and verified. |
| `ListWorldBackups { instance_id, limit, cursor }` | `TaskQueued` | Queues bounded metadata discovery and full verification of the returned page. Limits are 1–50; cursors order by creation time and UUID. The task result contains summaries and an optional next cursor. |
| `VerifyWorldBackup { instance_id, backup_id }` | `TaskQueued` | Queues a full integrity scan for one backup and returns its bounded summary. |
| `StartServer { instance_id }` | `ServerStatus` | Starts Java or returns the status of the already verified process. |
| `StopServer { instance_id, timeout_seconds }` | `ServerStatus` or structured error | Sends the Minecraft `stop` command and waits up to 25 seconds. A zero timeout selects the 20-second default. |
| `SendConsoleCommand { instance_id, command }` | `ConsoleCommandAccepted` | Sends one line to a ready, verified process. Commands are limited to 512 printable bytes. |
| `ReadServerLogs { instance_id, after_offset, max_bytes }` | `ServerLogs` | Returns at most 64 KiB from the captured console output and the next byte offset. |

Start and stop responses are observations, not long-lived promises. The TUI polls the detail endpoint while a server is starting or stopping. Readiness requires Paper's `Done (` log marker; an open port alone does not mean the game is ready. Before start, the supervisor checks that the configured TCP port can be bound. If it is occupied without a verified supervisor PID, the state is `unknown` and no process is started.

## Process ownership and recovery

- The daemon starts a detached per-instance runner as its own executable in hidden internal mode. The runner starts the configured Java executable in `server/` with `server.jar nogui` and a private JVM marker.
- The runner owns Java's stdin/stdout/stderr pipes and binds a per-user Unix socket under a mode-0700 temporary directory. The socket and its parent are mode 0600/0700. The daemon sends bounded JSON-line console requests over the socket and reconnects after daemon restarts.
- A bounded log queue applies backpressure to Java output instead of allowing output bursts to consume unbounded memory. The runner writes `logs/console.log`, rotates at 32 MiB, and retains one prior segment as `console.log.1`; API reads remain capped at 64 KiB.
- `server/supervisor.json` records the runner PID and identity marker, Java PID when available, start time, stopping flag, exit code, and a startup error. It remains separate from user configuration and artifact identity. The API's `pid` field is the Java PID, not the runner PID.
- A process is controlled only after the runner PID and unique instance marker match. Linux checks `/proc/<pid>/cmdline`; macOS uses a timed `ps` query. Missing or mismatched identity produces `unknown`; the supervisor never signals that PID.
- In-process async locks and `server/.lifecycle.lock` serialize lifecycle changes for one instance. Separate instances remain independent. Lock acquisition retries asynchronously for short record updates. A graceful stop releases the lock after sending `stop`, keeping status and log reads available while the process exits.
- The detached runner is not killed when the daemon or TUI exits. A new daemon can rediscover it, reconnect to its socket, and continue reading logs. The runner records Java's exit code even while no management daemon is running. Stop timeout never force-kills Java.

## Current platform boundary

The macOS process path and daemon-exit reattachment are covered with a real runner child-process test using a fake Java executable. The Linux `/proc` branch is implemented but still needs CI or host validation. User-level daemon installation and boot/login startup through launchd or systemd are not implemented; the daemon must currently be started separately with `cargo run -p mc-daemon` or the installed binary. Reboot auto-start is therefore outside this slice.

Run the focused implementation checks with:

```sh
cargo test -p mc-supervisor
cargo test -p mc-tui
cargo build -p mc-tui
python3 scripts/tui_instance_detail_pty.py
```
