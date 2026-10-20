# Modpack analysis and server-pack import

> Version 0.2 · 2026-10-20 · Modrinth `.mrpack` analysis and stopped-server
> materialization

## Scope

`mc-modpack` is an isolated, read-only analyzer for Modrinth format version 1
packs. It produces a bounded report used by the CLI and TUI before any
filesystem mutation is allowed.

Actual import is implemented in the daemon's portable-pack workflow. It
requires a stopped-instance lifecycle lease, stages and validates every file,
then commits without overwriting an existing file. The analyzer itself never
starts the server, downloads files, or writes into an instance directory.

## Inputs and limits

- The file extension must be `.mrpack`.
- `modrinth.index.json` must exist, be valid UTF-8, and be at most 4 MiB.
- The archive may contain at most 10,000 entries.
- At most 1,000 file-plan rows are returned; count summaries still cover the full index.
- Relative paths must be UTF-8-safe, canonical, and must contain no absolute root, parent traversal, backslash ambiguity, or NUL.

## Evidence order

1. Package-level `formatVersion`, `game`, `dependencies`, and file environment declarations.
2. File-level `env.server`, HTTPS download URLs, and sha1/sha512 hashes.
3. Conservative directory rules for obvious client-only resources such as `shaderpacks/` and `resourcepacks/`.
4. Unknown Mod JARs enter the review bucket; the filename is never used as proof of server compatibility.

## Dispositions

| Disposition | Meaning |
| --- | --- |
| `install` | Declared server-required and has a supported hash plus HTTPS download |
| `review` | Optional/unknown server compatibility and therefore cannot be installed silently |
| `exclude` | Explicitly `server=unsupported` or a known client-only resource directory |

## Readiness

- `ready`: no blockers, review files, or warnings.
- `review`: no blockers, but one or more files or metadata fields need a human decision.
- `blocked`: unsafe path, duplicate path, missing Minecraft version, multiple loaders, invalid schema, or an install-required file without hash/download trust.

## Import transaction

The import path is deliberately narrower than “install any client pack”:

- the instance must be stopped; the stopped-instance lease is held for the
  complete import;
- the archive is capped at 10,000 entries, 512 MiB expanded size, and 256 MiB
  per file, with traversal, absolute, backslash, control-character, and symlink
  paths rejected;
- only `mods/*.jar`, `plugins/*.jar`, and `config/**` are eligible; Paper rejects
  `mods/`, Fabric rejects `plugins/`, and world, player identity, administrator
  lists, `server.jar`, and `server.properties` are never imported;
- `server-overrides/` replaces the same path in `overrides/`;
  `client-overrides/` is excluded, and same-priority or case-only duplicate
  paths reject the archive;
- a Modrinth index must match the instance's Minecraft and loader versions.
  Downloads must use HTTPS on `cdn.modrinth.com`, match the declared size, and
  pass SHA-512 verification before entering the staging directory;
- JAR metadata is checked against the target provider, and an exported
  `mc-pack.json` manifest is verified with SHA-256;
- the staged files are compared with the live tree before commit. Identical
  files are skipped; any different-content conflict rejects the whole import
  without leaving a partial result.

Cancellation is checked before commit. The import result is not proof that the
server or its add-ons will start successfully.

## API and clients

Protocol v8 adds `AnalyzeModpack { path }` and `ModpackAnalysis(report)`. The
daemon runs the analyzer on a blocking worker. The CLI exposes
`modpack inspect <path>`. Protocol v9 adds typed `ManageInstance` pack
import/export operations. The TUI command palette exposes “导入整合包”, scans
common local folders, renders the adapter report, and then runs
`/import <path> --apply` through the stopped-instance workflow.

## Next phase

The remaining import work is first-start verification with clear load-failure
evidence, an automatic rollback boundary after that verification, and loader
support beyond the current Paper/Fabric targets. Forge, NeoForge, and CurseForge
file-ID authorization remain unimplemented. Importing a pack is not equivalent
to proving that every downloaded add-on can run together on a live server.
