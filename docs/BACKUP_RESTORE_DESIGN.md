# World backup and restore contract

> Version 0.2 · 2026-10-17 · stopped-server backup plus restore transaction core

This contract narrows the backup and recovery work in [the Base implementation plan](../BASE_IMPLEMENTATION_PLAN.md). It defines the first safe milestone and records the restore behavior that must be implemented before the feature is presented as a complete recovery flow.

## Module boundary

- `mc-core` defines the `StoppedInstanceLease` contract. `mc-supervisor` implements it and owns the per-instance stopped-state reservation: it verifies the recorded runner identity, rejects a live or unknown process, checks for an occupied server port, and holds both the in-process mutex and filesystem lifecycle lock for the lease's lifetime.
- `mc-backup` discovers world roots, stages copies, writes and checks manifests, and verifies existing backups. It does not start or stop a server and does not write instance configuration.
- The daemon/API layer owns durable task records and maps `BackupError` into the shared structured error model. `backup create <instance-id>`, `backup list <instance-id>`, `backup verify <instance-id> <backup-id>`, and the instance-detail `b`/`l` actions submit through this API rather than editing instance files.

## Version 1: stopped-server backup

### Consistency and exclusion

The server must be stopped before a backup begins. The caller obtains the supervisor's `StoppedInstanceGuard` asynchronously and moves it into `BackupService::create` through the `StoppedInstanceLease` contract. The lease remains owned by a Tokio blocking-pool task until the filesystem work completes, including when the async caller is cancelled after the task starts. Start and stop calls for that instance wait on the same lifecycle locks.

`consistency` is recorded as `server_stopped`. This means the supervisor verified that its managed runner was not live and the configured port was free at the start of the operation. It does not claim protection against another local process or user changing world files directly; source metadata is rechecked during copying and a detected change fails the backup.

### World discovery

1. Read `server.properties` and use the last `level-name` value; missing file or key falls back to `world`.
2. Require the primary world directory.
3. Include sibling `<level-name>_nether` and `<level-name>_the_end` directories when present.
4. Reject unsafe names, symlinks, special files, non-UTF-8 paths, and entries that cannot be represented by the manifest.

No other server directories or logs are included. The backup is a plain directory tree under `<instance>/backups/<backup-uuid>/`, with `manifest.json` alongside the copied world roots.

### Create transaction

1. Measure world file bytes and check free space for that amount plus a 1 MiB safety margin.
2. Create a private `.staging-<backup-uuid>` directory under `backups/` (directories mode 0700 and files mode 0600 on Unix).
3. Copy files in bounded 64 KiB chunks. Record each relative path, entry kind, byte length, and SHA-256 for files. Recheck source file and directory metadata during the copy.
4. Validate the sorted manifest and scan the staged tree again to compare its actual hashes and paths with the manifest.
5. Flush files and directories, atomically rename staging to the UUID destination, then flush `backups/`.

Any returned error before rename removes the staging directory. A process or machine crash can leave a `.staging-*` directory; it is never listed as a completed backup. If rename succeeds but the final directory flush fails, the service removes the destination and flushes `backups/` again. It returns `CommitRolledBack` when removal is confirmed, or `CommitUncertain` with the backup ID and path if cleanup cannot be confirmed. Completed backup IDs are immutable and a duplicate ID is rejected.

### Manifest and verification

Schema version 1 stores the backup UUID, instance UUID, UTC creation time, stopped consistency marker, sorted world roots, and sorted relative entry records. Manifest paths are UTF-8, relative, canonical, and confined to the listed worlds. Manifest reads are capped at 16 MiB.

`verify` validates schema and identities, rejects unsafe paths and filesystem entries, hashes the complete stored tree, and rejects missing, extra, or changed entries. The library `list` verifies every completed backup before returning it, newest first; corrupt entries fail the operation instead of silently disappearing. Leftover staging directories are skipped.

