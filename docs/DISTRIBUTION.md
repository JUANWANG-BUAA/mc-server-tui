# macOS / Linux 分发与热更新设计

> 版本：0.2.2 · 2026-10-21
>
> 状态：Phase 0/1 与 `v0.2.2` unsigned alpha 发布已完成；协议窗口为 8–11；Phase 2–4 仍待完成
>
> 关联：[实现差距](IMPLEMENTATION_GAPS.md)、[监督契约](SUPERVISION.md)、[产品设计的发布要求](../PRODUCT_DESIGN.md)

## 0. 一句话结论

当前代码已经具备做“无停机更新”的最关键基础：**Java 由 per-instance runner 持有，management daemon 退出后 runner 继续运行，新 daemon 可以重连**。  
Phase 0 已补上 `mc` 启动器、`config.toml`/默认 User-Agent、`version --json`、`ServerInfo` 探针和 launchd/systemd 用户服务管理；Phase 1 已补上 versioned tarball、SHA-256、release manifest、`install.sh`、`mc uninstall` 和 Homebrew formula 渲染。API 已支持 8–11 协议窗口，runner 控制协议为 v1，`mc update status/rollback` 已能切换 `current`/`previous` 并重启服务。仍然缺的是：签名/公证、Homebrew tap 实际发布、状态迁移快照、更新前排空任务、签名自动更新器和健康检查式自动回退。

## 0.1 Phase 0 实装边界（2026-10-19）

已落地并能运行：

- `crates/mc-config`：`config.toml`、默认 User-Agent、state/socket/cache/log 路径、环境变量覆盖。
- `crates/mc`：`mc` / `mc tui` / `mc daemon run` / `mc cli ...` / `mc version --json` / `mc status --json` / `mc setup` / `mc service ...` / `mc uninstall`。
- `ServerInfo`：Ping 之外的新增强探针，返回产品版本和 API 协议范围；旧 daemon 仍可回退到 Ping。
- launchd LaunchAgent / systemd user unit：`install/start/stop/restart/status/uninstall`，`--dry-run` 输出生成内容；launchd 已在 macOS 做过临时配置真实闭环。
- Phase 1：`scripts/package_release.py` 生成 tar.gz + SHA-256 + manifest；`scripts/install.sh` 安装到 `versions/<version>`、切换 `current`、写稳定 launcher；`mc uninstall` 清理服务/launcher/版本目录并保留数据；`packaging/homebrew/` 模板与渲染脚本。
- 基础 CI：macOS 14 / Ubuntu 22.04，fmt、clippy、test、build、launcher/PTY smoke；release job 打包并执行本地 installer/uninstaller smoke。
- 发布历史：`v0.1.0` 是首个 public alpha；当前 `v0.2.2`（unsigned alpha pre-release）包含三个 target（Apple Silicon、Linux x86_64、Linux arm64）、合并 manifest、install.sh 和 Homebrew formula。

尚未落地（Phase 2+）：

- 签名/公证、实际 Homebrew tap、apt/dnf 仓库、SBOM。
- ~~daemon 接受 N/N-1 协议~~：已接受 8–11，并按请求版本回包；旧客户端看不到高于自身版本的协议操作与任务结果。
- ~~runner 协议握手~~：控制帧携带 `protocol: 1`，缺失字段按 v1 处理。
- ~~手工回滚~~：`mc update status/rollback` 已切换 `current`/`previous` 并重启服务。
- 状态 schema 迁移快照、更新前排空任务、签名自动更新器、健康检查式自动回退。
- Linux systemd 真实安装测试（当前有生成单测、`systemd-analyze` 测试和 CI 交叉编译）；macOS 公证；开机前启动/linger。

## 1. 目标与非目标

### 目标

- macOS Apple Silicon 与 Linux x86_64 / arm64 提供同一条用户路径：安装 → `mc setup` → 建服 → 日常管理 → 更新 → 卸载。
- 优先用户级安装，不需要 sudo；实例数据、日志、socket 都在当前用户目录下。
- 组件来源可验证：SHA-256 + 签名；macOS 签名/公证；Linux 包/仓库签名。
- 默认保留实例数据；卸载只移除程序和服务，`--purge` 才删除数据。
- 更新时 **不杀正在运行的 Minecraft Java 进程**；更新失败可回滚程序，数据迁移前有快照。
- 包管理器安装（brew/apt/dnf）不与应用内 self-update 抢文件；检测来源并给出正确升级命令。

### 非目标（首版）

- Windows 原生支持。当前代码依赖 Unix socket 和 Unix 进程模型。
- 在远端主机上只装 TUI 的 SSH 模式（P2）。
- 无审核的第三方镜像、自动从非官方源拉服务端包。
- 在运行中的恢复事务/备份任务上强行热更新；默认拒绝，`--force` 只作为有明确警告的逃生口。

## 2. 现有架构与热更新的关系

