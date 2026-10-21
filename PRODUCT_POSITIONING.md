# Product Positioning: mc-server-tui

> Version 1.0 · 2026-10-20 · Strategic repositioning for clarity and focus

## Executive Summary

`mc-server-tui` is a **terminal-first Minecraft server manager** designed for server administrators who manage infrastructure over SSH and need verifiable, auditable operations. It is **not** a low-barrier tool for first-time server operators. It is positioned for the gap between web panels (too heavyweight, browser-dependent) and raw `screen` sessions (no supervision, no verification).

## Target Users

### Primary: Technical Server Administrators

**Who they are:**
- Manage 1–5 Minecraft servers via SSH
- Comfortable with terminal environments and basic system administration
- Run macOS locally for server preparation, deploy to Linux production hosts
- Value operational transparency over automated convenience

**What they need:**
- Verifiable backups they can prove are correct
- Process supervision that survives daemon restarts
- Terminal workflow without browser dependencies
- Audit trails for configuration changes and world modifications

**What they reject:**
- Web panels with their own databases and attack surfaces
- Tools that hide the actual Java process behind layers of abstraction
- Solutions that require learning a web UI for tasks better done in a shell
- Backup systems where "success" means a file exists, not that it's verified

### Secondary: Multi-Server Operators

Small hosting operations or friend groups managing multiple instances who need:
- Per-instance isolation with shared download cache
- Durable task queues that survive crashes
- Console access without multiplexer juggling
- Resource accounting across instances

### Non-Target: First-Time Server Operators

We **do not** target users who:
- Need step-by-step tutorials for every operation
- Expect GUI-level automation
- Have never opened a terminal or SSH'd into a host
- Want one-click everything with no understanding of what happens

Accepting a smaller addressable market in exchange for a focused, coherent product.

## Core Value Proposition

### The Three Pillars: Reliable · Auditable · Recoverable

#### 1. Reliable
- **Resident supervision:** Each instance runs under a supervisor with process identity markers. TUI/daemon restarts do not kill Java.
- **Durable tasks:** Downloads, backups, and imports survive daemon crashes and resume from checkpoints.
- **Idempotent operations:** Start/stop/restart are safe to retry; the system knows what state it's in.

#### 2. Auditable
- **Locked builds:** Paper servers lock to exact build numbers, file sizes, and SHA-256 hashes.
- **Change tracking:** Configuration edits show diffs before applying. External modifications are detected.
- **Evidence-based logs:** Startup failures cite specific log lines, not vague summaries.

#### 3. Recoverable
- **Transactional backups:** SHA-256 manifests, paged verification, restore previews, rollback journals.
- **Failure transparency:** Crashed tasks leave diagnostic trails, not silent corruption.
- **State reconstruction:** The daemon can reattach to running servers after restart using process identity.

## Core Scenarios

### Scenario 1: SSH Multi-Host Management
**User:** Manages three servers across two Linux hosts from a MacBook.  
**Pain:** Web panels require port forwarding or public exposure. SSH + screen loses context switching terminals.  
**Solution:** Prepare servers locally with verified downloads. Deploy to Linux. Manage all instances through one TUI session over SSH.

### Scenario 2: Verifiable Production Backups
**User:** Runs a semi-public server with 20 regular players.  
**Pain:** Existing backup scripts check file existence, not integrity. Restore is manual and risky.  
**Solution:** Stopped-world backups with SHA-256 manifests. Paged verification. Restore plan shows exactly what will change, requires explicit confirmation, logs every step.

### Scenario 3: Supervised Long-Running Instances
**User:** Keeps multiple test and production servers running continuously.  
**Pain:** Screen/tmux sessions die on disconnect. Wrapper scripts have no visibility. Can't tell if a server is really running or just has a stale PID file.  
**Solution:** Resident runners with process identity markers. Daemon reattachment. Console access without multiplexer hunting. Status is provable, not guessed.

## Competitive Positioning

We acknowledge the TUI market is small. Our differentiation is **not** "we have a TUI." It is:

### vs. Web Panels (MCSManager, PufferPanel, Crafty)

**Why choose us:**
- No browser, daemon, or database dependency for local/SSH use
- Process supervision without containerization overhead
- Backups you can verify, not just files that exist
- Terminal-native workflow matches sysadmin habits

**Why choose them:**
- Multi-user access control and web-based delegation
- Browser UI for users uncomfortable with terminals
- Multi-node orchestration across geographic regions

**Market reality:** Web panels own the multi-user, web-UI-preferred segment. We serve the "SSH-first, single-operator" niche they over-serve.

### vs. Configuration Managers (mcman, mctl)

**Why choose us:**
- Long-running supervision, not just install-and-exit
- Durable task queues with progress tracking
- Backup verification and transactional restore
- TUI session for interactive work, not CLI-only

