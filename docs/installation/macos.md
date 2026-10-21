# macOS Installation Guide

> **For mc-server-tui v0.2.1** — Detailed macOS installation and troubleshooting

## System Requirements

### Supported Hardware
- ✅ **Apple Silicon** (M1, M2, M3, M4): Full support, native binary
- ⚠️ **Intel Mac**: Not published in v0.2.1 (build from source or wait for next release)
- ⚠️ **Rosetta 2**: Not recommended (performance penalty for Minecraft server)

### Software Requirements
- **macOS**: 13.0 (Ventura) or later
- **Java**: 21+ for Minecraft 1.21.x, 25+ for 26.x
- **Terminal**: 80×24 minimum (Terminal.app, iTerm2, Kitty, Alacritty all work)
- **Tools**: curl, tar, python3 (included with macOS)

## Pre-Installation

### 1. Check Your Mac Architecture

```bash
uname -m
```

- `arm64` → Apple Silicon ✅ (M-series chip)
- `x86_64` → Intel Mac ⚠️ (not published in v0.2.1)

### 2. Install Java

**Recommended: Use Homebrew**

```bash
# Install Homebrew if you don't have it
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install Java 21
brew install openjdk@21

# Link Java so it's on PATH
sudo ln -sfn $(brew --prefix)/opt/openjdk@21/libexec/openjdk.jdk \
  /Library/Java/JavaVirtualMachines/openjdk-21.jdk
```

**Alternative: Download from Adoptium**

Visit https://adoptium.net/ and download Temurin 21 for macOS.

**Verify Installation**

```bash
java -version
```

Should show version 21.x.x or higher.

### 3. Check Available Disk Space

```bash
df -h ~
```

You need at least **500MB** free:
- mc-server-tui binary: ~20MB
- First Paper download: ~50MB
- Per-server data: 100MB–5GB (depends on world size)

## Installation

### One-Command Install

```bash
TAG=v0.2.1
curl -fsSL "https://github.com/JUANWANG-BUAA/mc-server-tui/releases/download/$TAG/install.sh" | sh
```

### What Happens

1. **Downloads** release manifest from GitHub
2. **Verifies** SHA-256 checksum (not a code signature)
3. **Installs** to `~/.local/share/mc-server-tui/versions/v0.2.1/`
4. **Creates** symlink at `~/.local/bin/mc`

### Post-Install: Add to PATH

If `~/.local/bin` is not in your PATH, add it:

```bash
# For zsh (default on macOS)
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc

# For bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bash_profile
source ~/.bash_profile
```

Verify:

```bash
which mc
# Should show: /Users/yourname/.local/bin/mc
```

## Setup

### Run Initial Setup

```bash
mc setup
```

This will:
1. Create `~/Library/Application Support/org.mc-server-tui.mc-server-tui/config.toml`
2. Check for Java (shows warning if < Java 21)
3. Install a launchd LaunchAgent (user service)
4. Start the daemon automatically
5. Wait for daemon to become ready

**Expected output:**

```
config: /Users/yourname/Library/Application Support/org.mc-server-tui.mc-server-tui/config.toml
state: /Users/yourname/Library/Application Support/org.mc-server-tui.mc-server-tui
socket: /Users/yourname/Library/Application Support/org.mc-server-tui.mc-server-tui/run/daemon.sock
user-agent: mc-server-tui/0.2.1 ...
java: /usr/bin/java (Java 21)
service: launchd installed at ~/Library/LaunchAgents/org.mc-server-tui.mc-server-tui.plist
daemon: ready
next: run `mc` to open the TUI
```

### Understanding the launchd Service

The setup creates a **user LaunchAgent** (not a system daemon):
- Runs as your user (no sudo required)
- Starts automatically when you log in
- Manages the background daemon
- Logs to `~/Library/Logs/mc-server-tui/daemon.log`

**Service file location:**
```
~/Library/LaunchAgents/org.mc-server-tui.mc-server-tui.plist
```

### Manual Mode (No Background Service)

If you prefer to run the daemon manually:

```bash
mc setup --no-service
```

