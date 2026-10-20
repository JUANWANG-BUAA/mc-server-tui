# FAQ

## Is this stable?

No. `v0.2.1` is an **unsigned alpha pre-release**. The core Paper/Fabric lifecycle,
backups, and TUI work, but the feature set is intentionally incomplete. See
[Implementation gaps](IMPLEMENTATION_GAPS.md).

## What do I need?

- macOS 13+ on Apple Silicon, or Linux (x86_64/arm64);
- the Java release required by the selected Minecraft version (Java 21 for
  1.21.x; newer releases may require newer Java);
- a terminal of at least 80×24; smaller layouts degrade or hide the mascot;
- internet access for the first Paper download.

## How do I install it?

Release installer (tag-specific while `v0.2.1` is a pre-release):

```sh
TAG=v0.2.1
curl -fsSL "https://github.com/JUANWANG-BUAA/mc-server-tui/releases/download/$TAG/install.sh" | sh
mc setup
mc
```

`mc setup` writes `config.toml`, checks Java, installs a launchd/systemd user
service, starts the daemon, and leaves the TUI to `mc`.

Source:

```sh
cargo build --workspace
./target/debug/mc setup
./target/debug/mc
```

## Does closing the terminal stop my server?

No. The TUI is a client. The resident runner owns the Java process and keeps
running when the TUI or management daemon restarts. Install the user service so
the daemon is supervised by launchd/systemd.

## Where is my data?

The platform application state directory for `mc-server-tui`, under
`org.mc-server-tui.mc-server-tui`. `mc doctor` prints the exact path. Instance
worlds live under `<state>/instances/<uuid>/server/`; do not edit the UUID
directory while the server is running.

## Can I use an existing server directory?

Not yet. “Take over existing directory” is a first-stable-version gap. Do not
point this alpha at a production world you cannot restore from a separate
backup.

## Does it support Vanilla, Fabric, or modpacks?

`v0.2.1` supports Paper and Fabric, player actions, and stopped-instance import
for `.mrpack` and server-pack ZIP files. Imports add compatible
mods/plugins/config only and refuse conflicts. Vanilla, Forge/NeoForge, and
CurseForge file-ID materialization are not implemented.

## Can I edit server.properties?

`/config` edits MOTD, max players, difficulty, and view distance. It shows a
diff and refuses to overwrite a file changed outside the editor. Other settings,
raw editing, and validation for every property are still follow-up work. A
running server keeps its current values until it is restarted; use `/restart`
or `R` after saving when the new values should be applied immediately.

## Are backups online?

No. Backups are stopped-world only. Stop the server before creating a backup;
restore requires the instance to be stopped and uses a preview plus explicit
confirmation.

## Why did the server fail to start?

The instance detail recognizes common startup failures from the current console
log: incompatible Java, an occupied port, an unaccepted EULA, insufficient Java
memory, or a missing/corrupt `server.jar`. Unknown failures retain the log-based
generic message. Run `/logs` or `/open` to inspect the evidence.

## How do I update?

There is no automatic updater yet. The development branch provides manual
`mc update status` and `mc update rollback` against the versioned
`current`/`previous` directories. Install a newer release over the existing
versioned install, or use the package manager once packages exist. Signed
releases, task draining, and health-checked automatic update/rollback remain
follow-up work.

## Why is the release unsigned?

Signing/notarization requires an Apple Developer ID and release secrets. The
installer verifies the manifest's SHA-256 and size, but this is not a substitute
for a signature. Do not describe the alpha artifacts as signed.

## Does it work on Windows?

No. The current implementation depends on Unix sockets and Unix process
semantics. macOS and Linux are the supported platforms.

## Where should I report a bug or vulnerability?

- Bugs/features: GitHub issue templates.
- Vulnerabilities: private GitHub Security Advisory, see [SECURITY](../SECURITY.md).
- General setup questions: [FAQ](FAQ.md) and `mc doctor --json`.

## Why does my console draft refuse to send after switching servers?

v0.2.1 binds a draft to the instance selected when typing began. Return to that
instance, or press Esc to clear and retype for the new target. The composer shows
whether input is an application command or a Minecraft console command.

## Where did detailed runtime and backup information go?

Press F3 in runtime setup, instance detail or backup manager. Ctrl-K exposes the
same expand/collapse actions. The default views prioritize the selected server,
logs and restore impact. This changes presentation, not restore confirmation.

## What was actually tested for v0.2.1?

See the [acceptance record](acceptance/0.2.1.md). It separates deterministic visual
fixtures from the isolated real Paper backup/restore drill and lists remaining
in-game and Linux service checks.
