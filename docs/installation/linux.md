# Linux Installation Guide

> **For mc-server-tui v0.2.1** — Detailed Linux installation and troubleshooting

## System Requirements

### Supported Distributions
- ✅ **Ubuntu** 20.04+ / **Debian** 11+
- ✅ **Fedora** 36+ / **RHEL** 8+ / **Rocky Linux** 8+
- ✅ **Arch Linux** (current)
- ✅ **openSUSE** Leap 15.4+ / Tumbleweed
- ✅ Other distributions with **systemd** and glibc 2.31+

### Supported Architectures
- ✅ **x86_64** (Intel/AMD 64-bit)
- ✅ **aarch64** (ARM64, including Raspberry Pi 4/5)
- ❌ **armv7l** (32-bit ARM, not published)

### Software Requirements
- **Kernel**: 4.15+ (for modern Unix socket features)
- **systemd**: Required for user service management
- **Java**: 21+ for Minecraft 1.21.x, 25+ for 26.x
- **Tools**: curl, tar, python3 (usually pre-installed)
- **Terminal**: 80×24 minimum (any terminal emulator works)

## Pre-Installation

### 1. Check Your Architecture

```bash
uname -m
```

- `x86_64` or `amd64` → 64-bit Intel/AMD ✅
- `aarch64` or `arm64` → 64-bit ARM ✅
- `armv7l` → 32-bit ARM ❌ (not published in v0.2.1)

### 2. Install Java

#### Ubuntu / Debian

```bash
# Update package list
sudo apt update

# Install Java 21
sudo apt install openjdk-21-jdk

# Verify
java -version
```

#### Fedora / RHEL / Rocky Linux

```bash
# Fedora
sudo dnf install java-21-openjdk

# RHEL/Rocky 8 (enable EPEL first)
sudo dnf install epel-release
sudo dnf install java-21-openjdk

# Verify
java -version
```

#### Arch Linux

```bash
# Install Java 21
sudo pacman -S jdk21-openjdk

# Set as default (if multiple Java versions)
sudo archlinux-java set java-21-openjdk

# Verify
java -version
```

#### openSUSE

```bash
# Leap / Tumbleweed
sudo zypper install java-21-openjdk

# Verify
java -version
```

#### Alternative: SDKMAN (Any Distribution)

```bash
# Install SDKMAN
curl -s "https://get.sdkman.io" | bash
source "$HOME/.sdkman/bin/sdkman-init.sh"

# Install Java 21
sdk install java 21-tem

# Verify
java -version
```

### 3. Check systemd User Service Support

```bash
systemctl --user status
```

If this works, you have systemd user services. If not:

```bash
# Enable lingering (service starts without active login)
sudo loginctl enable-linger $USER
```

### 4. Check Available Disk Space

```bash
df -h ~
```

You need at least **500MB** free:
- mc-server-tui binary: ~10–20MB
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
2. **Selects** appropriate binary:
   - `x86_64-unknown-linux-musl` (static, no libc dependency)
   - `x86_64-unknown-linux-gnu` (glibc fallback)
   - `aarch64-unknown-linux-musl` or `gnu` (ARM64)
3. **Verifies** SHA-256 checksum (not a code signature)
4. **Installs** to `~/.local/share/mc-server-tui/versions/v0.2.1/`
5. **Creates** symlink at `~/.local/bin/mc`

### Post-Install: Add to PATH

If `~/.local/bin` is not in your PATH:

```bash
# For bash (add to ~/.bashrc)
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# For zsh (add to ~/.zshrc)
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc

# For fish (add to ~/.config/fish/config.fish)
echo 'set -gx PATH $HOME/.local/bin $PATH' >> ~/.config/fish/config.fish
source ~/.config/fish/config.fish
```

Verify:

```bash
which mc
# Should show: /home/yourname/.local/bin/mc
```

## Setup

### Run Initial Setup

```bash
mc setup
```

This will:
1. Create `~/.config/org.mc-server-tui.mc-server-tui/config.toml`
2. Check for Java (shows warning if < Java 21)
3. Install a systemd user unit
4. Enable and start the service
5. Wait for daemon to become ready

**Expected output:**

```
config: /home/yourname/.config/org.mc-server-tui.mc-server-tui/config.toml
state: /home/yourname/.local/state/org.mc-server-tui.mc-server-tui
socket: /home/yourname/.local/state/org.mc-server-tui.mc-server-tui/run/daemon.sock
user-agent: mc-server-tui/0.2.1 ...
java: /usr/bin/java (Java 21)
service: systemd installed at ~/.config/systemd/user/mc-server-tui.service
daemon: ready
next: run `mc` to open the TUI
```

### Understanding the systemd User Service