```text
TUI (client) ── Unix socket ──> daemon (API 8–11)
                                   │
                                   ├─ task manager / SQLite
                                   ├─ backup / restore
                                   └─ supervisor
                                        │
                                        └─ per-instance runner (detached)
                                             │
                                             └─ java -jar server.jar
```

关键事实（来自当前代码）：

- `mc-daemon` 以 `--internal-runner=<uuid>` 启动 runner；runner 是同一个 daemon 可执行文件的隐藏模式。因此**已有 runner 在磁盘二进制被替换后仍持有旧 inode**；更新必须使用原子版本目录/rename，不能原地覆盖正在运行的二进制。
- runner 独立于 daemon，持有 Java stdin/stdout/stderr，绑定私有 Unix socket，写 `server/supervisor.json`。
- daemon 重启后通过 `supervisor.json` 和 runner socket 重连；已有真实子进程测试。新 daemon 管理旧 runner 已通过 runner 协议 v1 握手表达，不再依赖“当前 JSON line 恰好兼容”。
- TUI 只是客户端；关闭 TUI 不影响服务器。
- daemon/TUI/CLI 已支持 **API 8–11 协商**；daemon 更新后旧 TUI 仍可按自己的版本工作，超出协商版本的管理、配置编辑和重启操作会在客户端本地被拒绝。
- 任务在 daemon 重启时从 queued/running 变成 `interrupted`，不会自动重放；下载有 `.part` 续传，但任务本身不恢复。
- 实例数据 schema 目前只有 v1；没有 in-place migration。
- runner 控制协议为 v1：控制帧携带 `protocol: 1`，缺失字段按 v1 处理。

这些事实决定了热更新策略：**daemon 可以短暂重启，runner 不动；但更新必须避开活动任务，并且 TUI/daemon 的协议兼容需要单独设计。**

## 3. 用户看到的入口：稳定的 `mc`

当前用户要记住三个二进制：

- `mc-server-tui`
- `mc-server-tui-daemon`
- `mc-server-tui-cli`

这不利于安装、服务单元和热更新。Phase 0 已增加稳定的启动器 `mc`，由它分发子命令并解析版本化安装目录：

```text
mc                     # 确保 daemon 可用，然后进入 TUI
mc setup               # 首次配置：Java、User-Agent、数据目录、服务安装
mc daemon run          # 前台 daemon（服务单元用）
mc daemon start|stop|restart|status
mc service install|uninstall|status|restart
mc cli <args>          # 等价于 mc-server-tui-cli
mc doctor [--json]
mc version [--json]    # 版本、target、协议、状态 schema、runner 协议
mc update status|rollback [--dry-run] [--no-restart]  # rollback implemented; update/check Phase 4
mc logs                                          # Phase 1（或先看服务日志）
mc uninstall
```

Phase 0/1 实际可用的子命令是 `tui`、`daemon run`、`cli`、`version`、`status`、`doctor`、`setup`、`service`、`uninstall`，以及 `update status/rollback`；`update` 的下载/校验/排空流程和 `logs` 仍是后续阶段。实现方式有两种：

| 方案 | 优点 | 缺点 | 建议 |
| --- | --- | --- | --- |
| `mc` 启动器 + 现有三个二进制 | 改动小；启动器只做 resolve/exec；签名和版本切换容易 | 多一个进程（exec 后可忽略）；包体稍大 | **Phase 0 已采用** |
| 合并成一个 `mc` 多合一二进制 | 单文件、签名简单、热更新只替换一个文件 | 需要重构 bin/clap/模块；daemon/runner 的 `current_exe` 逻辑要调整 | Phase 3 评估 |

`mc` 必须是稳定路径（例如 `~/.local/bin/mc`），它读取 `current` 实际版本并 `exec`：

```text
~/.local/share/mc-server-tui/
  versions/
    0.1.0/...
    0.2.0/bin/mc
    0.2.0/bin/mc-server-tui
    0.2.0/bin/mc-server-tui-daemon
    0.2.0/bin/mc-server-tui-cli
  current -> versions/0.2.0
  previous -> versions/0.1.0
```

升级只做原子 `rename`/symlink 切换；运行中的进程继续用旧 inode。  
`mc` 本身由安装脚本放到 `~/.local/bin`，不随版本目录切换；服务单元执行 `.../current/bin/mc daemon run` 或稳定路径 `mc daemon run`。

## 4. 安装布局与用户目录

建议保持现有 `directories::ProjectDirs` 的状态/缓存路径，避免迁移：

