# Documentation

Start with the [README](../README.md) for installation and the quick tour. The
current release is v0.2.1. User guides describe shipped behavior; design and
review documents may contain explicitly marked future work. The dated release
notes and testing history preserve the evidence available at the time.

## For users

| Document | What it covers |
| --- | --- |
| [User guide](USER_GUIDE.md) | Complete English tutorial: install, create, operate, back up, troubleshoot, and understand alpha limits |
| [中文使用手册](USER_GUIDE.zh-CN.md) | 完整中文教程：安装、建服、日常操作、备份恢复、排错与免责声明 |
| [Online documentation](https://juanwang-buaa.github.io/mc-server-tui/docs/) | Native bilingual site with chapter navigation and mobile layout |

## Product and architecture

| Document | What it covers |
| --- | --- |
| [Product design](../PRODUCT_DESIGN.md) | product scope, P0/P1/P2 features, release gates |
| [Productization review (中文)](PRODUCTIZATION_REVIEW.md) | Apple-inspired product critique, scope freeze, complete user journeys, proposed release vetoes and acceptance gates |
| [Base implementation plan](../BASE_IMPLEMENTATION_PLAN.md) | original vertical-slice decisions and milestones |
| [Base readiness review](../BASE_READINESS_REVIEW.md) | risks and prerequisites identified before implementation |
| [Interaction design](INTERACTION_DESIGN.md) | shell/overlay/drill-in hierarchy, themes, mascot, copy rules |
| [Implementation gaps](IMPLEMENTATION_GAPS.md) | documented-vs-implemented audit and release blockers |
| [Engineering optimization guide (中文)](ENGINEERING_OPTIMIZATION_GUIDE.md) | evidence-based refactoring, module ownership, prioritized work packages, validation and metrics |

## Runtime contracts

| Document | What it covers |
| --- | --- |
| [Supervision](SUPERVISION.md) | runner ownership, PID identity, lifecycle, daemon reattachment |
| [Instance schema](INSTANCE_SCHEMA.md) | `instance.toml`, `lock.json`, SQLite index, staging/recovery |
| [Async concurrency](ASYNC_CONCURRENCY.md) | request path, bounded tasks, progress, cancellation |
| [Backup and restore](BACKUP_RESTORE_DESIGN.md) | stopped-world backup, verification, restore transaction |
| [Modpack import](MODPACK_IMPORT.md) | `.mrpack` analysis, verified materialization, staging, and conflict rules |

## TUI

| Document | What it covers |
| --- | --- |
| [Interaction design](INTERACTION_DESIGN.md) | user-facing interaction contract |
| [UI and interaction review (中文)](UI_UX_REVIEW.md) | visual hierarchy, input and focus defects, async feedback, key-screen redesign briefs and validation gates |
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
| [0.2.1 release notes](releases/0.2.1.md) | current unsigned alpha |
| [0.2.1 acceptance](acceptance/0.2.1.md) | real Paper drill, automated checks and limits |
| [0.2.0 release notes](releases/0.2.0.md) | previous unsigned alpha |
| [0.1.0 release notes](releases/0.1.0.md) | first public unsigned alpha |
| [Website](WEBSITE.md) | static landing page, local preview, GitHub Pages deploy |

## Upstream design

| Document | What it covers |
| --- | --- |
| [Upstream download design](../UPSTREAM_DOWNLOAD_DESIGN.md) | Paper/Vanilla/Java sources, download and lock contract |
| [Mirror acceleration design](../MIRROR_ACCELERATION_DESIGN.md) | source model, failover, candidate policy |
