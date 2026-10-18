# TUI feedback and test loop

The TUI uses the daemon API and durable task event cursor; rendering and input handling stay separate from daemon I/O. This makes the UI testable without contacting Paper or waiting on real downloads.

## Module boundaries and visual behavior

Keep the TUI split along responsibilities so screen growth does not turn the runtime into a screen-sized module:

- `app.rs` owns the screen-level state and action transitions. Feature state and behavior that need their own lifecycle belong in `app/`, `instance_detail.rs`, `backup_manager.rs`, `task_detail.rs`, or `wizard.rs` with colocated tests.
- `runtime.rs` owns the async event loop, request scheduling, and result delivery. Key mapping is isolated in `runtime/input.rs`; daemon access is behind `ManagementSource` in `source.rs`.
- `view.rs` routes to the dashboard, wizard, task-detail, instance-detail, and world-backup renderers. Each screen renderer owns its layout and presentation, while `theme.rs` is the single palette and shared-style boundary.
- Renderers consume state and return no I/O effects. User actions update state; the runtime observes the resulting queries/commands and schedules bounded asynchronous work. Keep daemon protocol details out of view modules.

The shared theme uses 24-bit RGB colors for the page, surfaces, focus, status, and key hints. Crossterm honors `NO_COLOR`; the browser color-review command removes that variable only for the visual check. Status meaning remains available through text and symbols when color is unavailable.

Animation is presentation-only. The runtime advances its four-frame status symbol every 160 ms while `AppState::has_animated_work` reports queued/running tasks, a pending operation, wizard submission, or a visible server start/stop transition. The Tokio interval uses `MissedTickBehavior::Skip`, so a delayed frame is dropped instead of causing a catch-up redraw burst. The normal two-second refresh and terminal event stream remain independent of animation; all waits are selected asynchronously, so animation does not block input or daemon replies. When adding an animated state, add both a state predicate test and a PTY assertion that observes at least two distinct frames.

The `TestBackend` tests own deterministic layout and content checks. PTY tests own terminal byte-stream, input, responsiveness, and animation checks. Browser screenshots own human visual review of color, spacing, and hierarchy; screenshots alone do not prove that an animation advances.

## Automated checks

1. Keep screen state and wizard input separate from rendering and daemon I/O. State tests cover field validation, back navigation with retained values, explicit EULA acceptance, queued task success, retry after a submission error, monotonic event cursors, duplicate event replay, retaining the last good detail after a failed refresh, backup-page deduplication, verification failure retention, and exact restore confirmation.
2. Render dashboard, wizard, task detail, instance detail, backup catalog, and restore-preview states to an in-memory terminal backend. Cover 80×24 reference layouts and 40×12 compact layouts, including every wizard step, EULA control, task progress/event cursor, server state/address, lifecycle controls, backup rows, restore impacts, and the explicit confirmation control.
3. Run input and polling together against fake daemons that delay replies or emit ordered events/log chunks. Assert that navigation, wizard text input, console editing, backup confirmation, and Ctrl-C remain responsive while a request is pending; task detail resumes from its event sequence; instance detail resumes from its log byte offset; durable backup tasks are polled from queued to terminal results; and a failed request preserves the last good data.
4. Use a PTY smoke test for raw-mode entry, resize, key handling, and terminal restoration on normal exit and panic. Keep this separate from view-model tests because terminal ownership is process-level behavior.

## Visual review

For a release candidate, run the TUI in a real terminal at the reference sizes, drive the fake daemon through each task state, and capture terminal screenshots. Review hierarchy, truncation, contrast, progress indication, and resize behavior against the same fixtures used by the renderer tests. A screenshot is a review artifact; the deterministic cell snapshot remains the CI regression check.

The optional browser projection uses `ttyd` on localhost and a normal browser, so it can be used when host-window screenshots are unavailable. Start the protocol-v8 fake daemon and TUI from the repository root:

```sh
python3 -u scripts/tui_visual_fake_daemon.py --socket /tmp/mc-tui-visual.sock
```

In another terminal, build and project the TUI. The test command clears `NO_COLOR` only for this color review; the TUI itself continues to honor a user's `NO_COLOR` preference.

```sh
cargo build -p mc-tui
env -u NO_COLOR ttyd --interface 127.0.0.1 --port 7681 --writable --max-clients 1 --client-option fontSize=17 ./target/debug/mc-server-tui --socket /tmp/mc-tui-visual.sock
```

