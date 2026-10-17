# World backup and restore contract

> Version 0.1 · 2026-10-17 · first implementation supports stopped-server backups

This contract narrows the backup and recovery work in [the Base implementation plan](../BASE_IMPLEMENTATION_PLAN.md). It defines the first safe milestone and records the restore behavior that must be implemented before the feature is presented as a complete recovery flow.

## Module boundary

- `mc-core` defines the `StoppedInstanceLease` contract. `mc-supervisor` implements it and owns the per-instance stopped-state reservation: it verifies the recorded runner identity, rejects a live or unknown process, checks for an occupied server port, and holds both the in-process mutex and filesystem lifecycle lock for the lease's lifetime.
- `mc-backup` discovers world roots, stages copies, writes and checks manifests, and verifies existing backups. It does not start or stop a server and does not write instance configuration.
- The daemon/API layer owns durable task records and maps `BackupError` into the shared structured error model. `backup create <instance-id>` and the instance-detail `b` action submit through this API rather than editing instance files.

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

`verify` validates schema and identities, rejects unsafe paths and filesystem entries, hashes the complete stored tree, and rejects missing, extra, or changed entries. `list` verifies every completed backup before returning it, newest first; corrupt entries fail the operation instead of silently disappearing. Leftover staging directories are skipped.

Create, list, and verify expose async methods. They run blocking filesystem and hash work on `tokio::task::spawn_blocking`; a worker join failure is returned as a typed `BackupError`.

## Restore transaction: required before claiming recovery support

Restore is not implemented in version 1. Its implementation must satisfy these steps before it is exposed in CLI/TUI:

1. Obtain `StoppedInstanceGuard` and retain it until success or rollback completes.
2. Verify the selected backup completely before changing live files; show the instance, backup time, world roots, and replacement impact to the user.
3. Copy verified roots into a private restore staging directory on the same filesystem as `server/`.
4. Rename current world roots into a uniquely named rollback area, then rename staged roots into place. Keep a durable operation journal so startup recovery can distinguish prepared, swapped, committed, and rolling-back states.
5. If any replacement step fails, restore every prior root from the rollback area. Keep the rollback data until the new tree has passed a second integrity scan and the transaction is committed.
6. Flush affected directories and preserve recovery evidence when rollback itself fails; never report success while the active world set is partial or uncertain.

Online backups, save-command acknowledgements, timeouts, and automatic retention are deferred. They require a separate consistency protocol and must not be inferred from the stopped-only marker.

## Verification status

The first implementation covers stopped-world discovery, space preflight, staged copy, SHA-256 manifest, list, and verify. Create is exposed as a durable daemon task through the versioned API, the CLI, and the instance-detail TUI; duplicate requests reuse the active backup task for that instance. Tests exercise the lifecycle reservation, primary and dimension roots, safe-name and symlink rejection, duplicate IDs, active-task reuse, listing, content tampering, task reporting, and the TUI backup queue interaction. List and verify remain library operations without CLI/TUI commands. Restore, recovery journaling, and a real server backup/restore drill remain follow-up work; this is not yet a complete recovery flow.
