# Marketing Copy & Social Media Templates

> Version 1.0 · 2026-10-20 · Ready-to-use copy for various channels

## Core Messaging

### Primary Message
Terminal-native Minecraft server management with verified operations, supervised processes, and transactional backups—built for SSH administrators who value reliability over automation.

### Key Benefits (Pick 3-4 per context)
- ✓ Supervised processes that survive daemon restarts
- ✓ SHA-256-verified downloads and backup manifests
- ✓ Transactional world backups with rollback journals
- ✓ Terminal workflow without browser dependencies
- ✓ Process reattachment using identity markers
- ✓ Durable task queues that resume after crashes
- ✓ Configuration diffs before applying changes
- ✓ Audit trails for every operation

## Website & Landing Page

### Hero Section

**Variant A (Technical):**
```
Minecraft Server Management for SSH Administrators

Terminal-native supervision · Verified backups · Audit trails
No browser. No guessing. No silent failures.

[Install v0.2.1 (alpha)]  [Read the docs]
```

**Variant B (Problem-Solution):**
```
Manage Minecraft Servers Without Installing a Web Panel

Verified backups you can prove are correct.
Supervised processes that survive restarts.
All from your terminal.

[Get started]  [See why]
```

### Feature Blocks

**Block 1: Reliable**
> **Supervised Long-Running Processes**
> 
> Each instance runs under a resident runner with process identity markers. The TUI or daemon can restart without killing your Java process. No more guessing whether the server is actually running or just has a stale PID file.

**Block 2: Auditable**
> **Evidence-Based Operations**
> 
> Paper builds lock to exact build numbers, sizes, and SHA-256 hashes. Configuration edits show diffs before applying. Startup failures cite specific log lines, not vague error messages.

**Block 3: Recoverable**
> **Transactional Backups with Verification**
> 
> SHA-256 manifests. Paged verification. Restore previews showing exactly what will change. Rollback journals for every operation. Prove your backups are correct before disaster strikes.

### "Is This For Me?" Section

**You'll like this if:**
- You manage servers over SSH
- You're comfortable with terminals and basic sysadmin tasks
- You want backups you can verify, not just hope are correct
- You value operational transparency over automated convenience
- You prepare servers on macOS and deploy to Linux

**This isn't for you if:**
- You need a web UI or multi-user access control
- You're setting up your first Minecraft server
- You want one-click everything with no terminal use
- You need multi-node orchestration across data centers

### FAQ for Landing Page

**Q: Why not just use a web panel?**  
A: Web panels are great for multi-user teams and browser-based workflows. If you SSH into servers and live in a terminal, this gives you the same supervision and verification without the browser dependency.

**Q: Is this beginner-friendly?**  
A: No. You should understand terminals, SSH, Java versions, ports, and memory allocation. This is for technical server administrators, not first-time operators.

**Q: How is this different from systemd + scripts?**  
A: We provide structured supervision, verified downloads, transactional backups, and audit trails—things you'd build yourself with scripts, but standardized and maintained.

**Q: Does it support Forge/Fabric/Paper?**  
A: Currently Paper (stable) and Fabric (alpha). Vanilla provider in progress. Forge/NeoForge planned for future releases.

## Social Media

### Twitter/X Announcement Thread

**Tweet 1 (Hook):**
```
If you manage Minecraft servers over SSH and want backups you can *prove* are correct—not just files you *hope* work—I built something for that workflow.

Introducing mc-server-tui: terminal-native server management.

🧵 What it does and who it's for:
```

**Tweet 2 (Problem):**
```
Most server tools fall into two camps:

1. Web panels: require browsers, databases, and broad privileges
2. Raw screen/systemd: full control but no verification or supervision

There's a gap for SSH admins who want structure without giving up the terminal.
```

**Tweet 3 (Solution):**
```
mc-server-tui provides:

✓ Resident runners (daemon restarts don't kill Java)
✓ SHA-256-verified downloads and backups
✓ Transactional world restores with rollback journals
✓ Terminal-native workflow, no browser

Built with Rust. MIT/Apache-2.0.
```

**Tweet 4 (Target User):**
```
This is for technical server admins who:
- SSH into Linux hosts regularly
- Prepare servers locally (macOS) and deploy to production
- Value operational transparency over automation
- Want audit trails for configuration changes

NOT for first-time server operators.
```

**Tweet 5 (Status + CTA):**
```
Current status: Alpha (unsigned, use at your own risk)
Platforms: macOS 13+ (Apple Silicon), Linux (x86_64/arm64)

GitHub: [link]
Docs: [link]
Website: [link]

Open to feedback. If this scratches your itch, give it a try.
```

### Reddit r/admincraft Post

**Title:**
```
[Tool] mc-server-tui: Terminal manager with verified backups for SSH workflows (alpha)
```