Open `http://127.0.0.1:7681`. With ttyd 1.7.7 at `fontSize=17`, an `830×500` browser viewport yields a measured terminal of exactly 80×24 cells; the older `820×520` viewport yielded only 79 columns and is not a valid 80-column visual reference. Capture the dashboard first:

```sh
npx playwright screenshot --channel chrome --wait-for-timeout 1800 --viewport-size 830,500 http://127.0.0.1:7681 output/playwright/theme-dashboard-80x24.png
```

Then press `Enter` to open instance detail, `l` to open the world-backup manager, and capture `output/playwright/theme-backup-manager-80x24.png`. Press `v` to verify the selected snapshot, then Enter to build the restore plan and capture `output/playwright/theme-backup-restore-plan-80x24.png`. The instance-detail PTY test separately performs `c`, types `RESTORE`, submits the durable restore task, and verifies the stored confirmation token. The PTY test also verifies that a running-task spinner emits at least two distinct frames; no screenshot can establish motion from a single frame.

The screen sequence is tested independently:

| Screen | In-memory renderer/state | PTY interaction |
| --- | --- | --- |
| Dashboard | Empty-state guidance, populated layout and grouped command palette at both sizes | `/`, fuzzy search, Enter dispatch and Ctrl-C while daemon replies are stalled |
| Guided create | Version/Java/memory pickers, review, explicit EULA and loading/error states at both sizes | `/new` path selection, Tab/arrow navigation, submit, byte progress and two-press cancellation |
| Task detail | Status, byte transfer bar, source, speed, events and cursor at both sizes | Events `1→2→3`, refresh cursor, close/reopen, progress polling and cancellation request |
| Modpack import | Picker, loading, blocked/review report and compact layout | Palette entry, local `.mrpack` selection and daemon analysis response (renderer/state coverage) |
| Instance detail | State, address, logs and controls at both sizes | Enter detail, start, incremental console log, console command, graceful stop, return and restore terminal |
| World and backups | Catalog, selected summary, plan impact, confirmation and restore result at both sizes | Enter detail, open manager, poll list task, verify selected backup, build restore plan, type RESTORE, poll restore task, return to detail |

Run `cargo build -p mc-tui && python3 scripts/tui_pty_smoke.py && python3 scripts/tui_task_detail_pty.py && python3 scripts/tui_guided_create_pty.py && python3 scripts/tui_instance_detail_pty.py`. These PTY checks validate the terminal stream and key path against fake daemons. The integrated create flow asserts the exact submitted version, Java executable, memory, port, EULA flag, transfer content, cancel request count, and alternate-screen cleanup.

## 历史真实终端验收记录（2026-10-17）

这条历史记录最初使用协议 v5 假 daemon，在 macOS Terminal 的独立 80×24 窗口运行真实 `mc-server-tui` 可执行文件；之后曾升级至协议 v6 并加入分页列表。当前假 daemon 和 PTY 回归已升级到协议 v8，包含恢复计划、恢复任务、取消请求和整合包静态分析接口。

复跑时先执行 `cargo build -p mc-tui`，然后在一个终端启动 `python3 -u scripts/tui_visual_fake_daemon.py --socket /tmp/mc-tui-visual.sock`，再在另一个终端运行 `./target/debug/mc-server-tui --socket /tmp/mc-tui-visual.sock`。结束 TUI 后用 Ctrl-C 停止假 daemon。

| 流程 | 实际验证 | 结果 |
| --- | --- | --- |
| 仪表盘 | 实例、后台任务、连接状态和服务地址 | 通过；真实终端绘制内容与假 daemon 响应一致 |
| 创建向导 | 输入 `VisualCheck`、`1.21.11`、`/usr/bin/java`；预览步骤显示资源和端口；Space 勾选 EULA 后提交 | 通过；返回仪表盘并显示新建任务排队 |
| 任务详情 | 打开固定任务，查看事件序列 1、2、3 和游标 | 通过；页面显示 3 条事件、游标 `#3` |
| 实例详情 | 查看安装信息、运行状态、地址和日志；启动后发送 `say visual`；执行优雅停止 | 通过；假 daemon 回读 `running`、控制台命令日志和 `stopped` 状态 |
| 退出清理 | 从 TUI 正常退出并检查进程；停止假 daemon，关闭独立测试窗口 | 通过；TUI 进程和假 daemon 均已退出，用户原有 Terminal 窗口未关闭 |