| 内容 | macOS | Linux |
| --- | --- | --- |
| 程序版本 | `~/.local/share/mc-server-tui/versions/<ver>` | 同左 |
| 稳定入口 | `~/.local/bin/mc` | 同左 |
| 配置 | `~/Library/Application Support/org.mc-server-tui.mc-server-tui/config.toml` | `~/.config/org.mc-server-tui.mc-server-tui/config.toml` |
| 实例/DB | `~/Library/Application Support/org.mc-server-tui.mc-server-tui` | `~/.local/state/org.mc-server-tui.mc-server-tui` |
| 缓存 | `~/Library/Caches/org.mc-server-tui.mc-server-tui` | `~/.cache/org.mc-server-tui.mc-server-tui` |
| socket | `<state>/run/daemon.sock` | 同左 |
| 服务日志 | `~/Library/Logs/mc-server-tui/daemon.log` | systemd journal；可选 `<state>/logs/daemon.log` |

Homebrew 安装时程序在 `brew --prefix` 下，不适用 `~/.local/share`；状态/配置仍在上述用户目录。`mc` 需要能识别安装来源：

```toml
# config.toml
[install]
# brew | apt | rpm | tarball | self-managed
source = "tarball"

[daemon]
state_dir = ""            # 空 = 平台默认
socket = ""               # 空 = state/run/daemon.sock
user_agent = "mc-server-tui/<version> (+https://github.com/<org>/<repo>)"
log_level = "info"

[update]
channel = "stable"        # stable | beta
auto_check = true
auto_install = false

[ui]
theme = "claude"
```

优先级：CLI flag > 环境变量 > `config.toml` > 平台默认。环境变量保留给 CI/测试；普通用户不再需要理解 `MC_SERVER_TUI_USER_AGENT`。

> **User-Agent 是发布阻塞项。** 当前 daemon 没有环境变量就退出。分发必须提供一个真实的项目 User-Agent 默认值，并允许用户覆盖；不能要求普通用户先读 README 再设置环境变量。

## 5. 安装渠道与产物矩阵

### 5.1 产物

| 平台 | Target | 产物 | 备注 |
| --- | --- | --- | --- |
| macOS Apple Silicon | `aarch64-apple-darwin` | `.tar.gz` / `.zip` | 签名 + 公证 |
| macOS Intel | `x86_64-apple-darwin` | 未发布 | 当前发布不包含 Intel macOS 产物 |
| macOS Universal | `universal2` | `.pkg` / `.dmg`（可选） | Homebrew 可用单架构；pkg 面向非 Homebrew 用户 |
| Linux x86_64 | `x86_64-unknown-linux-musl` | `.tar.gz` | 静态，减少 glibc 差异 |
| Linux arm64 | `aarch64-unknown-linux-musl` | `.tar.gz` | 静态 |
| Linux x86_64 | glibc（Ubuntu 22.04 基线） | `.deb` / `.rpm` | 系统包用户 |
| Linux arm64 | glibc | `.deb` / `.rpm` | 可选 |

Linux 静态分发前要先处理一个当前实现细节：TUI 的 CPU/RSS 读数是 shell out 到 `ps`。最小化发行版可能没有 procps；应在 Linux 上改用 `/proc/<pid>/stat`（macOS 继续用 `ps`），或把 `procps` 写进包依赖。静态 musl tarball 不应暗中依赖系统 `ps`。

每个产物同时发布：

- `<file>.sha256`
- `<file>.minisig`（minisign）或 cosign signature
- CycloneDX `sbom.cdx.json`
- 合并的 `release-manifest.json` + `release-manifest.json.minisig`

### 5.2 渠道优先级

1. **Homebrew tap**（macOS 首选）：`brew install <org>/tap/mc-server-tui`；formula 从源码或 bottle 安装，`mc setup` 装用户服务。
2. **一键 tarball 安装**（macOS/Linux）：`curl -fsSL .../install.sh | sh`，不 sudo；校验签名/哈希；装到 `~/.local`。
3. **Linux 系统包**：`.deb`/`.rpm`；包安装只放二进制和 systemd unit，不自动替用户 enable（避免无用户会话时启动失败）；用户执行 `mc setup`/`mc service install`。
4. **Homebrew on Linux**：可选，复用 formula。
5. **不建议**为这个 CLI/daemon 做 AppImage：AppImage 更适合桌面 GUI，服务/文件路径/升级语义不匹配。

### 5.3 安装脚本（Phase 1 已实现）

`scripts/install.sh` 的实际流程：

```sh
detect_os_arch
resolve_channel_version()      # 默认 stable，可 --version 固定
download manifest + signature
verify_signature(manifest)     # 内置公钥
select target
download artifact + sha256 + signature
verify_sha256
verify_signature
extract to versions/.staging-<ver>
fsync + rename to versions/<ver>
atomically switch current
install launcher to ~/.local/bin/mc
print "next: mc setup"
```

安全要求：

- 默认不 `sudo`，不写 `/usr/local`（除非用户显式 `--prefix`）。
- `curl | sh` 的脚本本身要可下载、可审计；支持 `--dry-run`、`--version`、`--channel`、`--no-service`。
- 失败不留下半安装；保留上一次 `current`。
- 不自动修改 shell profile；只在 `mc` 不在 PATH 时提示。

