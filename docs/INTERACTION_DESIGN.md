# TUI interaction and guidance contract

> Version 0.1 · 2026-10-18 · guided create, command discovery, transfer control, and adapter-report entry points

## Reference direction

The interaction layer borrows patterns, not visual assets, from mature terminal tools:

- **Codex / Claude Code:** one strong primary surface, persistent bottom guidance, compact status vocabulary, explicit approval for destructive actions, and a clear separation between navigation, current context, and next action.
- **DeepSeek Harness command surface:** grouped slash-command discovery, local fuzzy matching, action/description pairs, empty state that teaches the next move, and direct dispatch instead of pretending a command is a normal message.
- **GLM-style layered configuration:** safe defaults first, recommended choices before exact values, advanced controls separated from the main path, and every source/parameter visibly attributed instead of silently inferred.

The design rule is: a first-time user must be able to complete the normal path without knowing Paper API fields, Java executable locations, raw ports, or daemon internals. An advanced user can still reach exact values and durable task details.


## 默认会话 shell（Claude Code 骨架）

主界面采用单会话结构，而不是让用户在多屏之间找回上下文：

1. **会话记录**：用户命令、系统结果、错误和任务状态按时间顺序追加，可用 PgUp/PgDn 或鼠标滚轮回看；`/help`、`/status`、`/tasks`、`/instances`、`/theme` 这类列表输出会从第一行开始展示，`/logs` 则保持尾部日志可见。
2. **底部 composer**：一个输入框同时承担 slash 命令、参数补全和当前实例控制台命令。输入 `/` 时在输入框上方弹出命令补全；Tab 填入，↑/↓ 选择或翻阅历史。对尚未输入完整名称的 slash 命令，Enter 只填入高亮完整命令，用户再次按 Enter 才执行；完整命令仍可直接执行。composer 支持 ←/→、Home/End、Delete、Ctrl-A/E 的光标编辑和长行水平滚动，长命令不需要只能从末尾退格。多行粘贴合并为单行待提交内容，用户检查后再按 Enter。
3. **输入框下方状态坞**：持续显示选中实例、版本/端口、配置内存、运行状态、CPU/RSS、最多两个活动任务的阶段和字节进度条。它不依赖用户打开任务详情或实例详情才能看见。
4. **结构化流程仍然保留**：`/create` 打开版本/Java/EULA 向导，`/open` 打开完整日志与交互式控制台，`/backups` 打开世界备份页。日常的 `/status`、`/tasks`、`/logs`、`/cancel` 都在会话内完成，不切页；只有设置、完整控制台和破坏性恢复才进入下一层。纯文本输入不会伪装成结构化选择；破坏性操作仍然需要显式确认。
5. **上下文快捷动作**：状态坞第五行根据状态显示 `/start` 或 `/stop`、`/open`、`/logs`、`/backups`、`/tasks`，有可取消下载时追加 `/cancel`。用户不需要记住整套命令表。
6. **经典双面板**：`/classic` 或实际界面中的 `t` 可切回实例/任务双面板；`t` 或命令面板切回会话。会话记录在切换期间保留。

鼠标是可选增强：滚轮滚动列表/会话，单击选中行，双击打开实例或任务，点击 EULA 行切换勾选。

## 主题

七套共享结构的低对比主题：

- **Claude**：近黑背景，暖珊瑚色 accent，绿色/琥珀色/红色只用于状态。
- **DeepSeek**：深海蓝背景，品牌蓝 accent，冷色调状态色。
- **Codex**：近黑中性背景，OpenAI 绿 accent，终端感较强。
- **Kimi**：近黑紫背景，柔和紫罗兰 accent。
- **PCL2**：近黑青蓝背景，天空蓝 accent，偏 Minecraft 启动器的活泼感但保持低饱和。
- **Mono**：纯黑背景，灰阶文字，仅错误保留红色。
- **Ocean**：近黑偏蓝背景，低饱和青色 accent。

主题用 `F2` 顺序循环、`/theme <name>` 指定、`/theme` 列出全部主题、命令面板切换，或由 `MC_SERVER_TUI_THEME` 覆盖。选择写入平台配置目录的 `theme` 文件，下次启动恢复。边框、keycap 和 footer 使用中性灰；彩色仅保留给选中标记、输入提示、活动 spinner、进度条和像素吉祥物，避免整屏高饱和。

### 像素吉祥物

状态坞左侧固定一个 7×4 像素史蒂夫头像（再加一行状态符号，共 5 行）：

- 保持 Minecraft 的经典配色：棕色头发、肤色、白色眼白与蓝色瞳孔、棕色嘴；
- 有活动任务时眼睛睁开，头顶 `✦/✧/·` 依次闪烁；
- 空闲时眼睛闭合成深色眼睑，并显示 `z/Z` 打盹；
- 每约 3.8 秒眨眼一次；
- 主题 accent 只用于头顶闪光或打盹符号，避免把史蒂夫染成七种奇怪的颜色；
- 它在 80×24 下占用 11 列，信息列仍保持实例、任务、资源、主题四类信息；40×12 紧凑布局隐藏吉祥物，把空间留给 composer 和状态文本。