**Why choose them:**
- Declarative infrastructure-as-code workflows
- Lighter weight if you only need provisioning
- More server types if you run exotic configurations

**Market reality:** `mcman` explicitly does not run servers. `mctl` is close but has minimal public adoption. We compete on operational maturity: backups, recovery, long-term supervision.

### vs. Raw screen/systemd

**Why choose us:**
- Process identity and reattachment
- Verified downloads and backup checksums
- Structured console access and log history
- Failure diagnosis with evidence

**Why choose them:**
- Zero dependencies
- Full control, no opinions
- Works everywhere, including air-gapped hosts

**Market reality:** Experienced admins use systemd + scripts for good reason. We serve those who want more structure without giving up the terminal.

## What We Are Not

- **Not a web panel.** No browser requirement, no user accounts, no RBAC. If you need web-based delegation, use MCSManager.
- **Not a one-click installer.** We require Java knowledge, SSH comfort, and understanding of server concepts. If you've never opened a terminal, this is not your first tool.
- **Not infrastructure-as-code.** We provide interactive TUI and CLI, not declarative config management. For GitOps workflows, consider mcman.
- **Not zero-configuration.** You must understand ports, Java versions, memory allocation, and EULA acceptance. We verify and guide, but do not hide these concepts.

## Messaging Framework

### Elevator Pitch (30 seconds)
"mc-server-tui is a terminal manager for Minecraft servers. It's built for admins who SSH into Linux hosts and need verifiable backups, supervised processes, and audit trails—without installing a web panel. If you value operational transparency over automated convenience, it's designed for your workflow."

### One-Sentence Description
A terminal-first Minecraft server manager with verified downloads, supervised processes, and transactional backups for SSH-based server administration.

### Tagline Options
- **Reliable · Auditable · Recoverable server management**
- **Terminal-native Minecraft server supervision**
- **SSH-first server management with verifiable operations**

### Key Talking Points

**For technical credibility:**
- Resident runners with process identity markers
- SHA-256-locked Paper builds and backup manifests
- Durable task queues that survive daemon restarts
- Rollback journals for every restore operation

**For user benefits:**
- Manage all instances from one SSH session
- Prove your backups are correct before disaster strikes
- No browser required, no separate admin panel
- Configuration changes show diffs before applying

**For positioning:**
- Built for the gap between web panels and raw screen sessions
- Terminal-first by design, not a web UI that happens to have a CLI
- Small market, focused product: SSH administrators, not first-time operators

## Social Media / Website Copy

### GitHub Description (160 chars)
A terminal-first Minecraft server manager for macOS/Linux with supervised processes, verified downloads, and transactional world backups.

### Twitter/X Bio (280 chars)
Manage Minecraft servers over SSH without web panels. Verified downloads, supervised processes, transactional backups. Built for admins who value operational transparency over automation. Terminal-native · macOS/Linux · MIT/Apache-2.0

### Website Hero Copy

**Headline:**  
Minecraft Server Management for SSH Administrators

**Subhead:**  
Terminal-native supervision, verified backups, and audit trails.  
No browser. No guessing. No silent failures.

**CTA Primary:** Install v0.2.1 (alpha)  
**CTA Secondary:** Read the docs

### Product Hunt Description

**Tagline:**  
Terminal-first Minecraft server manager with verified operations

**Description:**  
mc-server-tui is built for server administrators who manage Minecraft infrastructure over SSH. It provides:

✓ Supervised processes that survive daemon restarts  
✓ SHA-256-verified Paper builds and backup manifests  
✓ Transactional world backups with rollback journals  
✓ Terminal-native workflow without browser dependencies  

It's not for first-time operators. It's for admins who SSH into Linux hosts, prepare servers locally on macOS, and need backups they can prove are correct—not just files that exist.

Built with Rust. Dual-licensed MIT/Apache-2.0. Currently in alpha.

### Reddit r/admincraft Post Title
`[Tool] mc-server-tui: Terminal manager with verified backups and supervised processes (alpha)`

**Post Body:**
I've been building a terminal-first Minecraft server manager for SSH-based workflows. It's designed for the gap between web panels (too heavyweight) and raw screen sessions (no supervision).

**Core features:**
- Resident runners: TUI/daemon restarts don't kill Java
- Verified downloads: SHA-256-locked Paper builds
- Transactional backups: Manifests, verification, rollback journals
- Durable tasks: Downloads resume after daemon crashes

**Target users:**
If you manage servers over SSH, value operational transparency, and want backups you can verify—not just hope are correct—it's worth a look.

**Current status:**
Alpha (unsigned). macOS/Linux. Rust. MIT/Apache-2.0.

GitHub: [link]  
Docs: [link]

Open to feedback. Not claiming it's better than existing tools, just scratching a specific itch.

### Documentation Site Intro

**For Server Administrators**