### 截图与视觉检查

真实终端的无障碍文本回读确认了上述屏幕内容和 80 列布局。像素截图没有通过验收：对独立窗口调用 CUA 截图和 `take_screenshot.py --window-id` 后，得到的 PNG 只有约 `198×222` 像素，终端画面发生明显形变；CUA 返回的窗口截图约 `186×206` 像素。它们无法用于判断字形、颜色对比和边框对齐，因此没有保存为项目截图或作为视觉通过证据。真实截图复核以及真实终端中的缩放/resize 视觉检查仍待宿主截图链路恢复后完成；40×12 紧凑布局目前由 `TestBackend` 渲染测试覆盖。

### 协议 v6 备份列表闭环（2026-10-17）

这一历史路径通过任务详情查看备份列表，已被下方的协议 v7 世界与备份管理器取代。

重新构建 `mc-server-tui`，由真实 TUI 进程连接协议 v6 PTY fake daemon，在 80×24 终端尺寸依次执行实例详情 `l`、返回任务面板、打开列表任务。已确认请求携带目标实例、`limit=3` 和空 cursor；详情页显示一条备份摘要的 ID 前缀、文件数 `12`、大小 `2.0 MiB`，并能返回仪表盘。随后继续验证启停、控制台命令、日志增量和 alternate screen 恢复。该回归覆盖真实终端事件与渲染字节流，不是像素截图验收。

### 协议 v7 直连备份恢复闭环（2026-10-17）

实例详情 PTY fake daemon 升级了一个完整的状态机，而不是直接返回最终数据：`list_world_backups`、`verify_world_backup`、`plan_world_backup_restore` 和 `restore_world_backup` 先返回排队任务，随后前一次 `get_task` 返回排队状态、下一次返回终态结果。真实 `mc-server-tui` 进程在 80×24 PTY 中依次完成：

1. 进入实例详情并创建停机备份，再按 `l` 打开“世界与备份”，确认首次列表请求使用目标实例、`limit=3` 和空 cursor，并轮询列表任务到已校验页面。
2. 按 `v` 校验选中快照，确认 TUI 提交的实例 ID 和备份 ID 正确。
3. 按 Enter 生成恢复影响预览，页面显示替换、新增、移除目录、空间、安全边界、事务步骤与确认引导。
4. 按 `c` 打开确认输入，输入完整 `RESTORE` 后提交；fake daemon 回读的 `confirmation_token` 与计划中的版本化 SHA-256 token 完全一致。
5. 轮询恢复任务到成功画面，再按 Esc 返回实例详情，继续启动、查看增量日志、发送控制台命令、优雅停服并正常恢复终端。

浏览器投射在实测 80×24 的 `830×500` 视口下保存了 `theme-backup-manager-80x24.png`、`theme-backup-restore-plan-80x24.png` 与 `theme-backup-restore-confirm-80x24.png`。视觉复核确认左右分栏、选中行、状态色、长 ID 可读性、影响列表、事务步骤、确认输入和底部快捷键层级；截图只作为视觉审查证据，导航、动画、输入和确认提交由状态测试与 PTY 字节流回归证明。

## 协议 v8 引导、进度与整合包检查闭环（2026-10-18）

这一轮把普通用户路径作为主要验收对象，而不是验证开发者是否知道如何填写版本或 Java 路径。

### 命令发现与分层

- 在仪表盘按 `/` 打开全屏命令面板；空查询保持“创建 → 实例 → 导航 → 应用”的意图顺序。
- 输入 `new` 可模糊命中“新建 Paper 服务器”，输入 `import` 可命中“导入整合包”；中英文别名、描述和快捷键都参与搜索。
- 面板每项使用“标题 + 说明 + 快捷键”的两行结构；Up/Down 选择、Enter 执行、Esc 关闭；执行后关闭面板并把动作送回共享 `Action` 调度。
- `Ctrl-K` 在文本编辑界面可用于全局命令发现，普通 `/` 输入仍保留给文本字段。
- 仪表盘空状态直接展示三步引导：Paper 建服、整合包检查、后台任务可恢复。

### 创建向导