The API exposes bounded `ListWorldBackups { instance_id, limit, cursor }` and `VerifyWorldBackup { instance_id, backup_id }` requests as durable tasks. `limit` must be 1–50. The paged listing reads and validates candidate manifests to establish a deterministic newest-first order (`created_at`, then UUID), caps the catalog at 1,000 entries, and fully rehashes only the returned page. The cursor contains the last row's timestamp and UUID, so equal timestamps do not cause duplicates or omissions. Returned task results contain only small summaries: ID, timestamp, consistency, file count, total bytes, and an optional next cursor. Full tree hashing and directory scans run on Tokio's blocking pool under the task manager's per-instance lock and global worker/outstanding limits. `backup verify` fully hashes the selected backup and returns the same bounded summary shape.

The CLI provides `backup list <instance-id> [--limit N] [--cursor <RFC3339>~<UUID>]`, `backup verify <instance-id> <backup-id>`, `backup plan-restore <instance-id> <backup-id>`, and `backup restore <instance-id> <backup-id> --confirm-token <token>`. The plan task displays the full replacement impact and exact token; restore rejects malformed or stale tokens. In instance detail, `l` queues a three-item first page so the complete page fits in task detail. The TUI does not yet offer cursor navigation, verification selection, or direct restore selection/confirmation.

Create, list, and verify expose async methods. They run blocking filesystem and hash work on `tokio::task::spawn_blocking`; a worker join failure is returned as a typed `BackupError`.

## Restore transaction core

The mc-backup library now provides plan_restore, restore, has_pending_restore, and recover_pending. Restore methods consume the same StoppedInstanceLease used by backup creation and move it into spawn_blocking; cancellation of the async caller cannot release the lifecycle reservation while hashing or filesystem work continues.

plan_restore fully verifies the selected backup, hashes the currently recognized world roots, and returns a BackupRestorePlan containing the backup timestamp, current and target roots, replaced/added/removed roots, estimated staging space, available space, and a confirmation token. A backup whose primary root differs from the configured level-name is rejected so restore cannot silently leave server.properties pointing at a different world. A missing primary root can be recreated. The token binds the instance, verified manifest, current root names, and current tree hashes; restore recomputes the plan under the stopped lease and rejects stale confirmation.

After confirmation, restore checks free space, copies the verified roots into a private staging directory under server/ (same filesystem), scans the staged copy against the manifest, and rechecks the live tree and manifest before the first live rename. It then moves the current recognized roots to a per-operation rollback directory and installs the staged roots. The operation directory contains a private marker and monotonically numbered, fsynced journal records for prepared, swapping, swapped, rolling_back, and committed states. Each root rename flushes both affected parent directories.

Before commit, any returned error triggers an idempotent rollback. A rollback failure keeps the journal and available rollback data and returns a distinct error; it never reports success while the active root set is partial or uncertain. The new tree is scanned against the verified manifest before a durable committed journal is written. Committed rollback data is removed only after that point.

The daemon checks for a marked restore operation before starting an instance. An interrupted non-committed operation is rolled back while holding the stopped-instance lease; a committed operation is cleaned up. An invalid journal or unconfirmed rollback blocks the start and preserves evidence for inspection. Unmarked directories with restore-like names are ignored.

The versioned daemon protocol and CLI now expose durable plan and restore tasks. The plan result includes the affected roots, staging capacity, and exact confirmation token; restore requires that token and revalidates it under the stopped lease. The TUI can inspect and render plan results in task detail, but it does not yet provide direct backup selection and confirmation controls.

## Verification status

The implementation covers stopped-world discovery, space preflight, staged copy, SHA-256 manifest, paged listing, verification, and the local restore transaction core. Create is exposed as a durable daemon task through the versioned API, CLI, and instance-detail TUI; listing and verification are bounded daemon tasks available through CLI, with a first-page TUI action. Restore transaction tests cover impact planning, a missing primary root, extra dimension removal, backup tampering, stale confirmation, primary-world mismatch, injected replacement failure, and recovery after a simulated process interruption. Supervisor tests exercise the async lease-bearing restore methods. Restore planning and execution are exposed through protocol v7 durable tasks and CLI commands; the TUI still lacks direct restore selection and confirmation controls. A real Minecraft server backup/restore drill remains follow-up work.
