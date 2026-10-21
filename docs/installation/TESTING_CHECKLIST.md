# Installation Testing Checklist

> **For mc-server-tui release validation**
>
> Use this checklist to validate installation on each supported platform before release.

## Pre-Release Validation

### Release Artifact Checks

- [ ] Release manifest schema version is correct
- [ ] All target binaries are present:
  - [ ] `aarch64-apple-darwin` (macOS Apple Silicon)
  - [ ] `x86_64-unknown-linux-musl` (Linux x86_64 static)
  - [ ] `x86_64-unknown-linux-gnu` (Linux x86_64 glibc)
  - [ ] `aarch64-unknown-linux-musl` (Linux ARM64 static)
  - [ ] `aarch64-unknown-linux-gnu` (Linux ARM64 glibc)
- [ ] SHA-256 checksums are present for all artifacts
- [ ] File sizes are recorded in manifest
- [ ] URLs are correct (relative or absolute)
- [ ] Channel is set correctly (`stable` or `beta`)
- [ ] Version string matches git tag

### Binary Verification

For each target:
- [ ] Binary is executable
- [ ] `mc version --json` runs successfully
- [ ] Correct target architecture reported
- [ ] API protocol range is correct
- [ ] State schema version is correct
- [ ] Runner protocol version is correct

---

## macOS Apple Silicon (aarch64-apple-darwin)

### Environment Setup
- [ ] Fresh macOS 13+ VM or test machine
- [ ] Java 21+ installed via Homebrew
- [ ] Terminal set to 80×24 minimum
- [ ] `~/.local/bin` not in PATH initially (to test PATH setup)

### Installation

**Fresh Install**
- [ ] Run one-command installer: `curl ... | sh`
- [ ] Installer detects platform correctly
- [ ] Installer downloads artifact successfully
- [ ] SHA-256 verification passes
- [ ] Files installed to `~/.local/share/mc-server-tui/`
- [ ] Launcher created at `~/.local/bin/mc`
- [ ] PATH warning shown (if applicable)

**Setup**
- [ ] `mc setup` creates config.toml at correct location
- [ ] Java detected and version shown
- [ ] Warning shown if Java < 21
- [ ] launchd service file created
- [ ] Service starts automatically
- [ ] Daemon becomes ready within timeout
- [ ] "next: run `mc`" message shown

**Verification**
- [ ] `mc version` shows correct version and target
- [ ] `mc status` reports daemon running
- [ ] `mc doctor` shows all paths correctly
- [ ] Socket file exists and is readable
- [ ] Daemon log file created

### TUI Operations

- [ ] `mc` launches TUI successfully
- [ ] TUI renders correctly (80×24)
- [ ] `/create` wizard starts
- [ ] Can create Paper server (accept EULA)
- [ ] Server starts successfully
- [ ] Can stop server gracefully
- [ ] Can exit TUI without stopping server
- [ ] Can reconnect to running daemon after TUI exit

### Service Management

- [ ] `mc service status` reports correct state
- [ ] `mc service stop` stops daemon
- [ ] Java process remains running after daemon stop
- [ ] `mc service start` restarts daemon
- [ ] Daemon reattaches to running Java process
- [ ] `mc service restart` cycles daemon without killing Java

### Upgrade Path

- [ ] Can install newer version over existing
- [ ] `current` symlink updated
- [ ] `previous` symlink preserved
- [ ] Old version directory retained
- [ ] `mc update status` shows both versions
- [ ] `mc update rollback` works correctly
- [ ] Service restarts with rolled-back version

### Uninstallation

- [ ] `mc uninstall --dry-run` shows plan
- [ ] `mc uninstall` removes program files
- [ ] Service file removed
- [ ] Launcher symlink removed
- [ ] Instance data preserved
- [ ] Configuration preserved

### Error Scenarios