Then in two terminals:

```bash
# Terminal 1: Run daemon in foreground
mc daemon run

# Terminal 2: Open TUI
mc tui
```

## Verification

### Check Installation

```bash
mc version
```

Should show:
```
mc-server-tui 0.2.1
target: aarch64-apple-darwin
profile: release
API protocol: 8–11
state schema: 2
runner protocol: 1
service manager: launchd
```

### Check Daemon Status

```bash
mc status
```

Should show:
```
daemon: running
socket: /Users/yourname/Library/Application Support/.../run/daemon.sock
version: 0.2.1 (API 8–11, state 2, runner 1)
```

### Run System Diagnostics

```bash
mc doctor
```

Checks:
- Data directory paths
- Java installations (all found on PATH)
- Socket status
- Service status

## File Locations on macOS

| Purpose | Location |
|---------|----------|
| Program files | `~/.local/share/mc-server-tui/versions/v0.2.1/` |
| Launcher symlink | `~/.local/bin/mc` |
| Configuration | `~/Library/Application Support/org.mc-server-tui.mc-server-tui/config.toml` |
| Instance data | `~/Library/Application Support/org.mc-server-tui.mc-server-tui/instances/` |
| Worlds | `~/Library/Application Support/.../instances/<uuid>/server/` |
| Backups | `~/Library/Application Support/.../instances/<uuid>/backups/` |
| Task database | `~/Library/Application Support/.../tasks.db` |
| Download cache | `~/Library/Caches/org.mc-server-tui.mc-server-tui/` |
| Socket | `~/Library/Application Support/.../run/daemon.sock` |
| Daemon log | `~/Library/Logs/mc-server-tui/daemon.log` |
| Service file | `~/Library/LaunchAgents/org.mc-server-tui.mc-server-tui.plist` |

## Troubleshooting

### "Daemon did not become ready"

**Check the service status:**

```bash
launchctl list | grep mc-server-tui
```

Should show something like:
```
12345  0  org.mc-server-tui.mc-server-tui
```

The middle number (0) is the exit code. If it's not 0, the daemon crashed.

**Check the log:**

```bash
tail -50 ~/Library/Logs/mc-server-tui/daemon.log
```

Common issues:
- Socket path too long (unlikely with default paths)
- Permissions issue (try `mc setup` again)
- Port already in use (daemon starts successfully but you see this in logs)

**Restart the service:**

```bash
mc service restart
```

**If still failing, try manual mode:**

```bash
mc service stop
mc daemon run  # Watch for errors
```

### "Java not found" during setup

Even though Java is installed, it's not on PATH.

**Find your Java:**

```bash
/usr/libexec/java_home -V
```

**Add to PATH:**

```bash
# Add to ~/.zshrc or ~/.bash_profile
export PATH="/Library/Java/JavaVirtualMachines/openjdk-21.jdk/Contents/Home/bin:$PATH"

# Reload shell
source ~/.zshrc
```

### "mc: command not found"

The launcher is not on your PATH.

**Quick fix:**

```bash
export PATH="$HOME/.local/bin:$PATH"
```

**Permanent fix:** Add the export to `~/.zshrc` or `~/.bash_profile`

### "Operation not permitted" errors

macOS may ask for permissions when:
1. Creating files in `~/Library/Application Support/`
2. Installing the LaunchAgent
3. Binding to the socket

Grant Terminal (or iTerm2, etc.) **Full Disk Access** if prompted:
1. System Settings → Privacy & Security → Full Disk Access
2. Add your terminal app

Usually not needed with default paths, but some strict security settings require it.

### Performance Issues on Intel Mac

If you installed on Intel Mac via Rosetta 2 or compiled from source for x86_64:
- Minecraft servers need significant CPU (especially world generation)
- Rosetta 2 adds 20–40% overhead
- Consider waiting for native x86_64 builds or use a dedicated Linux server

### Port 25565 already in use

Another Minecraft server or service is using the default port.

**Find what's using it:**

```bash
lsof -i :25565
```

**Options:**
1. Stop the other server
2. Use a different port when creating your server in the TUI wizard
3. Change the port in `server.properties` and restart

