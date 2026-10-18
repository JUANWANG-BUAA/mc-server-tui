# TUI interaction and guidance contract

> Version 0.1 · 2026-10-18 · guided create, command discovery, transfer control, and adapter-report entry points

## Reference direction

The interaction layer borrows patterns, not visual assets, from mature terminal tools:

- **Codex / Claude Code:** one strong primary surface, persistent bottom guidance, compact status vocabulary, explicit approval for destructive actions, and a clear separation between navigation, current context, and next action.
- **DeepSeek Harness command surface:** grouped slash-command discovery, local fuzzy matching, action/description pairs, empty state that teaches the next move, and direct dispatch instead of pretending a command is a normal message.
- **GLM-style layered configuration:** safe defaults first, recommended choices before exact values, advanced controls separated from the main path, and every source/parameter visibly attributed instead of silently inferred.

The design rule is: a first-time user must be able to complete the normal path without knowing Paper API fields, Java executable locations, raw ports, or daemon internals. An advanced user can still reach exact values and durable task details.

## Visual hierarchy

Every primary screen follows the same scan order:

1. **Context line:** product mark, current area, and instance/task identity.
2. **State or plan:** the current outcome, recommendation, or impact.
3. **Primary content:** version list, Java list, logs, backup rows, or adapter issues.
4. **Guidance footer:** current status, 3–5 contextual actions, and the universal escape path.

Color is never the only carrier of state. Selected rows use a cursor and background; states use words and symbols; focus uses a leading marker and border; danger uses the action name plus a second explicit confirmation. The 80×24 layout is the reference, while compact 40×12 layouts reduce secondary copy and keep the primary action reachable.

## Slash command palette

- `/` opens the palette on non-text screens; `Ctrl-K` opens it from text-editing screens.
- The query matches Chinese titles, English aliases, descriptions, and command names with subsequence matching. Prefix matches rank first.
- Commands are grouped by intent: 创建、实例、世界与备份、任务、导航、应用.
- Each row shows the human title, shortcut, and a short explanation of the effect.
- Empty query preserves curated intent order instead of alphabetically scattering the most important command.
- Enter executes, Esc closes, Up/Down selects. Executing a command closes the palette before dispatching the shared `Action`.
- The palette never bypasses the daemon API or writes instance files directly.

Current contextual entries include new Paper instance, import modpack, refresh, panel switch, start, graceful stop, backup, open backup manager, verify, restore preview, load more, cancel download, back, and quit.

## Guided create flow

The creation flow has three user-facing steps:

1. **实例与游戏版本**
   - Name is a text field.
   - Minecraft version is a list loaded from Paper metadata.
   - RC and pre-release identifiers are filtered out of the normal flow.
   - The exact build and SHA-256 remain locked by the daemon at submit time.
2. **Java、内存与端口**
   - Java is selected from runtimes discovered by the core doctor probe.
   - Java 21 is preferred when available; otherwise the closest compatible higher runtime is recommended.
   - Exact executable paths are shown as secondary evidence, not required typing.
   - Memory uses presets: 0.5–1 GiB, 1–2 GiB, 2–4 GiB, 4–8 GiB.
   - The port is pre-filled with a locally available value and remains editable for advanced cases.
3. **安装计划与 EULA**
   - Shows the selected Paper version, Java distribution/version/architecture, memory, port, download source, and hash policy.
   - EULA acceptance is explicit and never pre-checked.
   - Enter creates a durable background task; it does not imply that installation is already done.

If version or Java discovery fails, the wizard states the failed dependency and a concrete next action. It does not silently accept hand-typed values as if they were discovered facts.

## Transfer progress and cancellation

Download tasks persist byte-level progress:

- downloaded bytes and total bytes;
- current throughput estimate;
- actual source label;
- task phase, task ID, and source metadata host.

The task detail renderer draws a bounded progress bar, percentage, transferred/total size, speed, and source. Task detail polls faster while the create task is queued or running.

Cancellation is cooperative:

1. The first `x` arms cancellation and explains that partial bytes will be retained.
2. The second `x` sends `cancel_task` to the daemon.
3. The task manager sets the shared cancellation flag.
4. The downloader checks it between network chunks and returns `TASK_CANCELLED`.
5. The task reaches the distinct `cancelled` terminal state; its partial file and resume metadata remain available for a later identical artifact request.

## Modpack import entry

“导入整合包” is a first-class palette entry. The current first phase is deliberately read-only:

- scans the current directory, Downloads, and Desktop for `.mrpack` files;
- reads at most 4 MiB of `modrinth.index.json`;
- checks the ZIP entry count and rejects unsafe/absolute/`..` paths;
- reads `dependencies.minecraft` and exactly one supported loader dependency;
- classifies files into install / review / exclude using `env.server` and conservative directory rules;
- verifies that install-required rows have an HTTPS download and a syntactically valid sha1/sha512 hash;
- reports actual download hosts, unknown loader information, duplicate paths, and server override counts;
- never extracts files, downloads Mods, or creates an instance in this phase.

The report uses a single readiness vocabulary: `可以导入 / 需要确认 / 禁止导入`. A blocked report preserves the evidence and tells the user how to fix the package. The next phase will add dependency materialization, loader installation, file extraction, and first-start verification; those steps must consume this same report and remain visible before anything is written.

## Download-source policy boundary

The normal create review currently exposes the exact source as `Paper 官方 CDN` and the metadata host. It does not offer a decorative mirror selector without a working transport. Mirrors must remain a separate backend contract:

- only the official metadata may define version, size, and SHA-256;
- a mirror may only change the byte transport path;
- every candidate must pass the same exact SHA-256 and size checks;
- failed candidates must be quarantined and explained;
- the UI must show actual host and fallback/retry state.

Custom HTTPS mirrors and audited mirror catalogs therefore remain a follow-up implementation item rather than a fake UI option. The long-term policy is defined in `MIRROR_ACCELERATION_DESIGN.md`.