真实 TUI 在 80×24 PTY 中从命令面板进入创建向导，等待 `doctor` 和 Paper 元数据异步返回。版本与 Java 均为列表选择，不允许把自由文本伪装成已发现事实。

1. 输入实例名，`Tab` 切到版本字段，`←/→` 把默认稳定版本从 1.21.11 改为 1.21.10。
2. Enter 进入“Java、内存与端口”：Java 21 被标记为推荐，Java 25 作为备选；内存档位显示 0.5–1 GiB、1–2 GiB、2–4 GiB、4–8 GiB；端口自动选择可用值。
3. Enter 进入“安装计划与 EULA”：显示 Paper 1.21.10、Java 发行版/架构、内存、端口、来源 `Paper 官方 CDN` 与 SHA-256 校验承诺。
4. Space 显式接受 EULA，Enter 提交。PTY fake daemon 验证请求中的名称、MC 版本、Java executable、内存、端口和 `accept_eula=true`。

该回归脚本是 `python3 scripts/tui_guided_create_pty.py`。它同时验证命令面板入口、列表选项、Review、提交参数和取消后终态。

### 字节进度与取消

创建任务返回后，TUI 进入快速轮询并打开任务详情。fake daemon 返回：

- `27.0 MiB / 54.0 MiB`、`50%`、`3.0 MiB/s`；
- 来源 `Paper 官方 CDN · fill-data.papermc.io`；
- 任务阶段 `下载校验 · 1/3`。

按第一次 `x` 只进入确认态，提示“再次按 x 确认取消；下载已完成的部分会保留用于续传”。第二次 `x` 才把 `cancel_task` 发送给 daemon。fake daemon 记录请求次数，下一次 `get_task` 返回 `cancelled`；PTY 回归要求恰好一次取消请求，并断言最终显示“已取消”。后端测试另外验证 `TaskStatus::Cancelled`、`TaskPhase::Cancelled`、无错误载荷，以及已取消的 `DownloadControl` 在触网前返回 `TASK_CANCELLED`。

### 整合包静态检查

命令面板选择“导入整合包”后，TUI 扫描当前目录、Downloads 和桌面中的 `.mrpack`。选择文件后，daemon 在 blocking worker 中只读分析并返回结构化报告。报告页显示：

- `可以导入 / 需要确认 / 禁止导入` 单一状态；
- 包名、版本、Minecraft 版本和唯一加载器；
- 安装/待确认/排除/Mod/server-overrides 数量与总大小；
- 每条问题的严重级别、证据、下一步动作；
- “当前只读检查不会下载 Mod，也不会创建或修改实例”的边界提示。

当前 TUI PTY 回归使用 fake daemon 覆盖选择与分析结果渲染；真实 `.mrpack` ZIP 解析由 `mc-modpack` 集成测试覆盖，包括 `env.server=required/optional/unsupported`、客户端资源排除、server-overrides 计数和 `../escape.jar` 路径越界阻断。

### 新视觉截图

在 830×500 浏览器视口、实测 80×24 终端下保存：

- `output/playwright/theme-guided-dashboard-80x24.png`
- `output/playwright/theme-command-palette-80x24.png`
- `output/playwright/theme-create-version-80x24.png`
- `output/playwright/theme-create-runtime-80x24.png`
- `output/playwright/theme-create-review-80x24.png`
- `output/playwright/theme-download-progress-80x24.png`
- `output/playwright/theme-modpack-picker-80x24.png`
- `output/playwright/theme-modpack-report-80x24.png`

视觉复核确认：命令面板以整屏暗色 scrim 清空底层信息，分组标题、标题/描述两行层级和选中背景清晰；向导进度点、字段焦点、推荐 Java、内存档位和 Review 资源摘要可读；任务详情同时显示进度条、字节、百分比、速度、来源和两级取消提示；整合包页面明确区分选择、证据、下一步与只读边界。

### 当前明确边界

- 下载来源目前只展示并验证官方 Paper CDN，不提供无真实传输实现和可信域名的装饰性镜像下拉框。镜像仍受官方 hash/size 权威、候选隔离和失败回退设计约束，详见 `INTERACTION_DESIGN.md` 与 `MIRROR_ACCELERATION_DESIGN.md`。
- `.mrpack` 当前完成只读适配分析，尚未执行文件下载、覆盖解压、加载器安装和首启验证。下一阶段必须把这些操作纳入持久任务和回滚边界。
- 取消目前由下载传输路径实际协作检查；未来新增的长时间任务必须同样定期检查 `TaskReporter::is_cancelled()`，不能仅依赖 UI 请求。

