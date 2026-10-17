# mc-server-tui

Rust workspace for managing local Minecraft Java server instances on macOS and Linux.

## Current implementation slice

The workspace has shared API and error contracts, local `doctor` checks, a Paper Fill v3 adapter, a foreground Unix-socket daemon, a verified downloader, persistent Paper instance creation/listing, and durable background tasks. Creation now returns a task ID; metadata resolution, download, verification, and installation proceed asynchronously and can be inspected through the CLI. Startup recovery registers valid committed directories, quarantines interrupted staging data, and marks unfinished tasks interrupted.

```sh
cargo run -p mc-daemon
# In another terminal:
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
```

Paper requires a non-generic User-Agent with a real project contact. Set one before starting the daemon:

```sh
export MC_SERVER_TUI_USER_AGENT='mc-server-tui/0.1.0 (https://your-real-project-url)'
```

Use a real project URL or contact email; do not leave the example value in place. The daemon socket is per-user and restricted to the current user. `MC_SERVER_TUI_SOCKET` can override its location; a custom socket must live in a directory with mode `0700`.

## Module boundaries

- `mc-core`: domain values, stable errors, provider trait, and transport-neutral request/response types.
- `provider-paper`: Fill v3 HTTP access and conversion to core types.
- `mc-download`: exact artifact fetching, redirect/DNS checks, safe range resume, SHA-256 validation, and content-addressed cache.
- `mc-system`: local Java discovery, port check, data directory and disk checks.
- `mc-instance`: versioned `instance.toml`/`lock.json`, atomic staging, SQLite index and startup recovery.
- `mc-task`: durable task/event storage, phase reporting, bounded workers, per-instance operation locks, and restart interruption recovery.
- `mc-daemon`: bounded local API transport and request dispatch; server process supervision is a later slice.
- `mc-cli`: command parsing and presentation; instance operations, task queries, and Paper metadata go through the same local API intended for the TUI.
- TUI, durable task progress, start/stop/logs, and backup/restore are subsequent slices.

Instance storage and crash reconciliation rules are recorded in [docs/INSTANCE_SCHEMA.md](docs/INSTANCE_SCHEMA.md).
