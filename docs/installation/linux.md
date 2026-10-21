# Install on Linux

The v0.2.2 unsigned alpha publishes GNU/Linux x86_64 and arm64 artifacts.
Real-host distribution and systemd acceptance remain incomplete; see the
[release acceptance limits](../acceptance/0.2.1.md).

## Install

Install Java compatible with the selected Minecraft version: Java 21 for 1.21.x,
Java 25+ for 26.x. Also provide `curl`, `tar`, `python3`, and a SHA-256 tool.

```sh
TAG=v0.2.2
curl -fsSL "https://github.com/JUANWANG-BUAA/mc-server-tui/releases/download/$TAG/install.sh" | sh
~/.local/bin/mc setup
~/.local/bin/mc
```

The installer verifies the archive hash and size against the release manifest.
Add `~/.local/bin` to your PATH for direct `mc` commands.
Keep independent backups of important worlds.

## User service and paths

`mc setup --dry-run` shows the resolved configuration and service plan.
`mc doctor --json` reports the actual data directory and Java runtimes.
Use these outputs instead of hard-coded XDG paths; configuration and environment
can override the defaults.

```sh
mc service status
mc status --json
journalctl --user -u mc-server-tui -n 100
```

A working systemd user session is required for automatic service installation.
Without one, use `mc setup --no-service`, run `mc daemon run` in one terminal,
and `mc tui` in another. Stopping the management service leaves resident Java servers running.

For source builds, run `cargo build --workspace --release --locked`, then
`./target/release/mc setup` and `./target/release/mc`.

See the [user guide](../USER_GUIDE.md), [quick start (中文)](../QUICK_START.md),
and [FAQ](../FAQ.md). No WSL or Raspberry Pi acceptance/performance claim is made here;
deb/rpm packages remain future work.
