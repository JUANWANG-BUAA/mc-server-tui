# Changelog

All notable user-facing changes are recorded here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Planned

- Vanilla provider and takeover of existing server directories.
- `server.properties` editing, Java compatibility rules, and player management.
- Online/scheduled backups and a real Minecraft restore drill.
- Signed/notarized releases, Homebrew tap, and Linux packages.
- Update manifest compatibility ranges and a safe updater.

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
