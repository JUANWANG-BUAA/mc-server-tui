# mc-server-tui

Rust workspace for managing local Minecraft Java server instances on macOS and Linux.

## Current implementation slice

The workspace has shared API and error contracts, a reusable async daemon client, local `doctor` checks, a Paper Fill v3 adapter, a foreground Unix-socket daemon, a verified downloader, persistent Paper instance creation/listing, durable background tasks, and a per-instance Java process supervisor. A stable `mc` launcher now loads `config.toml`, probes the daemon's version/API range, and installs a user-level launchd (macOS) or systemd (Linux) service without sudo. Creation returns a task ID; metadata resolution, download, verification, and installation proceed asynchronously. The TUI starts in a Claude Code-style session shell with a command transcript, a bottom composer, command completion, persistent status dock, live task progress, and CPU/RSS resource readouts for the selected instance. It also keeps a structured Paper creation wizard, instance detail with lifecycle controls and a live console, task detail with byte-level progress and cancellation, classically browsable panels, and a world-backup manager with pagination, integrity verification, restore preview, explicit confirmation, and durable restore status. Seven low-contrast themes (`Claude`, `DeepSeek`, `Codex`, `Kimi`, `PCL2`, `Mono`, `Ocean`) can be switched at runtime, and the status dock includes a small pixel Steve head that blinks while work is active and dozes when idle. A new `mc-modpack` analyzer inspects Modrinth `.mrpack` files read-only for Minecraft version, loader, Mod environment markers, unsafe paths, hashes, and download hosts. Network and process lifecycle requests run asynchronously, so input remains responsive while requests are pending. Startup recovery registers valid committed directories, quarantines interrupted staging data, and marks unfinished tasks interrupted.

```sh
cargo build --workspace

# Preview the generated config and launchd/systemd unit without touching the system:
./target/debug/mc setup --dry-run

# Write config.toml, install the user service, and wait for the daemon:
./target/debug/mc setup

# Then open the TUI:
./target/debug/mc

# Inspect the local service or forward to the text CLI:
./target/debug/mc service status --json
./target/debug/mc version --json
./target/debug/mc status --json
./target/debug/mc doctor
./target/debug/mc doctor --json
./target/debug/mc cli status
./target/debug/mc cli doctor
./target/debug/mc cli paper versions
./target/debug/mc cli instance list
./target/debug/mc cli task list
```

The source-build service points at `target/debug/mc`; it is for development, not a production install. `mc setup --no-service` writes config only, and `mc service install --dry-run` prints the exact launchd/systemd plan. The daemon can still be started manually with `cargo run -p mc-daemon` or `./target/debug/mc daemon run`.

The default Paper User-Agent is recorded in `config.toml` and includes the project identity. Override it for a real deployment (or for a fork) with:

```sh
./target/debug/mc setup --user-agent 'my-server-tool/0.1.0 (https://example.com/contact)'
# or, for tests/automation:
export MC_SERVER_TUI_USER_AGENT='my-server-tool/0.1.0 (https://example.com/contact)'
```

The default points at this project repository. Forks or shared deployments should set a deployment-specific contact with `--user-agent`. The daemon socket is per-user and restricted to the current user. `MC_SERVER_TUI_SOCKET` and `MC_SERVER_TUI_STATE_DIR` still override the config; a custom socket must live in a directory with mode `0700`. The config path can be selected with `--config` or `MC_SERVER_TUI_CONFIG`.

## Module boundaries

- `mc-core`: domain values, stable errors, provider trait, and transport-neutral request/response types.
- `mc-client`: bounded async Unix-socket client shared by the CLI and TUI.
- `mc-config`: `config.toml`, platform paths, User-Agent defaults, and state/socket/cache resolution shared by every frontend.
- `mc`: stable launcher for TUI/daemon/CLI, version/API probe, setup, and launchd/systemd user-service management.
- `provider-paper`: Fill v3 HTTP access and conversion to core types.
- `mc-download`: exact artifact fetching, redirect/DNS checks, safe range resume, SHA-256 validation, content-addressed cache, byte-level progress, and cooperative cancellation.
- `mc-modpack`: bounded read-only `.mrpack` parsing, path safety, loader/version inspection, Mod environment classification, and adapter-report generation.
- `mc-system`: local Java discovery, port check, data directory and disk checks.
- `mc-instance`: versioned `instance.toml`/`lock.json`, atomic staging, SQLite index and startup recovery.
- `mc-task`: durable task/event storage, phase and transfer-progress reporting, cooperative cancellation, bounded workers, per-instance operation locks, and restart interruption recovery.
- `mc-supervisor`: per-instance Java start/stop, resident runner, PID identity verification, private console socket, readiness observation, and bounded log reads.
- `mc-daemon`: bounded local API transport, request dispatch, durable tasks, and process-supervisor integration; user-level service installation is handled by `mc`.
- `mc-cli`: command parsing and text presentation through the local API.
- `mc-tui`: session transcript and composer, command completion and palette, guided Paper creation flow, task progress/cancel detail, instance detail, lifecycle actions, world backups, Modrinth adapter reports, incremental console output, process resource readouts, mouse interaction, and runtime themes.

Instance storage and crash reconciliation rules are recorded in [docs/INSTANCE_SCHEMA.md](docs/INSTANCE_SCHEMA.md). The guided terminal interaction contract is in [docs/INTERACTION_DESIGN.md](docs/INTERACTION_DESIGN.md), the read-only `.mrpack` adapter contract is in [docs/MODPACK_IMPORT.md](docs/MODPACK_IMPORT.md), and TUI layout snapshots plus the PTY/fake-daemon test plan are tracked in [docs/TUI_TESTING.md](docs/TUI_TESTING.md). The documented-versus-implemented audit is in [docs/IMPLEMENTATION_GAPS.md](docs/IMPLEMENTATION_GAPS.md), and the macOS/Linux packaging, service-install, and hot-update plan is in [docs/DISTRIBUTION.md](docs/DISTRIBUTION.md).