The setup creates a **user unit** (not a system service):
- Runs as your user (no sudo required)
- Starts automatically when you log in (if lingering is enabled)
- Managed with `systemctl --user`
- Logs to systemd journal

**Service file location:**
```
~/.config/systemd/user/mc-server-tui.service
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
target: x86_64-unknown-linux-musl
profile: release
API protocol: 8–11
state schema: 2
runner protocol: 1
service manager: systemd
```

### Check Daemon Status

```bash
mc status
```

Should show:
```
daemon: running
socket: /home/yourname/.local/state/.../run/daemon.sock
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

## File Locations on Linux

| Purpose | Location |
|---------|----------|
| Program files | `~/.local/share/mc-server-tui/versions/v0.2.1/` |
| Launcher symlink | `~/.local/bin/mc` |
| Configuration | `~/.config/org.mc-server-tui.mc-server-tui/config.toml` |
| Instance data | `~/.local/state/org.mc-server-tui.mc-server-tui/instances/` |
| Worlds | `~/.local/state/.../instances/<uuid>/server/` |
| Backups | `~/.local/state/.../instances/<uuid>/backups/` |
| Task database | `~/.local/state/.../tasks.db` |
| Download cache | `~/.cache/org.mc-server-tui.mc-server-tui/` |
| Socket | `~/.local/state/.../run/daemon.sock` |
| Service file | `~/.config/systemd/user/mc-server-tui.service` |
| Logs | `journalctl --user -u mc-server-tui` |

## Troubleshooting

### "Daemon did not become ready"

**Check the service status:**

```bash
systemctl --user status mc-server-tui
```

Should show `active (running)`. If not:

```bash
# View recent logs
journalctl --user -u mc-server-tui -n 50

# Restart service
mc service restart
```

Common issues:
- Socket path too long (very rare with default paths)
- Permissions issue (check `~/.local/state/` ownership)
- SELinux blocking (see SELinux section below)

**If still failing, try manual mode:**

```bash
mc service stop
mc daemon run  # Watch for errors
```

### "Java not found" during setup

Java is not on PATH or not installed.

**Verify Java is installed:**

```bash
which java
java -version
```

If installed but not found:

```bash
# Find Java installations
update-alternatives --list java  # Debian/Ubuntu
alternatives --list java         # Fedora/RHEL

# Add to PATH (adjust path as needed)
echo 'export PATH="/usr/lib/jvm/java-21-openjdk/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### "mc: command not found"

The launcher is not on your PATH.

**Quick fix:**

```bash
export PATH="$HOME/.local/bin:$PATH"
```

**Permanent fix:** Add the export to `~/.bashrc` or `~/.zshrc`

### systemd User Service Won't Start at Login

Enable lingering to start the service without an active session:

```bash
sudo loginctl enable-linger $USER
```

Verify:

```bash
loginctl show-user $USER | grep Linger
# Should show: Linger=yes
```

### Socket Path Too Long

Unix domain sockets have a 108-character path limit. If you get this error:

```bash
# Check current socket path length
mc doctor | grep socket

# Use a shorter state directory
mc setup --no-service
# Edit config.toml and set a shorter state_dir
mc service install
```

### Port 25565 Already in Use

Another Minecraft server or service is using the default port.

**Find what's using it:**

```bash
sudo ss -tulpn | grep :25565
# or
sudo lsof -i :25565
```

**Options:**
1. Stop the other server
2. Use a different port when creating your server in the TUI wizard
3. Change the port in `server.properties` and restart

## SELinux Considerations (Fedora/RHEL/Rocky)

If SELinux is enforcing, it may block the daemon or Java.

**Check SELinux status:**

```bash
getenforce
```

If `Enforcing`:

**Option 1: Allow in SELinux (Recommended)**

```bash
# View denials
sudo ausearch -m avc -ts recent | grep mc-server-tui

# Generate and install policy
sudo ausearch -m avc -ts recent | audit2allow -M mc-server-tui-local
sudo semodule -i mc-server-tui-local.pp
```

**Option 2: Set SELinux to Permissive (Testing Only)**

```bash
# Temporary
sudo setenforce 0

# Permanent (not recommended for production)
sudo sed -i 's/^SELINUX=enforcing/SELINUX=permissive/' /etc/selinux/config
```

**Option 3: Label the binary**

```bash
# Allow execution
sudo chcon -t bin_t ~/.local/share/mc-server-tui/current/bin/mc
```

## Firewall Configuration

### Ubuntu / Debian (ufw)

```bash
# Allow Minecraft port
sudo ufw allow 25565/tcp

# Check status
sudo ufw status
```

### Fedora / RHEL / Rocky (firewalld)