## 6. 用户级服务：launchd 与 systemd

### 6.1 共同要求

- 以当前用户运行；不 root。
- 服务重启 **不能杀死 runner/Java**。
- 日志可查；启动失败给出明确下一动作。
- 服务单元由 `mc service install` 生成，路径基于实际安装目录，不硬编码 Homebrew/用户路径。
- 安装/卸载幂等；`status` 返回 running/stopped/not-installed。
- 登录自动启动；Linux 可选 `loginctl enable-linger` 支持开机（无登录）启动，但必须显式询问用户。

### 6.2 macOS LaunchAgent

文件：`~/Library/LaunchAgents/org.mc-server-tui.daemon.plist`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>org.mc-server-tui.daemon</string>
  <key>ProgramArguments</key>
  <array>
    <string>/Users/USER/.local/bin/mc</string>
    <string>daemon</string>
    <string>run</string>
  </array>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
  <key>AbandonProcessGroup</key><true/>
  <key>ProcessType</key><string>Standard</string>
  <key>StandardOutPath</key>
  <string>/Users/USER/Library/Logs/mc-server-tui/daemon.log</string>
  <key>StandardErrorPath</key>
  <string>/Users/USER/Library/Logs/mc-server-tui/daemon.log</string>
  <key>EnvironmentVariables</key>
  <dict>
    <key>MC_SERVER_TUI_CONFIG</key>
    <string>/Users/USER/Library/Application Support/org.mc-server-tui.mc-server-tui/config.toml</string>
  </dict>
</dict>
</plist>
```

`AbandonProcessGroup` 是关键：服务重启/`launchctl kickstart -k` 不应连带杀死 runner。  
命令：

```sh
launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/org.mc-server-tui.daemon.plist
launchctl kickstart -k gui/$(id -u)/org.mc-server-tui.daemon
launchctl bootout gui/$(id -u)/org.mc-server-tui.daemon
```

注意：只有用户登录 GUI 会话后才运行。若要在登录前启动，需要 LaunchDaemon（root）或用户 lingering 方案；首版不做，文档明确边界。

### 6.3 Linux systemd user unit

文件：`~/.config/systemd/user/mc-server-tui.service`

```ini
[Unit]
Description=mc-server-tui management daemon
Documentation=https://github.com/ORG/REPO
After=network-online.target

[Service]
Type=simple
ExecStart=%h/.local/bin/mc daemon run
Restart=on-failure
RestartSec=2
# 关键：只杀 daemon，不杀 runner/Java 子进程
KillMode=process
KillSignal=SIGTERM
TimeoutStopSec=20
# 不要让 systemd 把整个 cgroup 一起清掉
# 如果使用 cgroup v2 + 额外隔离，可评估 Delegate=yes
Environment=MC_SERVER_TUI_CONFIG=%h/.config/org.mc-server-tui.mc-server-tui/config.toml
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=default.target
```

命令：

```sh
systemctl --user daemon-reload
systemctl --user enable --now mc-server-tui.service
systemctl --user restart mc-server-tui.service
systemctl --user status mc-server-tui.service
loginctl enable-linger "$USER"   # 可选，开机/未登录也运行
```

`KillMode=process` 是热更新的硬要求；默认的 `control-group` 会在 daemon 重启时杀掉 runner 和 Minecraft，直接违背“更新不杀服”。这一条必须在安装后验证。

非 systemd 发行版（Alpine/OpenRC、s6）首版只提供 tarball + `mc daemon run`，不承诺自动启动；后续可加 OpenRC 脚本。

## 7. 发布流水线

### 7.1 CI 矩阵

建议 `.github/workflows/release.yml`：

```text
fmt + clippy + test (macOS arm64, Linux x86_64)
  ↓
build matrix:
  aarch64-apple-darwin      macos-14 (arm64)
  universal2                lipo 两个 darwin 产物
  x86_64-unknown-linux-musl ubuntu-22.04 + musl toolchain / cross
  aarch64-unknown-linux-musl ubuntu-24.04-arm 或 cross
  x86_64-unknown-linux-gnu  ubuntu-22.04（deb/rpm）
  ↓
sign / notarize / package / sbom / checksums
  ↓
release-manifest.json + signature
  ↓