If you manage Minecraft servers over SSH and want:
- Backups you can verify, not just files you hope are correct
- Process supervision that survives tool restarts
- Terminal workflow without browser dependencies
- Audit trails for configuration changes

Then `mc-server-tui` is designed for your workflow.

**Not a low-barrier tool.** You should be comfortable with terminals, SSH, Java versions, and server concepts. This is not your first server manager—it's the tool you graduate to when reliability and auditability matter more than automation.

## Competitive Comparison Table

| Capability | mc-server-tui | Web Panels | mcman | systemd + scripts |
|------------|---------------|------------|-------|-------------------|
| **Multi-user access control** | ✗ Single operator | ✓ Full RBAC | ✗ | ✗ |
| **Browser-free operation** | ✓ Terminal-native | ✗ Web required | ✓ CLI only | ✓ |
| **Long-running supervision** | ✓ Resident runners | ✓ | ✗ Install-only | ✓ systemd units |
| **Verified backups** | ✓ SHA-256 + journals | ~ File existence | ✗ | ~ Script-dependent |
| **Process reattachment** | ✓ Identity markers | ✗ | N/A | ~ PID files |
| **Durable task queues** | ✓ SQLite-backed | ✓ | ✗ | ✗ |
| **Configuration diffs** | ✓ Pre-apply review | ~ | ✗ | ✗ |
| **TUI + CLI** | ✓ Both | ~ CLI for web | ✓ CLI only | ✓ CLI only |
| **Learning curve** | Medium | Low-Medium | Medium | High |
| **Best for** | SSH admins | Multi-user teams | IaC workflows | Full DIY control |

**Read this table as:** "Why would I choose X over Y for my specific scenario?"

- Choose **mc-server-tui** if: You SSH into servers, want terminal workflow, need verified backups, value operational transparency.
- Choose **web panels** if: You need browser UI, multi-user delegation, or don't use terminals regularly.
- Choose **mcman** if: You want declarative config and only need provisioning, not long-running supervision.
- Choose **systemd + scripts** if: You have the expertise and want zero abstractions.

## Roadmap Alignment

This positioning commits to:

1. **Verifiability first.** Every download, backup, and configuration change must be auditable.
2. **Terminal-native UX.** No "web UI with a CLI bolted on." Shell-first design.
3. **SSH workflow focus.** macOS local prep → Linux deployment is a first-class path.
4. **Honest about scope.** We won't promise multi-user RBAC, web dashboards, or zero-learning-curve automation.

## Metrics That Matter

**Vanity metrics we ignore:**
- GitHub stars (correlation ≠ adoption)
- Page views on landing site

**Real metrics we track:**
- First-server success rate within 10 minutes
- Backup verification pass rate
- Daemon crash recovery without world corruption
- Issue resolution time for "can't restore world" reports

**Community signals we monitor:**
- Repeat usage after 30 days (not just install-and-abandon)
- Issues opened for "how do I…" vs. "this broke my world"
- Third-party documentation or scripts built on top

## Evolution Path

### Phase 1 (Current Alpha): Prove Core Reliability
- Focus: One complete Paper server lifecycle
- Gate: Real-world backup/restore drills, no silent corruption
- Target: 10 active alpha testers willing to report failures

### Phase 2 (Stable Beta): Broaden Platform Support
- Focus: Signed binaries, Homebrew tap, Linux packages
- Gate: Three clean install → backup → restore cycles on each platform
- Target: 100 users, documented failure modes

### Phase 3 (v1.0): Feature Completeness
- Focus: Vanilla, Fabric mature; online backups; TPS/MSPT monitoring
- Gate: One month of production use by maintainer with no world loss
- Target: Recommend to r/admincraft without "alpha" caveat

### Phase 4 (v2.x): Ecosystem
- Focus: Multi-host SSH, Forge/NeoForge, plugin dependency analysis
- Gate: User-contributed guides for common hosting scenarios
- Target: Known as "the SSH admin's server manager"

## FAQ for Positioning

**Q: Why not target first-time server operators? That's the biggest market.**  
A: Market size ≠ product-market fit. First-timers need tutorials, handholding, and safety rails we can't build well. Better to serve 1,000 SSH admins excellently than 10,000 beginners poorly.

**Q: Won't web panels always win because they're easier?**  
A: Yes, for users who want web panels. We serve the niche that finds browsers slower than terminals and wants operational transparency over convenience.

**Q: TUI market is tiny. Why build this?**  
A: Because the alternative is raw systemd + scripts, which works but has no standardization. There's room for "better than scripts, lighter than panels."

**Q: What if someone builds this into a web panel?**  
A: Fine. We're open source. If the supervision model and backup verification help other projects, that's success. Our differentiation is the integrated terminal workflow, not a patent on process supervision.

---

**Document status:** Strategic positioning v1.0, approved for external communication.  
**Next review:** After v0.3.0 beta release or six months, whichever comes first.  
**Maintainer:** Product lead with input from early adopters.