The TUI opens in the session shell. The session transcript is above the composer; the status dock below it shows the selected instance, configured memory, CPU/RSS for a running server, and live progress bars for up to two tasks. Type `/` for inline command completion, or type free text to send it to the selected instance's console. Useful commands include `/help`, `/status`, `/instances`, `/use <序号|名称>`, `/open`, `/start`, `/stop`, `/console <命令>`, `/tasks`, `/task <序号>`, `/logs`, `/cancel [序号]`, `/create`, `/import`, `/backup`, `/backups`, `/refresh`, `/theme <name>`, `/classic`, and `/quit`.

- `Enter` sends, `Tab` completes a slash command, `←/→` moves the composer caret, `Home/End` jumps to line start/end, `Delete` removes forward, `↑/↓` walks history or the completion list, `PgUp/PgDn` scrolls the transcript, `Ctrl-A/Ctrl-E` jumps to line start/end, `Ctrl-L` clears the session, `Ctrl-K` opens the full command palette, `F1` opens keyboard help, and `F2` cycles themes. When a completion is highlighted, `Enter` executes the highlighted command; an exact command name always wins over the highlighted suggestion.
- The selected instance is refreshed in the background so the status dock can show its address, server state, CPU, and RSS without opening a separate page. The dock also shows a contextual quick-action line; `/logs` tails the last 12 console lines inline and `/cancel [n]` cancels a download without opening task detail. Long list outputs such as `/help`, `/status`, `/tasks`, `/instances`, and `/theme` scroll to their first line; `/logs` keeps the tail in view.
- `/create` opens the structured Paper wizard. Versions are selected with `↑/↓` (not left/right), `PgUp/PgDn` pages the list, `Home/End` jumps to either end, and a scrollbar plus `start-end/total` counter make the list state explicit. Tab still moves between fields and `←/→` remains a compatibility alias for changing a choice.
- `/classic` (or `t` in the classic view) switches to the two-panel instance/task dashboard. Mouse wheel scrolls lists, single click selects, double click opens, and clicking the EULA row toggles it.
- The server detail view keeps `s` start, `x` graceful stop, `:` console input, `b` backup, `l` world/backup manager, and `r` refresh. The backup manager keeps `v` verify, Enter restore preview, `c` + exact `RESTORE`, `m` load more, and `b` create backup. Task detail keeps `x` twice to cancel and `r` to fetch events after the current sequence.

Theme selection is available via `F2`, `/theme <name>`, the command palette, or the `MC_SERVER_TUI_THEME` environment variable (`claude`, `deepseek`, `codex`, `kimi`, `pcl2`, `mono`, `ocean`). The chosen theme is saved under the platform config directory (`org.mc-server-tui.mc-server-tui/theme`) and restored on the next launch. All themes use a near-black page; color is reserved for focus, status, active progress, and the pixel mascot. The pixel Steve head keeps canonical Minecraft colors (brown hair, skin, white/blue eyes, brown mouth); only the sparkle or `z`/`Z` above it uses the theme accent. It blinks on a short cycle, shows a sparkle while tasks run, and dozes when idle.

## Distribution status

Phase 0 and the Phase 1 packaging loop are implemented: `mc` launcher, `config.toml` with a default User-Agent, `version --json`, `ServerInfo`/API-range probe, launchd/systemd user-service management, a versioned tarball installer with SHA-256 verification, `mc uninstall`, a release-manifest generator, and a rendered Homebrew formula template. A macOS/Linux CI workflow builds, packages, installs, and uninstalls a local artifact. There is still no published/signed artifact, public Homebrew tap, apt/dnf repository, macOS notarization, or hot updater; those remain Phase 2–4. Release steps and the draft-release review checklist are in [docs/RELEASING.md](docs/RELEASING.md). The implementation audit and the distribution/hot-update design are recorded in [docs/IMPLEMENTATION_GAPS.md](docs/IMPLEMENTATION_GAPS.md) and [docs/DISTRIBUTION.md](docs/DISTRIBUTION.md). The supervisor runner already survives daemon restarts, which is the key prerequisite for updating the daemon without killing running Minecraft servers.

Release packaging and the versioned installer can be exercised locally without a public URL:

```sh
cargo build --workspace --release
python3 scripts/package_release.py \
  --version 0.1.0 --target aarch64-apple-darwin \
  --base-url "file://$PWD/dist/0.1.0"
./scripts/install.sh \
  --manifest-url "file://$PWD/dist/0.1.0/release-manifest.json" \
  --prefix /tmp/mcst-prefix --root /tmp/mcst-root
/tmp/mcst-prefix/bin/mc version --json
/tmp/mcst-prefix/bin/mc uninstall
```

This is an unsigned alpha path: the installer verifies SHA-256 but warns that no signature was checked. Homebrew formula rendering is in `packaging/homebrew/`; publishing a tap is still Phase 2.

Run the UI renderer and state tests with `cargo test -p mc-tui`. Build the TUI and run `python3 scripts/tui_pty_smoke.py`, `python3 scripts/tui_task_detail_pty.py`, `python3 scripts/tui_guided_create_pty.py`, and `python3 scripts/tui_instance_detail_pty.py` to verify command-palette creation, option loading, byte progress, cancellation, task event resume, backup verification/restore confirmation, and instance lifecycle interactions through PTY fake daemons. Lifecycle API details and the process identity contract are in [docs/SUPERVISION.md](docs/SUPERVISION.md).
