# Changelog

All notable user-facing changes are recorded here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

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
- API protocol 9 with N/N-1 negotiation: the daemon accepts protocols 8–9,
  replies with the request's protocol, and hides v9-only task kinds/results from
  v8 clients. `ServerInfo` and the release manifest now report `state_schema`
  and `runner_protocol`. CI starts an isolated real daemon (no Paper/Java) to
  check the window with `scripts/check_protocol_window.py`.
- Runner control protocol v1 with additive fields; legacy frames without a
  protocol field are treated as v1.
- `mc update status` and `mc update rollback`, which atomically swap the
  versioned `current`/`previous` symlinks, restart the user service, and refuse
  a state-schema downgrade when the target binary cannot read the running
  daemon's layout.
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

### Planned

- Vanilla provider and takeover of existing server directories.
- `server.properties` editing, Java compatibility rules, and structured
  management results for the CLI.
- State schema migration snapshots and update-time task draining.
- Online/scheduled backups and a real Minecraft restore drill.
- Signed/notarized releases, Homebrew tap, and Linux packages.
- Signed self-updater with health checks, plus mirrors/retries for constrained
  networks.

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

[Unreleased]: https://github.com/JUANWANG-BUAA/mc-server-tui/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/JUANWANG-BUAA/mc-server-tui/releases/tag/v0.1.0
