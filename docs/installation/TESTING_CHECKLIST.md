# Installation acceptance checklist

This is a list of pending manual checks, not a passed test report.
Run each scenario with isolated program/config/state directories and record OS,
architecture, exact artifact, commands and observed results.

- [ ] macOS Apple Silicon: install the published artifact, run setup and verify launchd readiness.
- [ ] Linux x86_64/arm64: install the artifact and verify a real systemd user service.
- [ ] Verify missing dependencies, failed downloads and invalid hashes fail with useful errors.
- [ ] Verify custom `--prefix`/`--root` and a PATH without `~/.local/bin`.
- [ ] Verify `mc setup --no-service` with foreground daemon/TUI.
- [ ] Create a Paper instance and verify readiness with a real Minecraft client.
- [ ] Verify uninstall preserves instance data, backups and configuration.

Automated packaging and installer smoke checks are defined in
[CI](../../.github/workflows/ci.yml). Current validation and remaining limits live in
[the maintenance status](../PRODUCTIZATION_STATUS.md).