**Body:**
```
I've been building a terminal-first Minecraft server manager for admins who work over SSH and want operational transparency without installing a web panel.

**What it does:**

- **Supervised processes:** Resident runners with process identity markers. TUI/daemon restarts don't kill your Java process. The system can reattach to running servers.

- **Verified operations:** SHA-256-locked Paper builds. Backup manifests with paged verification. Configuration diffs before applying changes.

- **Transactional backups:** Restore previews showing exactly what will change. Explicit confirmation tokens. Rollback journals for recovery.

- **Terminal-native:** All daily operations (start/stop/logs/console/tasks) in one shell session. No browser tabs, no separate admin panel.

**Target users:**

If you:
- Manage servers via SSH
- Want backups you can verify, not just hope are correct
- Prepare servers locally (macOS) and deploy to Linux
- Value reliability and auditability over automated convenience

...then this is designed for your workflow.

**NOT for:**
- First-time server operators
- Users who need web UI or multi-user RBAC
- One-click automation without understanding what's happening

**Current status:**

Alpha (unsigned). Supports Paper (stable) and Fabric (alpha). macOS 13+ Apple Silicon and Linux x86_64/arm64.

Built with Rust. Dual-licensed MIT/Apache-2.0.

- GitHub: [link]
- Docs: [link]
- Website: [link]

**Honest positioning:** TUI market is small. This isn't trying to replace web panels for multi-user teams. It's for the gap between panels (too heavyweight) and raw screen sessions (no supervision).

Open to feedback, especially from admins running production servers. If you try it, I'd love to know what breaks.
```

### Hacker News "Show HN" Post

**Title:**
```
Show HN: mc-server-tui – Terminal Minecraft server manager with verified backups
```

**Description:**
```
I built a terminal-first Minecraft server manager for SSH-based workflows. It's designed for server administrators who want:

- Process supervision that survives tool restarts (resident runners with identity markers)
- Verified downloads and backups (SHA-256-locked builds and transactional restore)
- Terminal-native operations without browser dependencies
- Audit trails for configuration changes

The target user is someone who SSH's into Linux servers, prepares locally on macOS, and values operational transparency over automated convenience. It's explicitly NOT for first-time server operators.

Current state: Alpha (unsigned). Supports Paper and Fabric. Built with Rust and ratatui for the TUI. MIT/Apache-2.0 dual-licensed.

Technical details:
- Resident supervisor per instance (TUI/daemon can restart independently)
- SQLite-backed durable tasks (downloads/backups resume after crashes)
- SHA-256 manifests for backups with paged verification
- Process identity markers for reattachment

The TUI market for Minecraft is tiny—most operators use web panels (MCSManager, etc.) or raw systemd. This exists for the niche that finds browsers slower than terminals but wants more structure than homegrown scripts.

GitHub: [link]
Docs: [link]

Would appreciate feedback, especially on the backup verification and process supervision models.
```

### Mastodon Post

```
🎮 New project: mc-server-tui

Terminal-native Minecraft server manager for SSH administrators.

✓ Supervised processes (daemon restarts ≠ killed servers)
✓ SHA-256-verified downloads & backups
✓ Transactional world restores
✓ No browser required

Built for admins who value reliability over automation.

Not for beginners. Alpha stage. Rust/MIT/Apache-2.0.

[link]

#Minecraft #SysAdmin #Rust #TUI #OpenSource
```

## Email / Newsletter

### Announcement Email

**Subject:** Introducing mc-server-tui: Verified Minecraft server management

**Body:**
```
Hi [First Name],

I wanted to share a project I've been working on: mc-server-tui, a terminal-first Minecraft server manager.

**What it is:**

A tool for server administrators who manage Minecraft infrastructure over SSH and need:
- Backups they can verify (not just hope are correct)
- Process supervision that survives tool restarts
- Terminal workflow without browser dependencies
- Audit trails for every operation

**Who it's for:**

Technical admins who:
- SSH into Linux servers regularly
- Prepare servers locally (macOS) and deploy to production
- Value operational transparency over automated convenience

This is NOT a beginner tool. You should be comfortable with terminals, Java versions, and server concepts.

**Current status:**

Alpha release (unsigned). Supports Paper and Fabric. macOS 13+ Apple Silicon and Linux x86_64/arm64. Built with Rust. MIT/Apache-2.0.

**Key features:**

✓ Resident runners with process identity markers
✓ SHA-256-verified Paper builds and backup manifests
✓ Transactional restores with rollback journals
✓ Durable task queues that survive crashes
✓ Configuration diffs before applying changes

**Links:**

- GitHub: [link]
- Documentation: [link]
- Website: [link]

**Honest positioning:**

The TUI market for Minecraft is small. Most operators use web panels or raw screen sessions. This exists for the gap: admins who want CLI control and verifiable operations that panels omit.

If this sounds useful for your workflow, give it a try. If you hit issues or have feedback, I'd love to hear it.

[Your Name]
```

## Product Hunt Launch

### Tagline (60 chars)
```
Terminal Minecraft server manager with verified operations
```

