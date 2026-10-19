# Contributing

Thanks for helping make local Minecraft server management less painful.

## Before you start

- Read [docs/IMPLEMENTATION_GAPS.md](docs/IMPLEMENTATION_GAPS.md) for the current
  slice and known missing behavior.
- For user-visible TUI work, read [docs/INTERACTION_DESIGN.md](docs/INTERACTION_DESIGN.md)
  and update the screenshots/tests described in [docs/TUI_TESTING.md](docs/TUI_TESTING.md).
- For lifecycle changes, read [docs/SUPERVISION.md](docs/SUPERVISION.md); process
  identity and world safety are not optional details.
- For distribution changes, read [docs/DISTRIBUTION.md](docs/DISTRIBUTION.md) and
  [docs/RELEASING.md](docs/RELEASING.md).

## Toolchain

The repository pins Rust **1.89.0** in `rust-toolchain.toml`. Install the
components with rustup if needed:

```sh
rustup toolchain install 1.89.0 --component rustfmt,clippy
```

## Local checks

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

For TUI behavior, build the workspace and run the relevant PTY scripts:

```sh
cargo build --workspace
python3 scripts/tui_pty_smoke.py
python3 scripts/tui_guided_create_pty.py
python3 scripts/tui_task_detail_pty.py
python3 scripts/tui_instance_detail_pty.py
```

These scripts use fake daemons and PTYs; they do not require a real Minecraft
server. Real-server behavior must still be validated manually before claiming a
release-ready lifecycle change.

## Code guidelines

- Keep blocking filesystem/process/SQLite work out of async request handlers;
  use `spawn_blocking` and bounded admission.
- Preserve the instance UUID, `instance.toml`/`lock.json` ownership split, and
  atomic staging rules in [docs/INSTANCE_SCHEMA.md](docs/INSTANCE_SCHEMA.md).
- Never silently switch download sources, accept an EULA, kill a process whose
  identity is unverified, or report a failed backup as verified.
- Prefer structured errors with a next action/evidence over generic failures.
- Do not add a dependency for a small helper without checking the existing
  workspace crates first.

## Pull requests

- Keep changes focused; separate TUI, daemon, and release-tooling work when
  possible.
- Add tests for new invariants and platform behavior.
- Include screenshots for visible TUI changes.
- Update the relevant contract document and README when user-facing behavior
  changes.
- Do not commit `dist/`, `target/`, instance data, logs, or credentials.

## License

By contributing, you agree that your contribution is dual-licensed under
MIT OR Apache-2.0, at the project's option. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).