## Visual hierarchy

Every primary screen follows the same scan order:

1. **Context line:** product mark, current area, and instance/task identity.
2. **State or plan:** the current outcome, recommendation, or impact.
3. **Primary content:** version list, Java list, logs, backup rows, or adapter issues.
4. **Guidance footer:** current status, 3–5 contextual actions, and the universal escape path.

Color is never the only carrier of state. Selected rows use a cursor and background; states use words and symbols; focus uses a leading marker and border; danger uses the action name plus a second explicit confirmation. The 80×24 layout is the reference, while compact 40×12 layouts reduce secondary copy and keep the primary action reachable.

## Slash command palette

- In the session shell, typing `/` starts inline command completion. `Ctrl-K` opens the full command palette from any screen; on classic non-text screens `/` also opens the palette.
- The query matches Chinese titles, English aliases, descriptions, and command names with subsequence matching. Prefix matches rank first.
- Commands are grouped by intent: 创建、实例、世界与备份、任务、导航、应用.
- Each row shows the human title, shortcut, and a short explanation of the effect.
- Empty query preserves curated intent order instead of alphabetically scattering the most important command.
- Enter executes, Esc closes, Up/Down selects. Executing a command closes the palette before dispatching the shared `Action`.
- The palette never bypasses the daemon API or writes instance files directly.

Current contextual entries include new Paper instance, import modpack, refresh, panel switch, start, graceful stop, controlled restart, backup, open backup manager, verify, restore preview, load more, cancel download, back, and quit.

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

## Server lifecycle

`/start` and `/stop` remain the primary lifecycle commands. `/restart` (or
`R` on the instance detail) performs a controlled replacement:

1. It requires an instance whose recorded Java process is verified and whose
   observed state is starting or running.
2. It sends the Minecraft `stop` command and waits for the verified process to
   exit. It never escalates to a signal or force-kill.
3. It starts a new process only after the old process reaches `stopped`.
4. Start, stop, restart, and stopped-world reservations share one per-instance
   coordination lock, so another lifecycle or world operation cannot enter the
   stopped window. Detail and log reads remain available while shutdown is in
   progress.
5. A timeout, identity mismatch, failed exit, or unverified port leaves the
   instance inspectable and reports the reason; the new process is not started.

The CLI exposes the same typed API as `mc cli instance start <id>`,
`mc cli instance stop <id>`, and `mc cli instance restart <id>`.

## Server properties editor

`/config` (alias `/properties`) opens the selected instance's
`server/server.properties`. The editor exposes a deliberately limited first
set: MOTD, max players, difficulty, and view distance.

- It reads a snapshot with a SHA-256 revision and presents the four values as
  fields with validation limits and inline explanation.
- `Enter` moves to a diff review. Only changed fields are shown, together with
  the guarantee that unknown keys, comments, and unrelated lines remain intact.
- Another `Enter` atomically replaces the file only if the revision still
  matches. An external edit produces a conflict state, preserves the draft, and
  requires `F5` to reload or an explicit return.
- The save result explains that a running server keeps the old values until its
  next start. Saving does not restart the server; `/restart` or `R` is the
  explicit follow-up when the user wants the new values applied immediately.
- Decoding warnings for unsupported values and unsafe multi-line escapes are
  visible. Raw editing and the rest of the property catalog remain follow-up
  work.

## Modpack import entry

“导入整合包” is a first-class palette entry:

- scans the current directory, Downloads, and Desktop for `.mrpack` files;
- reads at most 4 MiB of `modrinth.index.json`;
- checks the ZIP entry count and rejects unsafe/absolute/`..` paths;
- reads `dependencies.minecraft` and exactly one supported loader dependency;
- classifies files into install / review / exclude using `env.server` and conservative directory rules;
- verifies that install-required rows have an HTTPS download and a syntactically valid sha1/sha512 hash;
- reports actual download hosts, unknown loader information, duplicate paths, and server override counts;
- analysis never extracts files, downloads Mods, or creates an instance.

The report uses a single readiness vocabulary: `可以导入 / 需要确认 / 禁止导入`. A blocked report preserves the evidence and tells the user how to fix the package. After review, `/import <path> --apply` holds the stopped-instance lease, stages and verifies eligible mods/plugins/config, then adds files without overwriting conflicts. First-start verification and automatic rollback after import remain follow-up work.

## Download-source policy boundary

The normal create review currently exposes the exact source as `Paper 官方 CDN` and the metadata host. It does not offer a decorative mirror selector without a working transport. Mirrors must remain a separate backend contract:

- only the official metadata may define version, size, and SHA-256;
- a mirror may only change the byte transport path;
- every candidate must pass the same exact SHA-256 and size checks;
- failed candidates must be quarantined and explained;
- the UI must show actual host and fallback/retry state.

Custom HTTPS mirrors and audited mirror catalogs therefore remain a follow-up implementation item rather than a fake UI option. The long-term policy is defined in `MIRROR_ACCELERATION_DESIGN.md`.