GitHub Release + Homebrew tap PR + apt/dnf repo publish
```

要求：

- `cargo build --release --locked`；记录 Rust 版本和 target。
- `cargo test --workspace`；对 Linux 跑 `/proc` runner 测试；PTY 回归在 macOS/Linux 都跑（需要 PTY）。
- `cargo clippy --workspace --all-targets -- -D warnings`；`cargo fmt --check`。
- `cargo deny check` / `cargo audit`；生成 SBOM。
- 产物在 CI 中签名；私钥只存在受保护的 release environment。
- 发布说明包含：版本、协议范围、状态 schema、最低支持版本、破坏性变更、回滚说明。

### 7.2 工具选择

| 方案 | 适合 | 注意 |
| --- | --- | --- |
| `cargo-dist` | GitHub Release、shell installer、Homebrew formula、checksums | 对 macOS 公证、deb/rpm、服务安装支持有限，需要自定义步骤 |
| 自定义 GitHub Actions + `nfpm` | 完全控制签名/公证/包 | 工作量大，但发布链路透明 |
| `cargo-release` | 版本号/tag/CHANGELOG | 不负责打包 |

Phase 0/1 已提交基础 CI（fmt/clippy/test/build/PTY smoke + 打包/安装/uninstall smoke）。建议：**Phase 1 用 cargo-dist 打底**（产物/安装器/Homebrew），**Phase 2 增加自定义 `notarytool`、`pkgbuild`、`nfpm`、仓库签名步骤**。如果团队更看重可审计性，直接写自定义 workflow 也合理。

### 7.3 macOS 签名与公证

需要对每个可执行文件和最终 `.pkg`/`.dmg`：

```sh
codesign --force --options runtime --timestamp \
  --sign "Developer ID Application: ORG (TEAMID)" \
  path/to/mc path/to/mc-server-tui ...
xcrun notarytool submit package.pkg \
  --apple-id "$APPLE_ID" --team-id "$TEAM_ID" --password "$APP_PASSWORD" --wait
xcrun stapler staple package.pkg
```

- 使用 hardened runtime；不需要给 Rust 二进制加 JIT/library-validation entitlement；Java 是外部子进程，不是加载进本进程的库。
- 公证成功后必须 `stapler`，否则离线 Gatekeeper 可能仍提示。
- Homebrew formula 从源码构建通常不经过公证；发布 bottle 时仍建议签名/公证流程，至少在 tap 文档说明 Gatekeeper 行为。
- 没有 Apple Developer ID 时，alpha 只能走 tarball/Homebrew source，并明确告知用户可能遇到 Gatekeeper 警告；不要假装“已签名”。

### 7.4 Linux 签名与仓库

- tarball/manifest：minisign（简单）或 cosign（Sigstore）。
- `.deb` 仓库：`InRelease`/`Release.gpg` 用 GPG 签名；`apt` 源提供 keyring。
- `.rpm` 仓库：`createrepo_c` + GPG 签名 `repomd.xml`。
- 包内不要自动 enable 用户服务；postinstall 只提示 `mc setup`。
- 静态 musl tarball 作为“无 root / 任意发行版”兜底；不依赖 glibc 版本。

### 7.5 发布清单（release manifest）

```json
{
  "schema": 1,
  "product": "mc-server-tui",
  "version": "0.2.0",
  "channel": "stable",
  "released_at": "2026-10-20T00:00:00Z",
  "min_supported_version": "0.1.0",
  "api_protocol": { "min": 8, "max": 11 },
  "state_schema": 1,
  "runner_protocol": { "min": 1, "max": 1 },
  "notes_url": "https://github.com/ORG/REPO/releases/tag/v0.2.0",
  "targets": {
    "aarch64-apple-darwin": {
      "url": "https://.../mc-server-tui-0.2.0-aarch64-apple-darwin.tar.gz",
      "sha256": "...",
      "size": 12345678,
      "signature": "..."
    }
  }
}
```

- 对 canonical JSON 签名；公钥编译进 `mc`，也可通过 `mc update --verify-key` 显示指纹。
- 反降级：默认拒绝低于当前版本的安装；`--allow-downgrade` 只用于明确回滚。
- `api_protocol`、`state_schema`、`runner_protocol` 是更新决策的一部分，不是装饰字段。

## 8. 热更新设计

### 8.1 可以热更新什么

| 组件 | 能否不杀 Java | 说明 |
| --- | --- | --- |
| TUI | ✅ | 纯客户端；退出/重启即可 |
| CLI | ✅ | 纯客户端；随程序替换 |
| daemon | ✅（在 runner 存活的条件下） | 短暂停止/重启；runner 继续持有 Java |
| runner | ❌ | runner 就是持有 Java 的进程；不能在运行中替换。新实例/重启后的实例使用新 runner |
| 实例数据 schema | ⚠️ | 迁移需要快照；旧程序不保证读新 schema |
| 服务单元 | ✅ | 只改路径/策略，重启 daemon 即可 |

### 8.2 前置兼容改造（必须先做）

1. ✅ **`ServerInfo`**：返回版本、api_protocol min/max、state_schema、runner_protocol。当前兼容窗口为 8–11，协议 8 以下的 daemon 不在窗口内；`Ping` 仍保留为最小存活检查。
2. ✅ **API 兼容窗口**：daemon 接受 `[8,11]` 内的客户端并按请求版本回包；客户端协商最高共同版本，高于协商版本的操作用户端会本地拒绝。
3. ✅ **runner 握手**：控制帧携带 `protocol: 1`；缺失字段按 v1 处理，新 runner 拒绝更高版本请求。
4. ❌ **state schema 迁移**：`mc migrate --check`/`--apply`；迁移前备份 `index.sqlite3` 和所有 `instance.toml`；记录迁移版本。
5. ❌ **更新保护**：`ListTasks` 有 queued/running 任务、或存在 restore journal/活动生命周期操作时，`mc update` 默认拒绝；`--wait` 等待完成，`--force` 显式警告。
6. ✅ **服务管理器抽象**：`launchctl`/`systemctl` 的 `install/start/stop/restart/status`，供 updater 调用。
7. ❌ **优雅停止 daemon**：SIGTERM 后停止接收新连接、等待在飞请求（有超时）、移除 socket、退出；不等待/不杀 runner。

### 8.3 更新流程

```text
mc update --check
  └─ 下载/验签 manifest → 显示 current/latest/channel/notes

