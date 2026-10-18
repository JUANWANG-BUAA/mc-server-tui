# Modpack adapter analysis

> Version 0.1 · 2026-10-18 · read-only Modrinth `.mrpack` inspection

## Scope

`mc-modpack` is an isolated, read-only analyzer for Modrinth format version 1 packs. It produces a bounded report used by the CLI and TUI before any filesystem mutation is allowed. It does not start the server, download files, or write into an instance directory.

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

## API and clients

Protocol v8 adds `AnalyzeModpack { path }` and `ModpackAnalysis(report)`. The daemon runs the analyzer on a blocking worker. The CLI exposes `modpack inspect <path>`. The TUI command palette exposes “导入整合包”, scans common local folders, and renders the adapter report.

## Next phase

Actual import must be a new durable task. It must consume an unchanged analysis result, download only hash-pinned files through the verified downloader, extract through a no-symlink/no-traversal staging path, install exactly one loader, preserve a rollback boundary, and require first-start verification. No part of this read-only report is itself proof that a modpack server can start.