- [ ] Install over existing version without `--force` fails with clear message
- [ ] Install with insufficient disk space fails gracefully
- [ ] Setup without Java shows helpful warning
- [ ] Setup with daemon already running handles gracefully
- [ ] Invalid manifest URL fails with troubleshooting steps
- [ ] SHA-256 mismatch fails with recovery instructions

---

## Ubuntu 22.04 LTS (x86_64)

### Environment Setup
- [ ] Fresh Ubuntu 22.04 VM or container
- [ ] Java 21+ installed via apt
- [ ] systemd user services enabled
- [ ] `~/.local/bin` not in PATH initially

### Installation

**Fresh Install**
- [ ] Run one-command installer
- [ ] Installer selects correct musl or gnu target
- [ ] Installation completes successfully
- [ ] Launcher created

**Setup**
- [ ] `mc setup` creates config.toml
- [ ] Java detected correctly
- [ ] systemd user unit created
- [ ] Service enabled and started
- [ ] Daemon becomes ready

**Verification**
- [ ] `mc version` correct
- [ ] `mc status` reports daemon running
- [ ] `systemctl --user status mc-server-tui` shows active
- [ ] `journalctl --user -u mc-server-tui` shows logs

### TUI Operations

- [ ] `mc` launches TUI
- [ ] Can create and start server
- [ ] Server operates correctly
- [ ] Can stop server gracefully

### Service Management

- [ ] `systemctl --user` commands work
- [ ] `mc service` commands work
- [ ] Daemon survives user logout (with lingering)
- [ ] Service starts on login

### Uninstallation

- [ ] `mc uninstall` removes program
- [ ] systemd unit removed
- [ ] Instance data preserved

---

## Fedora 39 (x86_64)

### Environment Setup
- [ ] Fresh Fedora 39 VM
- [ ] Java 21+ installed via dnf
- [ ] SELinux enforcing (default)
- [ ] firewalld active

### Installation

**Fresh Install**
- [ ] Installer runs successfully
- [ ] Binary executes despite SELinux
- [ ] No AVC denials block installation

**Setup**
- [ ] `mc setup` completes
- [ ] systemd user unit created and started
- [ ] Daemon runs under SELinux

**SELinux Checks**
- [ ] No unexpected SELinux denials
- [ ] Socket creation allowed
- [ ] Java execution allowed
- [ ] If denials occur, documented workaround exists

**Firewall**
- [ ] Server can start on port 25565
- [ ] Manual firewall rule documented
- [ ] Can connect locally without firewall rule

---

## Debian 12 (x86_64)

### Environment Setup
- [ ] Fresh Debian 12 minimal install
- [ ] Java 21+ installed
- [ ] systemd user services available

### Installation & Validation

Follow Ubuntu checklist (same .deb-based workflow)

- [ ] Installation successful
- [ ] Setup completes
- [ ] TUI operates
- [ ] Service management works

---

## Arch Linux (x86_64)

### Environment Setup
- [ ] Current Arch installation
- [ ] Java 21+ installed via pacman
- [ ] systemd user services enabled

### Installation & Validation

Follow Fedora checklist (same systemd workflow)

- [ ] Installation successful
- [ ] Setup completes
- [ ] TUI operates
- [ ] Can use `systemctl --user`

---

## Raspberry Pi 4/5 (aarch64, Raspberry Pi OS 64-bit)

### Environment Setup
- [ ] Raspberry Pi 4 (4GB+) or Pi 5
- [ ] 64-bit Raspberry Pi OS
- [ ] Java 21+ installed
- [ ] At least 500MB free space

### Installation

**Fresh Install**
- [ ] Installer detects ARM64 correctly
- [ ] Downloads aarch64 binary
- [ ] Installation completes successfully

**Setup**
- [ ] `mc setup` completes
- [ ] Java detected
- [ ] systemd service created
- [ ] Daemon starts

### Performance

- [ ] Can create Paper 1.21.x server
- [ ] Server starts (may be slow)
- [ ] Can handle 1–2 players
- [ ] Memory limits respected
- [ ] CPU usage reasonable for ARM