## Service Management

### Start/Stop Service

```bash
mc service start    # Start daemon
mc service stop     # Stop daemon (keeps Java running)
mc service restart  # Restart daemon
mc service status   # Check status
```

### Disable Auto-Start at Login

The LaunchAgent starts automatically when you log in. To disable:

```bash
launchctl unload ~/Library/LaunchAgents/org.mc-server-tui.mc-server-tui.plist
```

To re-enable:

```bash
launchctl load ~/Library/LaunchAgents/org.mc-server-tui.mc-server-tui.plist
```

### Manual Service Control

```bash
# Stop
launchctl unload ~/Library/LaunchAgents/org.mc-server-tui.mc-server-tui.plist

# Start
launchctl load ~/Library/LaunchAgents/org.mc-server-tui.mc-server-tui.plist

# Check if running
launchctl list | grep mc-server-tui
```

## Firewall Configuration

### Allow Incoming Connections

When you first start a server, macOS may ask:

> "Do you want the application 'java' to accept incoming network connections?"

Click **Allow**. This is normal—Minecraft servers need to accept connections on port 25565.

### Manual Firewall Rule

If you denied it accidentally:

1. System Settings → Network → Firewall → Options
2. Click the `+` button
3. Add Java (usually `/Library/Java/JavaVirtualMachines/.../bin/java`)
4. Set to "Allow incoming connections"

### LAN Access

For friends on your local network to connect:
1. Find your IP: `ifconfig | grep "inet " | grep -v 127.0.0.1`
2. Give them: `<your-ip>:25565`
3. No firewall changes needed for LAN (by default)

### Port Forwarding for Internet Access

1. Log into your router's admin panel
2. Forward port 25565 (TCP) to your Mac's local IP
3. Find your public IP: `curl ifconfig.me`
4. Give players: `<public-ip>:25565`

**Security**: Enable whitelist mode in Minecraft before exposing to internet.

## Updating

### Check for Updates

```bash
mc update status
```

Shows:
- Current version
- Previous version (if any)
- Install location

### Manual Update

Download and run the installer again:

```bash
TAG=v0.2.2  # when available
curl -fsSL "https://github.com/JUANWANG-BUAA/mc-server-tui/releases/download/$TAG/install.sh" | sh
```

The installer:
- Installs to a new version directory
- Updates the `current` symlink
- Keeps your previous version for rollback
- Preserves all instance data

### Rollback to Previous Version

```bash
mc update rollback --dry-run  # Preview
mc update rollback            # Execute
```

This swaps `current` and `previous`, then restarts the daemon.

## Uninstallation

### Remove Program (Keep Data)

```bash
mc uninstall
```

This removes:
- Program files (`~/.local/share/mc-server-tui/`)
- Launcher (`~/.local/bin/mc`)
- Service file (`~/Library/LaunchAgents/...`)

This keeps:
- Instance data and worlds
- Configuration
- Backups

### Remove Everything

```bash
# First, note the data directory
mc doctor | grep state

# Uninstall program
mc uninstall

# Then manually remove data (replace with actual path)
rm -rf ~/Library/Application\ Support/org.mc-server-tui.mc-server-tui
rm -rf ~/Library/Caches/org.mc-server-tui.mc-server-tui
rm -rf ~/Library/Logs/mc-server-tui
```

## Building from Source (Intel Mac or Latest Features)

If you need an Intel build or want the latest code:

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/JUANWANG-BUAA/mc-server-tui.git
cd mc-server-tui
cargo build --workspace --release

# Install
./target/release/mc setup
./target/release/mc
```

The source-build service points to `target/release/mc` for development.

## Getting Help

- Check `mc doctor --json` for diagnostics
- Review daemon log: `~/Library/Logs/mc-server-tui/daemon.log`
- [FAQ](../FAQ.md) for common issues
- [GitHub Issues](https://github.com/JUANWANG-BUAA/mc-server-tui/issues) for bugs

---

**Next:** [Create your first server →](../USER_GUIDE.md#4-create-the-first-server)
