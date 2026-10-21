# Install on macOS

The v0.2.2 unsigned alpha publishes an Apple Silicon artifact for macOS 13+.
Intel macOS has no published artifact in this release. Source builds are a development path,
not a separately validated release target. Keep independent backups of important worlds.

## Install

Use Java compatible with the selected Minecraft version: Java 21 for 1.21.x,
Java 25+ for 26.x. The Paper wizard discovers installed runtimes.

```sh
TAG=v0.2.2
curl -fsSL "https://github.com/JUANWANG-BUAA/mc-server-tui/releases/download/$TAG/install.sh" | sh
~/.local/bin/mc setup
~/.local/bin/mc
```

The installer requires `curl`, `tar`, `python3`, and a SHA-256 tool.
It verifies the archive hash and size against the release manifest; it does not verify a code signature.
Add `~/.local/bin` to your shell PATH to use `mc` directly.

## Configuration and service

`mc setup --dry-run` shows the exact config and launchd service plan without applying it.
`mc doctor --json` reports the resolved data directory and detected Java runtimes.
The default configuration is under the platform application support directory;
custom configuration and environment overrides can change the paths.

```sh
mc service status
mc status --json
mc doctor --json
```

For foreground operation, run `mc setup --no-service`, then `mc daemon run` in one
terminal and `mc tui` in another. Stopping the management service leaves resident servers running.

For source builds, run `cargo build --workspace --release --locked`, then
`./target/release/mc setup` and `./target/release/mc`. Keep that build directory
available while its launchd service points to it.

See the [user guide](../USER_GUIDE.md), [quick start (中文)](../QUICK_START.md),
and [FAQ](../FAQ.md). Signing, notarization and Homebrew tap publication remain future work.
