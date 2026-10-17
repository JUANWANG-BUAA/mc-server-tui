# TUI feedback and test loop

The TUI uses the daemon API and durable task event cursor; rendering and input handling stay separate from daemon I/O. This makes the UI testable without contacting Paper or waiting on real downloads.

## Automated checks

1. Keep screen state and wizard input separate from rendering and daemon I/O. State tests cover field validation, back navigation with retained values, explicit EULA acceptance, queued task success, retry after a submission error, monotonic event cursors, duplicate event replay, and retaining the last good detail after a failed refresh.
2. Render dashboard, wizard, task detail, and instance detail states to an in-memory terminal backend. Cover 80×24 reference layouts and 40×12 compact layouts, including every wizard step, EULA control, task progress/event cursor, server state/address, and lifecycle controls.
3. Run input and polling together against fake daemons that delay replies or emit ordered events/log chunks. Assert that navigation, wizard text input, console editing, and Ctrl-C remain responsive while a request is pending; task detail resumes from its event sequence; instance detail resumes from its log byte offset; and a failed request preserves the last good data.
4. Use a PTY smoke test for raw-mode entry, resize, key handling, and terminal restoration on normal exit and panic. Keep this separate from view-model tests because terminal ownership is process-level behavior.

## Visual review

For a release candidate, run the TUI in a real terminal at the reference sizes, drive the fake daemon through each task state, and capture terminal screenshots. Review hierarchy, truncation, contrast, progress indication, and resize behavior against the same fixtures used by the renderer tests. A screenshot is a review artifact; the deterministic cell snapshot remains the CI regression check.

The screen sequence is tested independently:

| Screen | In-memory renderer/state | PTY interaction |
| --- | --- | --- |
| Dashboard | Empty and populated layout at 80×24; compact layout at 40×12 | Concurrent initial refresh while daemon replies are stalled |
| Creation wizard | All three steps, field validation, review and EULA at both sizes | Field entry, submit, and Ctrl-C while list requests are stalled |
| Task detail | Status, events, cursor at both sizes | Events `1→2→3`, refresh cursor, close and reopen from `3` |
| Instance detail | State, address, logs and controls at both sizes | Enter detail, queue a stopped-world backup and bounded first-page listing, inspect the list result in task detail, start, incremental console log, console command, graceful stop, return and restore terminal |

Run `cargo build -p mc-tui && python3 scripts/tui_pty_smoke.py && python3 scripts/tui_task_detail_pty.py && python3 scripts/tui_instance_detail_pty.py`. These PTY checks validate the terminal stream and key path against fake daemons.

## 历史真实终端验收记录（2026-10-17）

历史真实终端记录使用协议 v5 假 daemon，在 macOS Terminal 的独立 80×24 窗口运行真实 `mc-server-tui` 可执行文件。当前接口扩展后，`scripts/tui_visual_fake_daemon.py` 和 PTY 假 daemon 已升级到协议 v6，并提供分页列表结果；本轮通过实际 TUI 进程的 PTY 测试验证 `l` 请求及任务详情结果。此轮没有重新采集可用于像素检查的宿主终端截图。

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

重新构建 `mc-server-tui`，由真实 TUI 进程连接协议 v6 PTY fake daemon，在 80×24 终端尺寸依次执行实例详情 `l`、返回任务面板、打开列表任务。已确认请求携带目标实例、`limit=3` 和空 cursor；详情页显示一条备份摘要的 ID 前缀、文件数 `12`、大小 `2.0 MiB`，并能返回仪表盘。随后继续验证启停、控制台命令、日志增量和 alternate screen 恢复。该回归覆盖真实终端事件与渲染字节流，不是像素截图验收。

## 本轮自动化回归

- `cargo test --workspace --all-features --no-fail-fast`：通过，85 项单元测试全部通过。
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`：通过。
- `cargo fmt --all --check`：通过。
- `python3 -m py_compile scripts/tui_visual_fake_daemon.py scripts/tui_pty_smoke.py scripts/tui_task_detail_pty.py scripts/tui_instance_detail_pty.py`：通过。
- 三个 PTY 脚本全部通过：仪表盘及向导在 daemon 请求挂起时保持响应，Ctrl-C 在 0.00 秒内退出并恢复 alternate screen；任务事件游标 `0 → 2 → 3` 且重开从最后游标继续；实例页 `b`/`l` 分别提交目标实例备份和有界列表任务，任务详情显示文件数与总字节数，之后启动、日志增量、控制台命令、优雅停止和终端恢复均通过。
