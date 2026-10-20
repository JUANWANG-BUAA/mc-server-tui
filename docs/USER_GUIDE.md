# mc-server-tui User Guide

> Applies to `v0.2.0` (unsigned alpha pre-release).
> This guide describes the shipped alpha. It is not a promise of production
> stability, data recovery, compatibility with every server pack, or support
> for every Minecraft version.

## 1. Read this before hosting a world

`mc-server-tui` is an independent terminal-based Minecraft Java server manager.
It is not affiliated with Mojang, Microsoft, PaperMC, Modrinth, Fabric, or
Forge. Minecraft and related trademarks belong to their respective owners.

You are responsible for:

- reviewing and accepting the Minecraft EULA yourself;
- using only server software, plugins, mods, packs, world data, and player data
  you are entitled to use;
- keeping independent backups outside this application;
- firewall, port forwarding, public exposure, passwords, permissions, and host
  security;
- validating that a plugin, mod, server pack, or update works before relying on
  it for a world you care about.

This alpha is unsigned. Release artifacts have SHA-256 integrity checks, but no
code signature or Apple notarization. Server software and third-party add-ons
execute with the permissions of your user account. Use a dedicated account and
do not install untrusted files. The software is provided under the repository's
MIT OR Apache-2.0 license, without a guarantee of availability or recovery.

## 2. Supported scope

| Platform | `v0.2.0` artifact |
| --- | --- |
| macOS 13+ Apple Silicon | `aarch64-apple-darwin` |
| Linux x86_64 | `x86_64-unknown-linux-gnu` |
| Linux arm64 | `aarch64-unknown-linux-gnu` |
| Intel macOS | Not published in this release |

The TUI relies on a Unix socket and Unix process behavior. Windows is not
supported by this release. Use a terminal at least 80×24; 40×12 is usable with
reduced chrome.

Java must match the selected Minecraft and server version. The creation flow
shows detected runtimes and recommends one, but you remain responsible for the
server version, plugins/mods, and client compatibility.

## 3. Install

### Release installer

`v0.2.0` is a pre-release, so use the tag-specific URL:

```sh
TAG=v0.2.0
curl -fsSL "https://github.com/JUANWANG-BUAA/mc-server-tui/releases/download/$TAG/install.sh" | sh
mc setup
mc
```

`mc setup` writes a user configuration, checks Java, installs a launchd or
systemd user service when available, starts the daemon, and opens the TUI from
`mc`. To write configuration without installing a background service:

```sh
mc setup --no-service
mc daemon run   # terminal 1
mc tui          # terminal 2
```

### Build from source

The repository pins Rust 1.89.0:

```sh
git clone https://github.com/JUANWANG-BUAA/mc-server-tui.git
cd mc-server-tui
cargo build --workspace
./target/debug/mc setup --dry-run
./target/debug/mc setup
./target/debug/mc
```

Verify the active version and protocol:

```sh
mc version --json
mc status --json
mc doctor --json
```

## 4. Create the first server

Start `mc`, then enter:

```text
/create
```

The guided Paper flow asks for:

1. instance name and Minecraft version;
2. Java runtime, memory range, and server port;
3. a review of the artifact, download source, memory, port, and EULA.

New date-based releases such as `26.3` and `26.2` are sorted before legacy
`1.21.x` releases in the version picker. Choose the exact version your players
will use. EULA acceptance is explicit and is never selected automatically.

After submission, use:

```text
/tasks
/task <number>
```

to inspect download progress, source, speed, phases, and cancellation.

For a Fabric server:

```text
/fabric MyFabricServer 1.21.11 --accept-eula
```

The Fabric path resolves a stable loader and compatible Java, checks the port,
and creates a managed instance.

## 5. Session shell

The default screen is a session shell. Text without a leading slash is sent to
the current Minecraft console; slash commands manage the application.

Common commands:

| Command | Purpose |
| --- | --- |
| `/help`, `F1` | Show all commands and keys |
| `/instances`, `/use <number-or-name>` | List and select an instance |
| `/open` | Open the selected instance detail |
| `/start`, `/stop`, `/restart` | Start, gracefully stop, or controlled restart |
| `/logs` | Tail console output in the session |
| `/console <command>` | Send one command to the Minecraft console |
| `/tasks`, `/task <n>`, `/cancel [n]` | Inspect or cancel durable tasks |
| `/backup`, `/backups` | Create or manage stopped-world backups |
| `/config` | Edit four common `server.properties` values |
| `/players [action] [player]` | OP, whitelist, ban, pardon, kick, LuckPerms helpers |
| `/rg [-i] [-e] <pattern>` | Search current and rotated logs |
| `/connect`, `/monitor` | Inspect join addresses and server state |
| `/mods`, `/import`, `/export` | Add-ons and server-pack workflows |
| `/pages`, `/page <n>`, `/close [n]` | Workspace pages |
| `/theme [name]`, `F2` | Themes |
| `/quit` | Exit the TUI; the server and tasks continue |

Useful keys: `Enter` submit, `Tab` complete, `←/→` move the cursor,
`Home/End` line boundaries, `↑/↓` history or selection, `Ctrl-K` command
palette, `F2` theme, and `F1` help.

## 6. Lifecycle and readiness

- `/start` is idempotent and verifies the Java path, server JAR, port, and
  process identity.
- `/stop` sends Minecraft's graceful `stop` command and waits for the verified
  process to exit. Timeout does not force-kill Java.
- `/restart` requires a verified starting or running process, gracefully stops
  it, waits for a clean stopped state, then starts the same instance. It does
  not start a replacement after an unclean stop.
- Readiness requires the server's `Done (` marker. An open port alone is not
  considered ready.
- Closing the TUI or daemon does not kill the resident runner or Java process.

The instance detail page (`/open`) exposes lifecycle controls, console input,
logs, process identity, CPU/RSS samples, and alerts. Press `R` there for a
controlled restart and `c` for configuration.

## 7. Common configuration

Run:

```text
/config
```

The current editor supports MOTD, maximum players, difficulty, and view
distance. It shows a diff before writing, preserves unknown keys and comments,
and rejects an external file change instead of overwriting it. Saving while
the server is running does not restart it. Use `/restart` or `R` when you want
the new values to take effect.

Raw editing, the complete property catalog, and per-version validation are not
implemented. For unsupported properties, edit `server.properties` while the
server is stopped and keep your own backup.

## 8. Backups and restore

Backups are stopped-world only.

1. Stop the server and wait for `stopped`.
2. Run `/backup`.
3. Open `/backups` and select a verified snapshot.
4. Press `v` to verify, then `Enter` to build a restore plan.
5. Review affected worlds, added/replaced/removed roots, staging space, and
   safety steps.
6. Type the exact `RESTORE` confirmation and submit.

Restore is transactional and keeps rollback evidence until commit verification.
The server remains stopped after restore. Start it explicitly and validate the
world with a Minecraft client. No automated online backup or scheduler exists
in this release.

## 9. Players, logs, and workspaces

Examples:

```text
/players list
/players op Steve
/players allow Alex
/players whitelist-on
/players kick Steve reason
```

Online-name actions may need Mojang profile lookup and can fail on constrained
networks. The TUI reports server responses or timeout errors; it does not
pretend the action succeeded.

`/rg` searches current and one rotated log. `/logs` shows recent output.
`/monitor` shows game status and online count. CPU/RSS history is local
observability, not TPS/MSPT or JVM heap telemetry.

Workspace pages preserve log filters and reading positions. Closing a page does
not stop the server.

## 10. Fabric, add-ons, and server packs

- `/mods search <name>` and `/mods install <name>` install supported Modrinth
  Paper plugins or Fabric mods, required catalog dependencies, and validated
  JARs. The instance must be stopped.