### First Comment (Maker's Introduction)
```
Hey Product Hunt! 👋

I'm [Name], and I built mc-server-tui to scratch a specific itch: managing Minecraft servers over SSH without installing a web panel.

**The problem:**

Most server tools assume you want a browser UI or don't mind learning a web admin panel. But if you SSH into servers daily and live in a terminal, that workflow feels backward.

At the same time, raw systemd + scripts gives you control but no standardized verification, supervision, or recovery.

**The solution:**

mc-server-tui provides:

✓ **Supervised processes:** Resident runners with identity markers. TUI/daemon restarts don't kill Java.

✓ **Verified operations:** SHA-256-locked downloads. Backup manifests with paged verification. Configuration diffs before applying.

✓ **Transactional backups:** Restore previews. Explicit confirmation. Rollback journals.

✓ **Terminal-native:** All daily operations in one shell session. No browser tabs.

**Who it's for:**

Server admins who:
- Manage infrastructure over SSH
- Prepare locally (macOS) and deploy to Linux
- Value reliability and auditability over automation
- Want backups they can prove are correct

**Who it's NOT for:**

- First-time server operators
- Users who need web UI or multi-user RBAC
- One-click automation without understanding

**Current status:**

Alpha (unsigned). Supports Paper and Fabric. macOS/Linux. Rust. MIT/Apache-2.0.

**Honest positioning:**

TUI market is tiny. This isn't trying to replace MCSManager for multi-user teams. It's for the niche that finds browsers slower than terminals but wants more than homegrown scripts.

Links in the main post. Would love feedback, especially from production server operators!
```

## Conference / Community Talk Abstract

### Title
```
Building Reliable Server Tools: Process Supervision and Verified Backups in Rust
```

### Abstract (300 words)
```
Most Minecraft server management tools fall into two categories: web panels with browser UIs and database dependencies, or raw systemd + scripts with no standardization. There's a gap for terminal-native tools that provide supervision and verification without heavyweight infrastructure.

This talk explores the design and implementation of mc-server-tui, a terminal-first Minecraft server manager built with Rust and ratatui. We'll cover three key challenges:

1. **Process supervision that survives tool restarts**
   - How resident runners with identity markers enable daemon reattachment
   - Why PID files aren't enough for process ownership
   - Handling Java process lifecycle across tool upgrades

2. **Verifiable backups and transactional restores**
   - SHA-256 manifests and paged verification
   - Transactional restore design with rollback journals
   - Why "file exists" isn't proof of backup correctness

3. **Durable task queues for long-running operations**
   - SQLite-backed task storage with event cursors
   - Cooperative cancellation and progress tracking
   - Resuming downloads after daemon crashes

We'll discuss architectural tradeoffs, failure modes, and lessons learned from alpha testing. The talk is aimed at developers building operational tools who want to balance reliability, transparency, and terminal-native UX.

Target audience: Systems programmers, DevOps engineers, and tool builders interested in process supervision, backup verification, and Rust TUI development.

Key takeaways:
- Patterns for supervised processes that survive tool restarts
- Verification strategies for backups and downloads
- Designing terminal UIs for operational transparency

Open source (MIT/Apache-2.0). GitHub: [link]
```

## Documentation Site Variations

### Homepage Hero

**Variant 1 (Direct):**
```markdown
# mc-server-tui

Terminal-native Minecraft server management with verified operations.

- Supervised processes that survive restarts
- SHA-256-verified downloads and backups  
- Transactional world restores
- No browser required

Built for SSH administrators. Not for beginners.

[Install](install.md) · [Quick Start](quickstart.md) · [GitHub](link)
```

**Variant 2 (Scenario-Based):**
```markdown
# mc-server-tui

You SSH into Linux servers. You prepare locally on macOS. You want backups you can verify.

This is a terminal-first Minecraft server manager for that workflow.

- Resident supervision (daemon restarts ≠ killed servers)
- Evidence-based operations (SHA-256 locks, diffs, audit trails)
- Transactional backups (manifests, verification, rollback journals)

[See if this is for you →](positioning.md)
```

### "Getting Started" Introduction

```markdown
## Before You Begin

mc-server-tui is designed for **technical server administrators** who:
- Are comfortable with terminals and SSH
- Understand Java versions, ports, and memory allocation
- Want operational transparency over automated convenience
- Manage one or more servers and need reliable backups

If you're setting up your first Minecraft server and want step-by-step tutorials, this tool will be frustrating. Start with a web panel like MCSManager and come back when you want terminal workflow.

If you're an experienced admin who's tired of browser tabs or maintaining homegrown scripts, keep reading.

### What You'll Need

- macOS 13+ (Apple Silicon) or Linux (x86_64/arm64)
- Java 21 (for MC 1.21.x) or Java 25+ (for 26.x)
- Terminal at least 80×24
- Basic SSH and sysadmin knowledge

### What You'll Learn

This guide covers:
1. Installing the alpha release
2. Creating your first Paper server
3. Understanding the supervision model
4. Taking and verifying backups
5. Recovering from failures

Let's begin.
```

---

**Document status:** Ready for use. Update after significant positioning changes.  
**Channels covered:** Website, social media, email, Product Hunt, conference talks, documentation.  
**Maintainer:** Marketing/community lead with product alignment.