## 本轮自动化回归

- `cargo test --workspace --all-features --no-fail-fast`：通过，131 项测试全部通过，其中 `mc-tui` 52 项、`mc-modpack` 2 项、`mc-task` 10 项、`mc-download` 8 项、`mc-core` 7 项。
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`：通过。
- `cargo fmt --all --check`：通过。
- `python3 -m py_compile scripts/tui_visual_fake_daemon.py scripts/tui_pty_smoke.py scripts/tui_task_detail_pty.py scripts/tui_guided_create_pty.py scripts/tui_instance_detail_pty.py`：通过。
- 四个 PTY 脚本全部通过：仪表盘及向导在 daemon 请求挂起时保持响应，Ctrl-C 在 0.00 秒内退出并恢复 alternate screen；命令面板新建流程验证稳定版本、推荐 Java、内存、端口和 EULA 参数；任务详情显示 50%、27/54 MiB、3.0 MiB/s 与真实来源，两次 `x` 后恰好发送一次取消请求并进入已取消状态；任务事件游标 `0 → 2 → 3` 且重开从最后游标继续，运行任务 spinner 输出至少两种帧；实例页 `b` 提交目标实例备份，`l` 打开备份管理器并轮询分页列表任务；随后 `v` 校验选中断点，Enter 生成恢复预览，输入 `RESTORE` 后恢复请求携带完整后端 token；返回实例详情后启动、日志增量、控制台命令、优雅停止和终端恢复均通过。实例 PTY 另外确认 TUI 发出了配置的 24 位 ANSI 品牌色，并观察到备份任务 spinner 至少两种不同帧。

### 浏览器投射视觉闭环（2026-10-17）

使用 `ttyd 1.7.7` 将真实 `mc-server-tui` 进程投射到仅监听 `127.0.0.1:7681` 的浏览器终端，并连接协议 v7 假 daemon。Playwright 截取实测为 80×24 终端的 830×500 浏览器视口，视觉检查仪表盘、实例详情、世界与备份管理器和恢复影响预览：仪表盘使用堆叠面板，实例详情显示状态色、强调字段、日志区和分色快捷键，备份管理器使用左右分栏并突出选中快照与安全操作，确认页同时展示影响、事务步骤与已输入的 `RESTORE`。图像保存在 `output/playwright/theme-dashboard-80x24.png`、`theme-instance-detail-80x24.png`、`theme-backup-manager-80x24.png`、`theme-backup-restore-plan-80x24.png` 与 `theme-backup-restore-confirm-80x24.png`。浏览器截图中的任务状态处于活动态；动画帧变化通过 PTY 字节流回归验证。

![80×24 彩色仪表盘](../output/playwright/theme-dashboard-80x24.png)

![80×24 彩色实例详情](../output/playwright/theme-instance-detail-80x24.png)

![80×24 世界与备份管理器](../output/playwright/theme-backup-manager-80x24.png)

![80×24 恢复影响预览](../output/playwright/theme-backup-restore-plan-80x24.png)

![80×24 恢复确认输入](../output/playwright/theme-backup-restore-confirm-80x24.png)

### 引导与整合包视觉验收（2026-10-18）

同一套 `ttyd` + Playwright 流程连接协议 v8 假 daemon，在实测 80×24 终端中完成命令面板、向导三阶段、下载进度及整合包检查截图。截图只证明静态视觉层级；动画、输入、取消请求和 alternate-screen 恢复仍由 PTY/状态回归证明。

![80×24 引导式仪表盘](../output/playwright/theme-guided-dashboard-80x24.png)

![80×24 命令面板](../output/playwright/theme-command-palette-80x24.png)

![80×24 版本选择](../output/playwright/theme-create-version-80x24.png)

![80×24 Java 与内存选择](../output/playwright/theme-create-runtime-80x24.png)
![80×24 安装计划与 EULA](../output/playwright/theme-create-review-80x24.png)

![80×24 下载进度与来源](../output/playwright/theme-download-progress-80x24.png)

![80×24 整合包选择](../output/playwright/theme-modpack-picker-80x24.png)

![80×24 整合包适配报告](../output/playwright/theme-modpack-report-80x24.png)