- `/import` previews a `.mrpack` or server-pack ZIP. `/import <path> --apply`
  stages and verifies compatible `mods`, `plugins`, and `config`; it refuses
  conflicts, unsafe paths, world/identity data, `server.jar`, and
  `server.properties`.
- `/export <path.zip>` creates a server add-on/config bundle with a SHA-256
  manifest. It is not a client launcher pack.

Imports do not execute pack scripts and do not prove that the combined
plugins/mods will start. Validate after import. Forge/NeoForge and CurseForge
file-ID materialization are not implemented.

## 11. CLI

The CLI uses the same typed daemon API:

```sh
mc cli instance list
mc cli instance start <instance-id>
mc cli instance stop <instance-id> --timeout-seconds 20
mc cli instance restart <instance-id> --timeout-seconds 20
mc cli backup create <instance-id>
mc cli backup list <instance-id>
mc cli task list
mc cli modpack inspect /path/server.mrpack
```

CLI lifecycle output is an observation; a server can still be `starting`
while the command returns. Poll dashboard/detail status or use the TUI until
the readiness marker is observed.

## 12. Service, update, and data

```sh
mc service install
mc service status --json
mc service restart
mc service uninstall
mc update status
mc update rollback --dry-run
```

The user service supervises the daemon. Resident Java runners are intentionally
separate. `mc service restart` restarts the daemon and leaves running servers
alone. Uninstall preserves instance data unless a destructive purge option is
explicitly used.

`mc doctor` prints the exact data directory. Instance worlds live under
`<state>/instances/<uuid>/server/`. Do not edit the UUID directory while the
server is running. Configuration, socket, state, and cache paths can be
overridden in `config.toml` or with documented environment variables.

## 13. Troubleshooting

Run first:

```sh
mc version --json
mc doctor --json
mc status --json
```

The instance detail recognizes common startup failures from the current log:

| Symptom | First action |
| --- | --- |
| `UnsupportedClassVersionError` | Select Java compatible with the server/MC version |
| Port bind failure | Stop the process using the port or choose another port |
| EULA warning | Review and explicitly accept the Minecraft EULA |
| `OutOfMemoryError` | Raise the instance maximum memory within host capacity |
| Missing/corrupt JAR | Reinstall the locked server build |

For unknown failures, use `/open` and `/rg` to inspect the full current and
rotated logs. Do not blindly force-kill an unknown PID; the supervisor refuses
to signal processes whose identity cannot be verified.

If the daemon is unavailable:

```sh
mc service status --json
mc service start
mc doctor --json
```

## 14. Network exposure

`/connect` suggests local, LAN, and possible public addresses. It does not
prove that a remote friend can connect. Port forwarding, NAT, firewalls,
carrier restrictions, and DNS are outside the application's control.

Do not expose an administrative host directly to the internet. Run the manager
under a dedicated user, restrict file permissions, keep the daemon socket
private, use a firewall, and review plugin/mod permissions. Minecraft server
software and third-party code may contain vulnerabilities.

## 15. Evidence and current limitations

The `v0.2.0` notes record 217 automatic tests, four PTY suites, protocol checks
from v8–v11, a local manifest installer smoke, and a real Paper restart drill.
These results do not prove every world, plugin combination, network, or client
scenario. A real Minecraft client join and post-restore in-game verification
still need to be performed.

Current gaps include Vanilla, existing-directory takeover, complete
`server.properties` editing, online/scheduled backups, Forge/NeoForge,
CurseForge file-ID downloads, signed/notarized artifacts, a signed self-updater,
state migrations, update-time task draining, and real Linux service validation.

## 16. License and no-warranty notice

The project is licensed under MIT OR Apache-2.0. Those licenses govern the
software; this guide does not add a warranty or support commitment. You are
responsible for backups, access control, hosted content, third-party licenses,
and compliance with the Minecraft EULA and applicable law.

If a world matters, keep an independent backup and test restoration before
changing server software, plugins, mods, or configuration.
