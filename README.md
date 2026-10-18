# mc-server-tui

Rust workspace for managing local Minecraft Java server instances on macOS and Linux.

## Current implementation slice

The workspace has shared API and error contracts, a reusable async daemon client, local `doctor` checks, a Paper Fill v3 adapter, a foreground Unix-socket daemon, a verified downloader, persistent Paper instance creation/listing, durable background tasks, and a per-instance Java process supervisor. Creation returns a task ID; metadata resolution, download, verification, and installation proceed asynchronously. The TUI has a searchable slash-command palette, a guided Paper creation flow backed by live version metadata and automatic Java/port discovery, task details with byte-level download progress and cooperative cancellation, instance details with lifecycle controls and a live console log, and a world-backup manager with pagination, integrity verification, restore preview, explicit confirmation, and durable restore status. A new `mc-modpack` analyzer inspects Modrinth `.mrpack` files read-only for Minecraft version, loader, Mod environment markers, unsafe paths, hashes, and download hosts. Network and process lifecycle requests run asynchronously, so input remains responsive while requests are pending. Startup recovery registers valid committed directories, quarantines interrupted staging data, and marks unfinished tasks interrupted.

```sh
cargo run -p mc-daemon
# In another terminal:
cargo run -p mc-tui
# The text CLI is also available:
cargo run -p mc-cli -- status
cargo run -p mc-cli -- doctor
cargo run -p mc-cli -- paper versions
cargo run -p mc-cli -- paper builds 26.1
cargo run -p mc-cli -- paper builds 26.1 --include-experimental
cargo run -p mc-cli -- paper plan 26.1
cargo run -p mc-cli -- instance list
cargo run -p mc-cli -- instance create "My Server" 26.1 --java /path/to/java --accept-eula
cargo run -p mc-cli -- task list
# Then inspect the task ID printed by create:
cargo run -p mc-cli -- task show TASK_UUID
cargo run -p mc-cli -- task events TASK_UUID
cargo run -p mc-cli -- modpack inspect /path/to/pack.mrpack
```

Paper requires a non-generic User-Agent with a real project contact. Set one before starting the daemon:

```sh
export MC_SERVER_TUI_USER_AGENT='mc-server-tui/0.1.0 (https://your-real-project-url)'
```

Use a real project URL or contact email; do not leave the example value in place. The daemon socket is per-user and restricted to the current user. `MC_SERVER_TUI_SOCKET` can override its location; a custom socket must live in a directory with mode `0700`.

## Module boundaries

- `mc-core`: domain values, stable errors, provider trait, and transport-neutral request/response types.
- `mc-client`: bounded async Unix-socket client shared by the CLI and TUI.
- `provider-paper`: Fill v3 HTTP access and conversion to core types.
- `mc-download`: exact artifact fetching, redirect/DNS checks, safe range resume, SHA-256 validation, content-addressed cache, byte-level progress, and cooperative cancellation.
- `mc-modpack`: bounded read-only `.mrpack` parsing, path safety, loader/version inspection, Mod environment classification, and adapter-report generation.
- `mc-system`: local Java discovery, port check, data directory and disk checks.
- `mc-instance`: versioned `instance.toml`/`lock.json`, atomic staging, SQLite index and startup recovery.
- `mc-task`: durable task/event storage, phase and transfer-progress reporting, cooperative cancellation, bounded workers, per-instance operation locks, and restart interruption recovery.
- `mc-supervisor`: per-instance Java start/stop, resident runner, PID identity verification, private console socket, readiness observation, and bounded log reads.
- `mc-daemon`: bounded local API transport, request dispatch, durable tasks, and process-supervisor integration; daemon service installation remains a later slice.
- `mc-cli`: command parsing and text presentation through the local API.
- `mc-tui`: searchable slash-command palette, guided Paper creation flow, task progress/cancel detail, instance detail, lifecycle actions, world backups, Modrinth adapter reports, and incremental console output.

Instance storage and crash reconciliation rules are recorded in [docs/INSTANCE_SCHEMA.md](docs/INSTANCE_SCHEMA.md). The guided terminal interaction contract is in [docs/INTERACTION_DESIGN.md](docs/INTERACTION_DESIGN.md), the read-only `.mrpack` adapter contract is in [docs/MODPACK_IMPORT.md](docs/MODPACK_IMPORT.md), and TUI layout snapshots plus the PTY/fake-daemon test plan are tracked in [docs/TUI_TESTING.md](docs/TUI_TESTING.md).

Press `/` on any non-text screen (or `Ctrl-K` anywhere) to open the searchable command palette. On the dashboard, choose “新建 Paper 服务器” or press `n`; the wizard loads Paper stable versions and local Java runtimes instead of asking users to type a version or Java path. Use Tab to move fields, `←/→` to change a version/Java/memory selection, Enter to advance, Esc to go back, and Space to explicitly accept the EULA on the review step. Choose “导入整合包” in the command palette to scan Downloads, Desktop, and the current directory for `.mrpack` files and review the static adapter report. Press Enter on an instance to open its detail view: `s` starts, `x` requests a graceful stop, `:` opens console input, Enter sends a command, and `r` refreshes status and output. Press `l` to open the world-backup manager: `v` re-verifies the selected snapshot, Enter previews restore impact, `c` asks for the exact `RESTORE` confirmation, `m` loads the next page, `r` reloads, and `b` creates a new stopped-world backup. Esc returns to instance detail. On the task panel, Enter opens task details; byte downloads show real transferred/total size, speed, and source. Press `x` twice to request cancellation; a resumable partial file is retained. `r` fetches events after the current sequence.

Run the UI renderer and state tests with `cargo test -p mc-tui`. Build the TUI and run `python3 scripts/tui_pty_smoke.py`, `python3 scripts/tui_task_detail_pty.py`, `python3 scripts/tui_guided_create_pty.py`, and `python3 scripts/tui_instance_detail_pty.py` to verify command-palette creation, option loading, byte progress, cancellation, task event resume, backup verification/restore confirmation, and instance lifecycle interactions through PTY fake daemons. Lifecycle API details and the process identity contract are in [docs/SUPERVISION.md](docs/SUPERVISION.md).