```bash
# Allow Minecraft port
sudo firewall-cmd --permanent --add-port=25565/tcp
sudo firewall-cmd --reload

# Check
sudo firewall-cmd --list-ports
```

### Arch Linux (iptables or firewalld)

```bash
# If using firewalld
sudo firewall-cmd --permanent --add-port=25565/tcp
sudo firewall-cmd --reload

# If using iptables directly
sudo iptables -A INPUT -p tcp --dport 25565 -j ACCEPT
sudo iptables-save | sudo tee /etc/iptables/iptables.rules
```

### Manual iptables Rule

```bash
sudo iptables -A INPUT -p tcp --dport 25565 -j ACCEPT
```

To persist across reboots, save rules per your distribution's method.

## Service Management

### Start/Stop Service

```bash
mc service start       # Start daemon
mc service stop        # Stop daemon (keeps Java running)
mc service restart     # Restart daemon
mc service status      # Check status
```

### systemd Commands

```bash
# Status
systemctl --user status mc-server-tui

# Start
systemctl --user start mc-server-tui

# Stop
systemctl --user stop mc-server-tui

# Restart
systemctl --user restart mc-server-tui

# View logs
journalctl --user -u mc-server-tui -f

# Enable auto-start
systemctl --user enable mc-server-tui

# Disable auto-start
systemctl --user disable mc-server-tui
```

## Performance Tuning

### Increase File Descriptor Limits

Minecraft servers can open many files. Increase limits:

```bash
# Edit /etc/security/limits.conf (requires sudo)
sudo nano /etc/security/limits.conf

# Add these lines (replace 'username' with your username)
username soft nofile 4096
username hard nofile 8192

# Logout and login again, then verify
ulimit -n
```

### Allocate More RAM

When creating a server in the TUI, you'll be asked for memory allocation (1–8 GB). For production:
- Small server (1–5 players): 2–4 GB
- Medium server (5–20 players): 4–6 GB
- Large server (20+ players): 6–8 GB

Edit after creation:
1. Stop the server
2. Edit instance configuration (future UI feature)
3. Or edit `instances/<uuid>/instance.toml` manually
4. Restart

### Use Performance CPU Governor

```bash
# Check current governor
cat /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# Set to performance (temporary)
echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor
```

For permanent setting, install and configure `cpupower` or edit systemd service.

## Updating

### Check for Updates

```bash
mc update status
```

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

### Rollback

```bash
mc update rollback --dry-run  # Preview
mc update rollback            # Execute
```

## Uninstallation

### Remove Program (Keep Data)

```bash
mc uninstall
```

This removes:
- Program files (`~/.local/share/mc-server-tui/`)
- Launcher (`~/.local/bin/mc`)
- Service file (`~/.config/systemd/user/mc-server-tui.service`)

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

# Then manually remove data (adjust paths)
rm -rf ~/.local/state/org.mc-server-tui.mc-server-tui
rm -rf ~/.config/org.mc-server-tui.mc-server-tui
rm -rf ~/.cache/org.mc-server-tui.mc-server-tui
```

## Building from Source

For the latest features or unsupported architectures:

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Clone and build
git clone https://github.com/JUANWANG-BUAA/mc-server-tui.git
cd mc-server-tui
cargo build --workspace --release

# Install
./target/release/mc setup
./target/release/mc
```

## Distribution-Specific Notes

### Raspberry Pi (ARM64)

Works on Raspberry Pi 4 and 5 (64-bit OS):

```bash
# Ensure 64-bit OS
uname -m  # Should show aarch64

# Install Java 21
sudo apt install openjdk-21-jdk

# Install mc-server-tui (use ARM64 build)
TAG=v0.2.1
curl -fsSL "https://github.com/JUANWANG-BUAA/mc-server-tui/releases/download/$TAG/install.sh" | sh
```

**Performance**: Raspberry Pi 4 (4GB+) can handle 1–5 player servers. Reduce view distance and pre-generate chunks for better performance.

### WSL2 (Windows Subsystem for Linux)

mc-server-tui should work in WSL2, but:

- Use Ubuntu or Debian on WSL2
- Ensure systemd is enabled (WSL 0.67.6+)
- Port forwarding from Windows host requires netsh rules

**Enable systemd in WSL2:**

Edit `/etc/wsl.conf`:

```ini
[boot]
systemd=true
```

Restart WSL: `wsl --shutdown` in PowerShell, then reopen.

## Getting Help

- Check `mc doctor --json` for diagnostics
- View daemon logs: `journalctl --user -u mc-server-tui -n 100`
- [FAQ](../FAQ.md) for common issues
- [GitHub Issues](https://github.com/JUANWANG-BUAA/mc-server-tui/issues) for bugs

---

**Next:** [Create your first server →](../USER_GUIDE.md#4-create-the-first-server)