mc update
  1. 识别安装来源（brew/apt/rpm/tarball/self）
     └─ 包管理器安装：打印升级命令，退出（不抢文件）
  2. preflight
     ├─ Ping daemon（若未运行则跳过）
     ├─ ListTasks：有活动任务 → 拒绝/等待
     ├─ 检查 restore journal / 生命周期操作
     ├─ 检查磁盘：新版本 + 状态快照空间
     └─ 检查协议/state schema 兼容
  3. 下载 + 验签 + 解压到 versions/.staging-<ver>
  4. 新二进制执行 `mc migrate --check`
     ├─ 不兼容 → 中止
     └─ 需要迁移 → 备份 state → `mc migrate --apply`
  5. 切换 current symlink（atomic rename）
  6. 重启 daemon（launchctl kickstart -k / systemctl --user restart）
  7. 健康检查
     ├─ socket/Ping + protocol range
     ├─ 原有运行实例 GetInstanceDetail：runner PID / Java PID 不变
     └─ 控制台/日志读取抽样
  8. 失败 → 切回 previous，重启 daemon；若已迁移且不可逆，恢复迁移前快照
  9. 输出：update applied；Minecraft servers kept running；restart TUI
```

关键点：

- 更新时 **runner 不能被服务管理器连带杀掉**：macOS `AbandonProcessGroup`，systemd `KillMode=process`，并在安装后做真实测试。
- 更新时 TUI 可能短暂断线；TUI 的自动重连应显示“daemon 正在更新/已恢复”，而不是一堆错误。
- 若 daemon 没在运行（用户只用 TUI 临时启动），`mc update` 只需替换二进制并提示重启 TUI；不需要服务重启。
- TUI 内不建议自己替换正在运行的可执行文件。TUI 显示“有更新，请退出后执行 `mc update`”，或提供 `/update` 打开一个受控的退出更新流程；首版先做外部 `mc update`。

### 8.4 TUI 更新

- `mc update` 更新磁盘上的 TUI 二进制；运行中的旧 TUI 不受影响。
- 用户退出 TUI 后重新启动即可用新版本；如果协议不兼容，daemon 的健康检查/提示会要求重启 TUI。
- 未来可做“更新后自动 exec 新 TUI + 保留会话历史文件”，但不要让运行中的 TUI 在 alternate screen 里替换自身。

### 8.5 包管理器安装的更新

| 来源 | 更新命令 | 应用内行为 |
| --- | --- | --- |
| Homebrew | `brew upgrade mc-server-tui` | `mc update` 只提示，不写 Homebrew 目录 |
| apt | `sudo apt update && sudo apt install --only-upgrade mc-server-tui` | 同上 |
| dnf | `sudo dnf upgrade mc-server-tui` | 同上 |
| tarball/self-managed | `mc update` | 版本化目录 + symlink + 服务重启 |

包升级后通常需要执行 `mc service restart`；如果服务单元使用 `current` 稳定路径，重启就会加载新 daemon，runner 不受影响。

### 8.6 自动检查与自动安装

```toml
[update]
channel = "stable"
auto_check = true          # 启动/每天检查一次，只提示
auto_install = false       # 默认关闭
```

- `auto_check` 只发一个 manifest 请求，不下载大包；可在设置里关闭。
- `auto_install` 默认关闭；即使打开，也必须满足“无活动任务、无恢复事务、签名验证通过”。
- 企业/服务器环境建议只用包管理器或固定版本，关闭 auto-update。
- 支持 `MC_SERVER_TUI_UPDATE_URL`/`--manifest` 做内网镜像，但必须固定公钥/允许的 URL。

## 9. 快速部署与首次运行

目标：普通用户 3 条命令内看到 TUI。

### macOS（Homebrew）

```sh
brew install ORG/tap/mc-server-tui
mc setup
mc
```

### macOS / Linux（tarball）

```sh
TAG=v0.2.2
curl -fsSL "https://github.com/JUANWANG-BUAA/mc-server-tui/releases/download/$TAG/install.sh" | sh
mc setup
mc
```

### Linux（apt 示例）

```sh
sudo install -m 0755 -d /etc/apt/keyrings
curl -fsSL https://example.invalid/mc-server-tui/key.gpg | sudo tee /etc/apt/keyrings/mc-server-tui.gpg >/dev/null
echo "deb [signed-by=/etc/apt/keyrings/mc-server-tui.gpg] https://example.invalid/mc-server-tui/deb stable main" | sudo tee /etc/apt/sources.list.d/mc-server-tui.list
sudo apt update && sudo apt install mc-server-tui
mc setup
```

### `mc setup` 应做什么

1. 检查 OS/arch/target 和安装完整性。
2. 检查 Java（可调用 `mc doctor` 或直接跑探测）；没有 Java 给出安装建议。
3. 写入/确认 `config.toml`：User-Agent（默认项目 UA，可改）、数据目录、更新通道。
4. 创建状态/日志目录，权限 0700。
5. 安装并启动 launchd/systemd 用户服务；等待 socket/Ping。
6. 显示下一步：`mc` 进入 TUI；`mc doctor` 排障；`mc service status`。
7. 可选：直接启动 TUI。

TUI 首次运行如果发现 daemon 未运行，应显示一个 setup 引导，而不是只显示“连接失败”：
- “安装并启动后台服务”（用户级）
- “仅本次运行”（临时启动 daemon）
- “打开 doctor”

`mc service` 已提供 Phase 0 的安装/启动能力；TUI 内的“未检测到 daemon，是否安装并启动服务”引导仍是下一步，不应让用户先读文档才知道 `mc setup`。

## 10. 回滚与数据安全

- 保留至少 2 个版本：`current`/`previous`；`mc update rollback` 已能原子切回并重启 daemon（`--dry-run` 只显示计划，`--no-restart` 只切换 symlink）。
- 回滚前会运行目标版本的 `mc version --json`，并与运行中 daemon 的 `state_schema` 比较；目标 schema 更旧时直接拒绝。当前仍没有迁移前状态快照，因此这只是“明确拒绝”，不是降级恢复。
- 程序回滚不等于数据回滚：
  - schema 迁移前快照 `index.sqlite3` + `instance.toml`/`lock.json`（不复制世界，避免双倍大）。
  - 记录 `state_schema`；旧程序不能读新 schema 时，回滚必须先恢复快照，或明确拒绝。
  - 世界文件不在更新事务里；更新流程绝不写 `server/` 世界目录。
- `mc uninstall` 当前保留 state/cache/config；`--purge` 和二次确认仍是后续阶段。
- runner 在更新期间不退出；如果健康检查发现 runner 未重连，只报告，不发送信号。
- 更新失败时：优先回滚程序；若 daemon 起不来，保留旧版本服务单元和日志，给 `mc doctor --json` 证据。

## 11. 安全与供应链

- `DEFAULT_PROJECT_URL` 已指向项目仓库；fork 或共享部署仍应通过 `mc setup --user-agent` 设置自己的联系方式。
- 所有下载：TLS + 签名 + SHA-256；manifest 签名优先于 HTTPS。
- 更新器不执行未验签的二进制；不把 manifest 中的任意 URL 当 shell 脚本执行。
- macOS：Developer ID + hardened runtime + notarization + staple。
- Linux：minisign/cosign + apt/rpm 仓库签名。
- `cargo deny`/`cargo audit`、`--locked`、SBOM、可复现构建参数。
- 服务单元不要求 root；Unix socket 0600、目录 0700（已实现）。
- 安装脚本支持 `--dry-run`，不静默修改 shell profile/PATH。
- 更新日志/诊断不包含用户主目录以外的敏感信息；User-Agent 中的联系地址可按用户选择隐藏。

## 12. 测试矩阵与验收

### 12.1 平台

- macOS 13/14/15，arm64 + x86_64（至少 CI 跑构建；真实机/VM 跑服务安装和更新）。
- Ubuntu 22.04/24.04 x86_64 + arm64；Debian 12；Fedora（可选）；Alpine/musl（手动 tarball）。
- systemd 与 launchd 各至少一套真实服务安装/重启/卸载测试；Phase 0 已完成 macOS launchd 临时配置闭环，Linux systemd 仍待真实主机/CI 安装。

### 12.2 必须通过的场景

1. 干净用户环境：install → `mc setup` → `mc` → 建 Paper 实例 → 启动 → 关闭 TUI → 重开仍可控制。
2. 重启 daemon：Java PID 不变；日志/控制台继续。
3. 更新 daemon（有运行中的服务器）：PID 不变；更新后 `status`/`logs`/console 正常。
4. 更新时有下载/备份/恢复任务：默认拒绝；`--wait` 等待后成功；`--force` 有明确警告。
5. 更新失败/健康检查失败：自动回滚到 previous，daemon 恢复，Java 仍在。
6. 协议不兼容：旧 TUI 得到明确“重启 TUI”提示，而不是崩溃/静默错误。
7. 状态迁移失败：不切换 current；旧 daemon 可继续运行。
8. 签名错误/哈希错误/磁盘满/无网络：不切换版本，不破坏 current。
9. 卸载：默认保留实例；`--purge` 二次确认；服务单元移除。
10. 开机/登录启动：macOS 登录后 daemon 起来；Linux `enable`（可选 linger）后 daemon 起来。
11. 包管理器升级：brew/apt/dnf 后 `mc service restart`，runner 存活。
12. 真实 Minecraft（Paper 至少一个版本）：启动、控制台、日志、备份、恢复、更新后继续运行。

### 12.3 建议加入 CI 的自动化

- fake Java runner 的 daemon 重启/更新 E2E（已有 supervisor 测试基础）。
- 服务单元生成快照测试（macOS plist / Linux unit 文本）。
- 版本/协议/状态 schema 兼容矩阵单测。
- manifest 签名/反降级/错误用例。
- 包安装 smoke（容器里装 .deb/.rpm，`mc version`）。
- 公证/签名只在 release job 做，PR 不接触密钥。

## 13. 分阶段落地

| 阶段 | 交付 | 验收 | 规模 |
| --- | --- | --- | --- |
| Phase 0 | `config.toml` + 默认 User-Agent；`mc` 启动器；`mc version --json`；ServerInfo/协议范围；服务管理器抽象 | **已实现**：daemon 不再依赖 shell 环境；`mc` 能稳定 exec 三个二进制；launchd 真实验证 | M |
| Phase 1 | tarball + checksums + install.sh + Homebrew formula；Linux systemd 验证；版本化目录 | **核心已实现**：公开 unsigned tarball、合并 manifest、file:// 安装/卸载闭环、SHA-256 和 launcher 切换完成；Homebrew tap 发布与真实 Linux 安装仍待 | M |
| Phase 2 | macOS 签名/公证；deb/rpm + apt/dnf 仓库；minisign/SBOM；release manifest | Gatekeeper 通过；Linux 包可安装；所有产物可验证 | M-L |
| Phase 3 | runner 握手；任务排空/更新保护；健康检查 + 自动回滚；在 Phase 1 版本化目录上接入更新器 | **部分实现**：runner 握手与 API 兼容窗口已落地；状态迁移、任务排空、健康检查和自动回退仍缺 | M |
| Phase 4 | `mc update --check/--rollback`；TUI 更新提示；auto_check；包管理器来源识别；文档/支持 | **部分实现**：手工 `status/rollback` 已完成；签名 updater、自动检查、TUI 提示和包管理器来源识别仍缺 | S-M |

Phase 0/1 已提供稳定入口、服务安装、公开 unsigned 产物、版本化目录和本机安装/卸载闭环；仍需签名/公证、Homebrew tap 和 Linux 实机验证，之后才适合做自动更新。

## 14. 待定决策

1. **单二进制 vs 启动器**：Phase 1 用 `mc` 启动器，Phase 3 再评估合并；如果希望最小签名面，直接合并更彻底。
2. **Homebrew formula 从源码还是 bottle**：源码简单、无需公证；bottle 快但发布链更重。
3. **Apple Developer ID 是否具备**：没有就只做 tarball/Homebrew source + 明确 Gatekeeper 说明，不承诺“无警告安装”。
4. **Linux 基线**：musl 静态优先；deb/rpm 用 Ubuntu 22.04 glibc 基线。
5. **systemd-only 是否可接受**：首版建议是；Alpine/OpenRC 只保证手动 tarball。
6. **自动更新默认值**：建议 `auto_check=true`、`auto_install=false`。
7. **活动任务时更新策略**：默认拒绝；`--wait` 等待；`--force` 仅高级用户。
8. **状态迁移回滚**：是否要求迁移必须可逆，还是只保证“迁移前快照 + 明确不可降级”。建议后者，文档写清楚。

## 15. 参考

- [实现差距](IMPLEMENTATION_GAPS.md)
- [产品设计的发布门槛](../PRODUCT_DESIGN.md)（“从长期项目到被广泛使用的产品”）
- [监督契约](SUPERVISION.md)（runner 存活/重连/身份核验）
- [实例 schema](INSTANCE_SCHEMA.md)（v1、恢复、迁移缺口）
- [异步契约](ASYNC_CONCURRENCY.md)（任务中断/取消/进度）
- [TUI 测试](TUI_TESTING.md)（真实终端/PTY 证据）
- [Base 落地方案](../BASE_IMPLEMENTATION_PLAN.md)（服务安装是第 2 步承诺）
