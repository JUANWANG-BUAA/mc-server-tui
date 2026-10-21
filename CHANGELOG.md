# Changelog

All notable user-facing changes are recorded here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Planned

- Vanilla provider and takeover of existing server directories.
- Broader `server.properties` editing (including raw mode), Java compatibility
  rules, and structured management results for the CLI.
- State schema migration snapshots and update-time task draining.
- Online/scheduled backups and post-restore in-game world verification.
- Signed/notarized releases, Homebrew tap, Linux packages, and a signed
  self-updater with health checks.
- Mirrors and retry policy for constrained networks.

### Fixed

- First-use actions now render in the default session as well as the classic
  dashboard. Fabric opens a command draft requiring explicit parameters/EULA.
- Operation failures display scrollable error details without losing backend
  evidence, recovery instructions, or retry policy. Background connection errors
  remain inline; no automatic retry or clipboard action is advertised.
- Restore workspace checks, attach error-template tests, and replace placeholder
  first-use checks with a real PTY flow included in CI.
- Correct installation/CLI instructions and consolidate duplicate optimization
  reports into [maintenance status](docs/PRODUCTIZATION_STATUS.md).

## [0.2.1] - 2026-10-20

Unsigned alpha pre-release; API 8–11, state schema 1 and runner protocol 1 unchanged.

### Added

- Complete English and Chinese user guides, plus native documentation pages
  with chapter navigation, code copy controls, mobile layout, and explicit
  hosting, data-safety, license, and no-warranty notices.

### Changed

- Instance-centered TUI redesign: selected-server summary and contextual next actions
  above the transcript, a single compact task summary, and separate management
  connection feedback. Phase-only tasks no longer imply a byte percentage.
- Runtime settings start with a summary; F3 expands settings, instance diagnostics
  or backup evidence. Ctrl-K exposes the same actions. Logs have more space by default.
- Backup details lead with local time, verification and restore impact; UUID/journal
  evidence is secondary. Pixel Steve is reserved for the empty welcome screen.

### Fixed

- Console drafts stay bound to their original instance and cannot silently follow
  a selection change. The selected server never inherits another detail page's status.
- Re-evaluate the suggested Java when entering runtime setup after choosing a game version.
- Runtime summaries retain instance/version context; backup loading copy matches the operation.

## [0.2.0] - 2026-10-20

Second public **unsigned alpha** pre-release.

### Added

- Workspace pages with stable IDs, keyboard/mouse switching, and preserved
  per-instance log filters and reading positions.
- `/connect`, `/monitor`, `/rg` log search (literal/regex), live log filtering,
  CPU/RSS history, and non-destructive alerts for crashes, ERROR logs, sustained
  CPU, and RSS above configured Xmx.
- Player management through typed console actions: `/players list`, `op`,
  `deop`, `ban`, `pardon`, `kick`, whitelist add/remove/toggle, plus LuckPerms
  group and permission helpers.
- Fabric server creation through `/fabric`, with stable loader selection,
  Java/port checks, and explicit EULA acceptance.
- `/mods` catalog search and add-on installation for Paper plugins and Fabric
  mods, including SHA-512 verification, required catalog dependencies, JAR
  metadata checks, client-only rejection, and stopped-instance enforcement.
- MRPACK/ZIP server-pack preview and stopped-server import, plus export of
  server add-on/config bundles with SHA-256 manifests and archive limits.
- API protocol 11 with compatibility negotiation: the daemon accepts protocols
  8–11, replies with the request's protocol, and hides newer task kinds/results from
  v8 clients. `ServerInfo` and the release manifest now report `state_schema`
  and `runner_protocol`. CI starts an isolated real daemon (no Paper/Java) to
  check the window with `scripts/check_protocol_window.py`.
- Runner control protocol v1 with additive fields; legacy frames without a
  protocol field are treated as v1.
- `mc update status` and `mc update rollback`, which atomically swap the
  versioned `current`/`previous` symlinks, restart the user service, and refuse
  a state-schema downgrade when the target binary cannot read the running
  daemon's layout.
- A limited `server.properties` editor for MOTD, max players, difficulty, and
  view distance, with diff review, revision conflict detection, preservation of
  unknown settings/comments, and atomic save. Saving does not restart the server.
- Controlled restart through `/restart` or `R` in the instance detail, plus
  `mc cli instance start/stop/restart`. A restart holds one coordination lock
  across graceful stop and restart, requires a verified running process, and
  aborts before starting if the old process did not stop cleanly.
- Actionable startup-failure diagnosis from the current console log for Java
  version incompatibility, occupied ports, unaccepted EULA, insufficient Java
  memory, and missing/corrupt `server.jar`.
- Static GitHub Pages landing site with real TUI screenshots, interactive
  theme switching, pixel Steve, architecture/install/roadmap sections, SEO
  metadata, sitemap, and a custom 404 page.

### Fixed

- `/players` read the wrong log offset after the console log exceeded 64 KiB.
- MRPACK client-overrides were imported into servers; `server-overrides` now
  takes precedence and same-path conflicts are resolved by the spec.
- Fabric API could be installed twice when the catalog project ID and JAR slug
  differed, which crashes Fabric.
- Fabric/Modrinth downloads ignored the configured User-Agent and used the
  system temporary directory before an instance existed.
- The workspace bar covered each page's first row, restore confirmation was
  clipped, and supervisor status text stayed English in the UI.
- Port selection and `server-port`/`--port` now agree on the assigned listener.

## [0.1.0] - 2026-10-19

First public **unsigned alpha** pre-release.

### Added

- `mc` launcher with TUI, daemon, CLI, version/status/doctor, setup, service, and
  uninstall commands.
- `config.toml` with default User-Agent, state/socket/cache paths, and
  environment overrides.
- launchd (macOS) and systemd (Linux) user-service management with
  `AbandonProcessGroup`/`KillMode=process` so resident runners survive daemon
  restarts.
- Paper creation, verified downloads, resumable cache, durable tasks, byte
  progress, cancellation, and a per-instance resident supervisor.
- Session-first TUI with transcript, composer, completion, command palette,
  keyboard editing, themes, pixel Steve, task/instance/backup detail screens,
  mouse support, and 80x24/40x12 render coverage.
- Stopped-world backup, paged listing, verification, restore preview, exact
  confirmation, transactional restore, and recovery journal.
- Read-only `.mrpack` analysis for MC version, loader, `env.server`, unsafe
  paths, hashes, and download hosts.
- Versioned tarball release packaging, SHA-256, release manifest, installer,
  Homebrew formula rendering, and tag-driven GitHub Actions draft releases.
- macOS arm64/x86_64 and Linux x86_64/arm64 release artifacts.

### Known limitations

- Unsigned and unnotarized artifacts; installer verifies SHA-256 only.
- Paper only; no Vanilla, Fabric, or actual `.mrpack` import.
- Stopped-world backups only; no online/scheduled backups.
- No server.properties editor, player management, or takeover of existing
  server directories.
- No auto-updater or N/N-1 daemon protocol window.
- Linux systemd is generated/tested in CI but has not had a full real-host
  install drill.

[Unreleased]: https://github.com/JUANWANG-BUAA/mc-server-tui/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/JUANWANG-BUAA/mc-server-tui/releases/tag/v0.2.1
[0.2.0]: https://github.com/JUANWANG-BUAA/mc-server-tui/releases/tag/v0.2.0
[0.1.0]: https://github.com/JUANWANG-BUAA/mc-server-tui/releases/tag/v0.1.0
