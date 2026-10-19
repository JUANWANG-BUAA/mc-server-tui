# Documentation

Start with the [README](../README.md) for installation and the quick tour. The
documents below are the implementation contracts behind the current alpha.

## Product and architecture

| Document | What it covers |
| --- | --- |
| [Product design](../PRODUCT_DESIGN.md) | product scope, P0/P1/P2 features, release gates |
| [Base implementation plan](../BASE_IMPLEMENTATION_PLAN.md) | original vertical-slice decisions and milestones |
| [Base readiness review](../BASE_READINESS_REVIEW.md) | risks and prerequisites identified before implementation |
| [Interaction design](INTERACTION_DESIGN.md) | shell/overlay/drill-in hierarchy, themes, mascot, copy rules |
| [Implementation gaps](IMPLEMENTATION_GAPS.md) | documented-vs-implemented audit and release blockers |

## Runtime contracts

| Document | What it covers |
| --- | --- |
| [Supervision](SUPERVISION.md) | runner ownership, PID identity, lifecycle, daemon reattachment |
| [Instance schema](INSTANCE_SCHEMA.md) | `instance.toml`, `lock.json`, SQLite index, staging/recovery |
| [Async concurrency](ASYNC_CONCURRENCY.md) | request path, bounded tasks, progress, cancellation |
| [Backup and restore](BACKUP_RESTORE_DESIGN.md) | stopped-world backup, verification, restore transaction |
| [Modpack analysis](MODPACK_IMPORT.md) | read-only `.mrpack` adapter boundary and next phase |

## TUI

| Document | What it covers |
| --- | --- |
| [Interaction design](INTERACTION_DESIGN.md) | user-facing interaction contract |
| [TUI testing](TUI_TESTING.md) | renderer/PTY tests, screenshots, visual review history |
| [FAQ](FAQ.md) | common setup, service, backup, and alpha questions |

## Distribution and releases

| Document | What it covers |
| --- | --- |
| [Distribution design](DISTRIBUTION.md) | macOS/Linux packaging, services, signing, hot update |
| [Release process](RELEASING.md) | tag → CI → draft Release → review checklist |
| [Implementation gaps](IMPLEMENTATION_GAPS.md) | what is still missing before a stable release |

## Project and releases

| Document | What it covers |
| --- | --- |
| [Contributing](../CONTRIBUTING.md) | local checks, code guidelines, PR expectations |
| [Security](../SECURITY.md) | supported versions and private vulnerability reporting |
| [Code of conduct](../CODE_OF_CONDUCT.md) | participation expectations |
| [Changelog](../CHANGELOG.md) | user-facing changes and alpha limitations |
| [0.1.0 release notes](releases/0.1.0.md) | first public unsigned alpha |

## Upstream design

| Document | What it covers |
| --- | --- |
| [Upstream download design](../UPSTREAM_DOWNLOAD_DESIGN.md) | Paper/Vanilla/Java sources, download and lock contract |
| [Mirror acceleration design](../MIRROR_ACCELERATION_DESIGN.md) | source model, failover, candidate policy |