### Notes
- [ ] Document performance expectations
- [ ] Recommend reduced view distance
- [ ] Suggest pre-generating chunks

---

## Automated Test Scripts

### Installation Smoke Test

```bash
#!/bin/bash
# Test basic installation flow
set -e

TAG="${1:-v0.2.1}"
PREFIX="/tmp/mc-install-test-$$"

# Install
MC_SERVER_TUI_MANIFEST_URL="https://github.com/JUANWANG-BUAA/mc-server-tui/releases/download/$TAG/release-manifest.json" \
  curl -fsSL "https://github.com/JUANWANG-BUAA/mc-server-tui/releases/download/$TAG/install.sh" | \
  sh -s -- --prefix "$PREFIX" --root "$PREFIX/share" --dry-run

echo "✓ Installer dry-run passed"

# Actual install
MC_SERVER_TUI_MANIFEST_URL="https://github.com/JUANWANG-BUAA/mc-server-tui/releases/download/$TAG/release-manifest.json" \
  curl -fsSL "https://github.com/JUANWANG-BUAA/mc-server-tui/releases/download/$TAG/install.sh" | \
  sh -s -- --prefix "$PREFIX" --root "$PREFIX/share"

# Verify
test -x "$PREFIX/bin/mc" || exit 1
test -d "$PREFIX/share/mc-server-tui/versions" || exit 1

"$PREFIX/bin/mc" version --json | grep -q "mc-server-tui" || exit 1

echo "✓ Installation verified"

# Cleanup
rm -rf "$PREFIX"
echo "✓ All tests passed"
```

### Setup Smoke Test

```bash
#!/bin/bash
# Test setup without service
set -e

mc setup --dry-run | grep -q "config:" || exit 1
echo "✓ Setup dry-run passed"

# Note: Full setup test requires environment-specific handling
```

---

## Platform Compatibility Matrix

| Platform | Architecture | Status | Tested Version | Notes |
|----------|--------------|--------|----------------|-------|
| macOS | Apple Silicon (ARM64) | ✅ Supported | 13.0+ | Native binary |
| macOS | Intel (x86_64) | ⚠️ Not published | 13.0+ | Build from source |
| Ubuntu | x86_64 | ✅ Supported | 20.04+ | Uses musl or gnu |
| Ubuntu | ARM64 | ✅ Supported | 20.04+ | Uses musl or gnu |
| Debian | x86_64 | ✅ Supported | 11+ | Same as Ubuntu |
| Debian | ARM64 | ✅ Supported | 11+ | Same as Ubuntu |
| Fedora | x86_64 | ✅ Supported | 36+ | SELinux tested |
| Fedora | ARM64 | ⚠️ Untested | 36+ | Should work |
| RHEL/Rocky | x86_64 | ✅ Supported | 8+ | Requires EPEL |
| Arch Linux | x86_64 | ✅ Supported | Current | Rolling release |
| openSUSE | x86_64 | ⚠️ Untested | Leap 15.4+ | Should work |
| Raspberry Pi OS | ARM64 | ✅ Supported | 64-bit only | Pi 4/5 only |
| Windows | Any | ❌ Unsupported | N/A | No Unix socket |
| WSL2 | x86_64 | ⚠️ Experimental | Ubuntu/Debian | Requires systemd |

**Legend:**
- ✅ Supported: Officially tested and documented
- ⚠️ Untested: Should work but not validated
- ⚠️ Not published: Works but no release binary
- ❌ Unsupported: Does not work or not planned

---

## Release Sign-Off

Before publishing a release, verify:

- [ ] All "✅ Supported" platforms tested
- [ ] Installation guide reviewed and updated
- [ ] Known issues documented
- [ ] CHANGELOG updated with installation changes
- [ ] README installation section accurate
- [ ] Website installation page updated
- [ ] GitHub release notes include installation instructions

**Tested by:** _____________  
**Date:** _____________  
**Version:** _____________  
**Sign-off:** _____________
