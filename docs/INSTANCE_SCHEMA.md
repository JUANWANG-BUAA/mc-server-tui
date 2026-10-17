# Instance and task data contract

> Version 0.3 · 2026-10-17 · implementation contract and current storage status

This contract records the storage boundary from `BASE_IMPLEMENTATION_PLAN.md` and `BASE_READINESS_REVIEW.md`.

## Paths

- Global state: the operating system's per-user application state directory for `mc-server-tui`.
- Instance directory: `<state>/instances/<instance-uuid>/`.
- User configuration: `instance.toml`.
- Exact installed artifact: `lock.json`.
- Server working directory: `server/`.
- Logs, backups, and recognized temporary work: `logs/`, `backups/`, and `tmp/`.
- Captured Java console output: `logs/console.log` (incremental byte offsets; the runner rotates at 32 MiB and keeps one previous segment as `console.log.1`).
- Per-instance runner identity, Java PID, and requested-stop flag: `server/supervisor.json`; lifecycle exclusion lock: `server/.lifecycle.lock`.
- Runner control socket: `<system-temp>/mcst-<uid>/<instance-uuid>.sock`, in a user-owned mode-0700 directory; the socket is mode 0600 and is removed when the runner exits.
- Global instance index, task records, and task events: `<state>/index.sqlite3`.
- Shared verified artifact cache: `<cache>/mc-server-tui-artifacts/sha256/<first-two-hex>/<sha256>`.

## Identity and file ownership

- Instance identity is UUID v4 and never changes. The display name is mutable and is not a path key.
- `instance.toml` is the authority for user choices: schema version, name, server provider, selected game version/build, Java binding, memory, port, EULA acceptance, and timestamps.
- `lock.json` is the authority for installed bytes: provider, project, exact version/build/channel, metadata origin, artifact name/size, upstream checksum, local SHA-256, and resolution time.
- SQLite is the global index and operation history, not a second editable copy of instance configuration.
- A task has a UUID, instance UUID (if applicable), operation, phase, progress, start/update/finish timestamps, structured result/error, and cancellation state.

## Commit and recovery rules

1. Resolve and validate a complete immutable artifact plan before downloading.
2. Download into a recognized `.part` file outside the instance directory. Verify expected size and upstream hash before creating a verified cache entry.
3. Stage server files and both config/lock records in a sibling temporary instance directory. Flush the files before atomic rename into the final UUID directory.
4. Register the instance in SQLite only after the directory commit. On startup, reconcile a committed directory missing from the index and quarantine incomplete temporary directories; never report them as installed.
5. Updating `instance.toml` or `lock.json` uses write-to-temp, flush, then atomic replacement. A schema migration keeps the previous file until the new version validates.
6. SQLite migrations are versioned and transactional. They never rewrite world files.

The persistence module must add migration tests and crash-recovery tests before it is considered complete.

## Implementation status

The `mc-instance` module implements schema version 1 for `instance.toml`, `lock.json`, and the rebuildable SQLite `instances` index. The `mc-task` module stores durable task snapshots and ordered events in the same database, with its own transactional `mc_task_schema` version 1 migration. `mc-supervisor` stores runtime process identity separately from user config and artifact lock data. Index snapshots are rebuilt from committed instance directories at daemon startup; interrupted `.staging-*` directories are moved to `instances/quarantine/`. Queued/running tasks become `interrupted` on daemon startup and are not replayed automatically. Task progress currently tracks phases, not transferred bytes. In-place config migration, cancellation, and safe task resume remain unimplemented.
