# Quick Start Guide

> **For mc-server-tui v0.2.1** — A terminal-first Minecraft Java server manager

This guide gets you from zero to a running Minecraft server in under 5 minutes.

## ⚠️ Before You Start

**This is an alpha release** (v0.2.1, unsigned). Use it for testing and learning, not for production worlds without independent backups.

- ✅ **Supported**: macOS 13+ (Apple Silicon), Linux (x86_64, ARM64)
- ❌ **Not supported**: Windows, Intel macOS (this release)
- 📦 **Required**: Java 21+ for Minecraft 1.21.x, 25+ for 26.x
- 🖥️ **Terminal**: Minimum 80×24 (wider is better)

[Read the full safety disclaimer →](docs/USER_GUIDE.md#1-read-this-before-hosting-a-world)

---

## 🚀 One-Command Installation

```bash
# v0.2.1 is a pre-release; use the tag-specific URL
TAG=v0.2.1
curl -fsSL "https://github.com/JUANWANG-BUAA/mc-server-tui/releases/download/$TAG/install.sh" | sh
```

**What this does:**
1. Downloads the release manifest
2. Verifies SHA-256 checksum
3. Installs to `~/.local/share/mc-server-tui/versions/`
4. Creates launcher at `~/.local/bin/mc`

**Then run:**

```bash
mc setup    # Creates config, installs service, checks Java
mc          # Opens the TUI
```

---

## 📋 Common Installation Issues

### "Java not found"

Install Java 21 or newer:

```bash
# macOS (Homebrew)
brew install openjdk@21

# Ubuntu/Debian
sudo apt update && sudo apt install openjdk-21-jdk

# Fedora/RHEL
sudo dnf install java-21-openjdk

# Arch Linux
sudo pacman -S jdk21-openjdk
```

Verify: `java -version` should show 21 or higher.

### "mc: command not found"

Add `~/.local/bin` to your PATH:

```bash
# For bash (add to ~/.bashrc)
export PATH="$HOME/.local/bin:$PATH"

# For zsh (add to ~/.zshrc)
export PATH="$HOME/.local/bin:$PATH"

# For fish (add to ~/.config/fish/config.fish)
set -gx PATH $HOME/.local/bin $PATH

# Then reload your shell
source ~/.bashrc  # or ~/.zshrc
```

Or use the full path: `~/.local/bin/mc setup`

### "Daemon did not become ready"

Check the service status:

```bash
mc service status

# On macOS, check the log
tail ~/Library/Logs/mc-server-tui/daemon.log

# On Linux with systemd
journalctl --user -u mc-server-tui -n 50
```

If the service failed to start, try manual mode:

```bash
# Terminal 1: Run daemon in foreground
mc daemon run

# Terminal 2: Open TUI
mc tui
```

### "Insufficient disk space"

The installer needs at least 500MB:
- Binary: ~20MB
- First Paper download: ~50MB
- Instance data: 100MB+ per world

Free up space or use a different install location:

```bash
./install.sh --root /path/to/larger/volume
```

---

## ✨ First Server in 3 Steps

### 1. Launch the TUI

```bash
mc
```

You'll see a session shell with a command prompt.

### 2. Create a Paper Server

Type (or press `/` for autocomplete):

```
/create
```

The guided wizard will:
- List available Minecraft versions (26.x, 1.21.x)
- Show detected Java runtimes
- Let you set memory (1–8 GB)
- Pick an available port (default: 25565)
- Ask you to accept the Minecraft EULA

### 3. Start Your Server

```
/start
```

Wait for "Server is ready" (usually 10–30 seconds), then:

```
/connect
```

This shows:
- **Local**: Connect from this machine (localhost:25565)
- **LAN**: Connect from your network (192.168.x.x:25565)
- **Internet**: Requires port forwarding (not configured by default)

Join with your Minecraft client at the displayed address!

---

## 🎮 Daily Commands

Once your server is created, these are your most-used commands:

| Command | Purpose |
|---------|---------|
| `/start` | Start the server |
| `/stop` | Gracefully stop the server |
| `/restart` | Stop and restart |
| `/logs` | View recent console output |
| `/backup` | Create a world backup (requires stopped server) |
| `/console say Hello!` | Send a command to the server console |
| `Hello from console` | Type directly to send to console (no `/`) |
| `/open` | Open instance detail (lifecycle, logs, backups) |
| `/config` | Edit server.properties (MOTD, max players, difficulty, view distance) |
| `/help` or `F1` | Show all commands and keybindings |

**Useful keys:**
- `Tab` — Autocomplete commands
- `↑/↓` — Navigate history
- `Ctrl-K` — Command palette
- `F2` — Switch themes (7 available)
- `PgUp/PgDn` — Scroll transcript

---

## 🛠️ Management Outside the TUI

```bash
mc status              # Check if daemon is running
mc doctor              # System diagnostics (Java, ports, paths)
mc version             # Show version and protocol info

mc cli instance list   # List all instances
mc cli instance start <id>
mc cli instance stop <id> --timeout-seconds 20

mc service status      # Check background service
mc service restart     # Restart the daemon
mc uninstall           # Remove program (keeps instance data)
```

---

## 🌍 Connecting to Your Server

### From the Same Machine
```
localhost:25565
```

### From Your Local Network (LAN)
1. In the TUI: `/connect` shows your local IP
2. Give players: `192.168.x.x:25565` (your actual IP)

### From the Internet (Port Forwarding)
1. Forward port 25565 (TCP) on your router to your machine's local IP
2. Find your public IP: https://ifconfig.me/
3. Give players: `your.public.ip:25565`

**Security note**: Internet-facing servers need proper configuration:
- Enable whitelist: `/console whitelist on`
- Set strong OP passwords
- Keep Java and server software updated
- Consider firewall rules

[More on player management →](docs/USER_GUIDE.md#7-player-management-oplists-and-permissions)

---

## 📚 Learning More

### Essential Docs
- [User Guide](docs/USER_GUIDE.md) — Complete tutorial from install to recovery
- [中文使用手册](docs/USER_GUIDE.zh-CN.md) — 完整中文指南
- [FAQ](docs/FAQ.md) — Common questions and troubleshooting
- [Website](https://juanwang-buaa.github.io/mc-server-tui/) — Online documentation with navigation

### Key Concepts
- **Resident runner**: Your Java process keeps running even if the TUI or daemon restarts
- **Durable tasks**: Downloads and operations continue in the background
- **Stopped-world backups**: Backups require the server to be stopped (online backups are roadmap)
- **Instance pages**: Use `/open` to get detailed views; close with `/close` (server keeps running)

### Platform-Specific Guides
- [macOS Installation](docs/installation/macos.md)
- [Linux Installation](docs/installation/linux.md)

---

## ⚡ Quick Troubleshooting

| Issue | Solution |
|-------|----------|
| Server won't start | Run `/logs` to see error; common: wrong Java version, port in use, insufficient memory |
| "EULA not accepted" | Delete the instance and recreate, explicitly accepting the EULA in the wizard |
| Can't connect from LAN | Check firewall allows port 25565; use `/connect` to verify the correct IP |
| TUI closed but server still running | **This is normal!** The resident runner keeps Java alive. Use `mc` to reconnect |
| Out of memory | Stop server, edit memory in instance settings, restart |
| Lost my world | Check `mc doctor` for data path; backups are in `<state>/instances/<uuid>/backups/` |

---

## 🚪 Clean Uninstall

```bash
mc uninstall
```

**What this does:**
- Stops the daemon service
- Removes the service unit
- Deletes program files from `~/.local/share/mc-server-tui`
- **Keeps** your instance data, worlds, and configuration

**To remove everything** (including worlds):
```bash
mc doctor   # Note the state directory path
mc uninstall
rm -rf <state-directory>  # Use the path from doctor
```

---

## 🆘 Getting Help

- **Installation issues**: Check [installation guides](docs/installation/)
- **Server won't start**: Run `mc doctor --json` and check logs
- **Bugs**: [GitHub Issues](https://github.com/JUANWANG-BUAA/mc-server-tui/issues)
- **Security**: [Private disclosure](SECURITY.md)

---

## ⚠️ Alpha Limitations

Current **v0.2.1** is an unsigned alpha. Known limitations:

- ❌ No code signature or macOS notarization
- ❌ Can't take over existing server directories
- ❌ Backups require stopped servers (no online backup)
- ❌ Only 4 server.properties fields editable in TUI
- ❌ No Forge/NeoForge support
- ⚠️ Fabric and MRPACK imports are basic (no CurseForge file IDs)

[Full implementation status →](docs/IMPLEMENTATION_GAPS.md)

---

**Ready to dive deeper?** Read the [complete User Guide](docs/USER_GUIDE.md) or visit the [website](https://juanwang-buaa.github.io/mc-server-tui/).

**Enjoying mc-server-tui?** Star the repo and share your feedback!
