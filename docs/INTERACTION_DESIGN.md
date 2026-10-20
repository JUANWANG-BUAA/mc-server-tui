# TUI interaction and guidance contract

> v0.2.1 · 2026-10-20 · shipped interaction contract

## Reference direction

The interaction layer borrows patterns, not visual assets, from mature terminal tools:

- **Codex / Claude Code:** one strong primary surface, persistent bottom guidance, compact status vocabulary, explicit approval for destructive actions, and a clear separation between navigation, current context, and next action.
- **DeepSeek Harness command surface:** grouped slash-command discovery, local fuzzy matching, action/description pairs, empty state that teaches the next move, and direct dispatch instead of pretending a command is a normal message.
- **GLM-style layered configuration:** safe defaults first, recommended choices before exact values, advanced controls separated from the main path, and every source/parameter visibly attributed instead of silently inferred.

The design rule is: a first-time user must be able to complete the normal path without knowing Paper API fields, Java executable locations, raw ports, or daemon internals. An advanced user can still reach exact values and durable task details.


## v0.2.1：实例优先的会话工作区

本节描述 v0.2.1 的交互行为。

1. **固定实例摘要**：顶部依次显示当前实例、运行状态、版本/端口、下一步动作和资源。状态只能来自相同实例 ID 的详情或总览快照；连接失败显示“上次状态”，管理连接与 Minecraft 就绪分开表达。
2. **一个主要工作区**：会话记录保留命令、结果与日志。后台任务只用两行显示阶段、真实传输进度、所属实例及对应 `/task <编号>`；没有字节总量的操作不显示百分比。完成结果进入会话，技术事件在任务详情。
3. **目标明确的输入框**：slash 输入显示“管理命令”；普通文本和 `/console` 显示控制台目标。草稿绑定开始输入时的实例，切服后提交会被阻止且内容保留；切回原实例或 Esc 清空重写。历史命令再次取出时绑定当前实例。尚未输入完整的命令按 Enter 只补全，再次 Enter 才执行；多行粘贴只合并为待检查文本。
4. **稳定导航**：`/open`、`/backups`、`/config` 和 Ctrl-K 进入相同业务页面。会话与经典视图保留；输入框底部显示管理连接或当前操作反馈，页脚只保留发送、补全、操作面板和帮助。
5. **按状态引导**：未创建时 `/create`；安装完成后 `/start`；启动中看日志；运行后 `/connect`；失败时先看原因。就绪提示不代表已验证外网可加入。
6. **渐进披露**：运行设置、实例摘要、备份技术详情统一用 F3 或 Ctrl-K 中的展开/收起入口。日志页默认收起详细资源和进程信息，将空间留给日志。

鼠标仍为可选增强；完整命令、快捷键和经典视图继续支持。

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

当前源码只在没有实例、没有任务、且空间足够的欢迎页显示像素史蒂夫。
运行、日志、下载与恢复场景优先展示内容；主题仍共用布局，保留七套配色。
旧版状态坞的固定头像与重复进度布局属于 v0.2.0 历史设计。

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
   - Minecraft version is a list loaded from Paper metadata. Matching friends’ client versions is explained above the list; “latest release” is informational, not a recommendation.
   - RC and pre-release identifiers are filtered out of the normal flow.
   - The exact build and SHA-256 remain locked by the daemon at submit time.
2. **运行设置摘要（F3 展开 Java、内存与端口）**
   - The default view summarizes Java, memory and port; Enter accepts these settings. F3 expands the editor, and Tab also enters it. Collapsed settings cannot be changed by hidden arrow-key actions.
   - Java is selected from runtimes discovered by the core doctor probe.
   - Java is selected against the chosen Minecraft version; the matching major is preferred, otherwise the closest compatible higher runtime.
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

When startup exits with an error, the instance detail replaces the generic
failed-start message with one actionable diagnosis when the bounded current log
contains a supported signature:

- incompatible Java runtime;
- occupied configured port;
- unaccepted EULA;
- Java heap exhaustion;
- missing, corrupt, or incompatible `server.jar`.

Unknown failures still direct the user to inspect the instance log. Diagnosis
describes evidence found in logs; it does not claim that the listed fix has
already been applied.

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

## Backup and restore presentation

Backup lists and details use local dates with an explicit offset in the detail.
The default view leads with the restore time, stopped-at-creation consistency,
size, verification and impact preview. It does not imply the server is currently stopped.
UUIDs and journal/token evidence are secondary information behind F3.
The restore preview explains replacements, additions, removals, retained rollback
copies and that progress after the backup is not merged. Explicit RESTORE confirmation
is unchanged. Completion asks the operator to start and verify the world in game.
