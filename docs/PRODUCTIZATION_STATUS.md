# 产品化维护状态

更新：2026-10-21。发布基线为 v0.2.1；下列修复属于源码中的未发布改动。
用户操作以[中文手册](USER_GUIDE.zh-CN.md)、[英文手册](USER_GUIDE.md)和
[快速开始](QUICK_START.md)为入口。已发布范围见[实施差距](IMPLEMENTATION_GAPS.md)。

## 本轮收敛

- 默认会话和经典视图共用首次使用入口。Paper 打开向导；Fabric 填入已有命令，
  由用户补充名称、精确版本并确认 EULA；整合包入口打开检查流程。
- 错误对话框接到用户操作失败路径，展示原因、恢复建议与可展开的证据。
  支持滚动和关闭；后台连接轮询失败保留状态栏反馈。
- 模板转换保留后台错误的原因、证据、恢复建议和重试标记。
  剪贴板复制、自动重试和 Fabric 图形向导尚未实现。
- 已挂接原先未执行的错误测试，修复无法编译的示例，将空 E2E 实现替换为真实 PTY 测试。
  安装器统一使用发布链路中的 `scripts/install.sh`。
- 合并上一轮重复的计划和完成报告，清理误提交的浏览器临时记录与会话 HTML。
  原始材料可在提交 `ad4cbdf`、`69cafbd` 中查询；其中的完成率、覆盖率和
  “全部验证通过”不作为当前验收证据。

## 后续优先级

1. 完成真实 Linux 用户服务与真实 Minecraft 客户端验收；恢复后进入游戏核对世界。
2. 按实际使用反馈推进 Vanilla、现有目录接管和更多配置编辑；保留各自验收标准。
3. 在线/定时备份须先完成一致性和故障恢复设计，再决定实现范围。
4. 签名/公证、Homebrew tap 和 Linux 包随分发验收推进。

不承诺六周内发布正式版。现有稳定性证据不足以宣称生产就绪，也没有
“最适合 3–5 个服务器”的性能测试结论。

## 验证方式

2026-10-21 本地 macOS 验证：下列命令均通过。Rust 全目标测试为 258 项通过、
0 项失败、0 项忽略；五套 PTY 流程和真实 daemon 协议窗口检查通过。
文档检查覆盖 125 个本地路径引用，并确认双语网页与源文件一致；不检查外部网址或锚点。
这些结果来自本机执行，远程 CI 与下述真实环境验收仍需各自执行。

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo test --workspace --doc --locked
cargo build --workspace --locked
python3 scripts/check_docs.py
python3 scripts/check_protocol_window.py
python3 scripts/tui_pty_smoke.py
python3 scripts/test_first_time_experience.py
python3 scripts/tui_guided_create_pty.py
python3 scripts/tui_task_detail_pty.py
python3 scripts/tui_instance_detail_pty.py
```

PTY 测试运行真实 TUI 二进制并使用隔离的模拟 daemon。它们验证终端交互，
不证明真实上游下载、Java 游戏进程、Linux 服务安装或客户端联机成功。
原有真实 Paper 演练见 [v0.2.1 验收](acceptance/0.2.1.md)。
