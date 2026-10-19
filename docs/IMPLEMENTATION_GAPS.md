# 实现差距与发布阻塞项

> 核对日期：2026-10-19  
> 代码版本：workspace `0.1.0`  
> 协议版本：`API_PROTOCOL_VERSION = 8`  
> 核对方式：逐份阅读 `PRODUCT_DESIGN.md`、`BASE_IMPLEMENTATION_PLAN.md`、`MIRROR_ACCELERATION_DESIGN.md`、`UPSTREAM_DOWNLOAD_DESIGN.md` 和 `docs/` 契约，再对照 `crates/*` 的实际命令、API、测试和脚本。本文只记录可证实的状态；未来规划不当作已实现。

## 结论

1. **基础闭环是真实的**：Paper 创建、下载校验、断点续传缓存、持久任务、常驻 runner、启停、控制台、日志、停机世界备份/恢复、TUI 会话 shell 和 CLI 查询都已经有代码和测试。
2. **离文档定义的“第一个稳定版本”还有明显差距**：Vanilla、接管现有目录、`server.properties` 编辑、玩家管理、在线/定时备份、故障规则仍未实现；用户级服务安装完成了 Phase 0，但 Linux 实机验证和更新保护仍待补。其中 Vanilla 和接管目录属于首版承诺，不是可选项。
3. **发布与分发仍处于起步**：已有基础 CI、Phase 0 的 `mc`/配置/服务安装，以及 Phase 1 的版本化安装器/manifest；但没有公开且签名的发布产物、Homebrew tap/apt/dnf 渠道、公证和更新器，当前仍是源码构建加本机打包。
4. **热更新的关键底座已经具备，但还缺协议兼容和更新保护**：per-instance runner 会独立于 daemon 存活，daemon 重启后可重连，这是最难得的一半。缺的是 daemon/runner/状态 schema 的版本握手、更新前排空任务、原子版本目录和回滚。
5. **建议先做发布工程，而不是继续堆功能**：服务安装只到 Phase 0，更新保护还没有；功能越多，真实用户升级和排障成本越高。功能线可以并行补 Vanilla、接管目录和 `server.properties`。
6. **Phase 0 与 Phase 1 发布工具已落地**（2026-10-19）：`mc` 启动器、`config.toml`、默认 User-Agent、`version --json`、`ServerInfo` 探针、launchd/systemd 用户服务管理；以及 tar.gz + SHA-256 + release manifest、版本化 `install.sh`、Homebrew formula 渲染、`mc uninstall`。macOS launchd 已做真实安装/重启/停止/卸载闭环；Linux systemd 有 `systemd-analyze` 测试和 CI 交叉编译，但还没有真实 Linux 主机安装证据。

## 状态图例

| 标记 | 含义 |
| --- | --- |
| ✅ | 已实现，并有单元/集成/PTY/浏览器证据 |
| 🟡 | 部分实现，或只在 macOS 验证，或只覆盖正常路径 |
| ❌ | 文档有要求，但代码没有入口 |
| ⏸ | 文档明确推迟，或当前阶段不做 |

## 1. P0：基础建服与日常管理

| 文档要求 | 状态 | 证据与差距 |
| --- | --- | --- |
| Paper 创建、列表、详情、启停、控制台、增量日志 | ✅ | `ApiRequest`、`mc-supervisor`、`mc-tui` 实例详情、PTY 回归 |
| 关闭 TUI/重启 daemon 后服务器继续运行并重连 | ✅ | `runner` 独立进程、`server/supervisor.json`、`mc-supervisor` daemon-exit 重连测试 |
| 下载中断续传、长度/SHA-256 校验、缓存、原子安装 | ✅ | `mc-download`、`mc-instance` staging/quarantine、PTY/单元测试 |
| 持久任务、事件游标、字节进度、两次确认取消 | ✅ | `mc-task`、`task_presentation`、TUI 任务详情、PTY |
| 停机世界备份、列表、校验、恢复预览、精确确认、事务恢复 | ✅ | `mc-backup`、`mc-daemon/backup*`、备份管理器和 PTY |
| 真实 Minecraft 世界的备份/恢复演练 | ❌ | 文档明确仍待真实演练；当前只有 fake daemon/事务测试 |
| **Vanilla 适配器** | ❌ | 只有 `provider-paper`；没有 Vanilla manifest/version JSON 解析 |
| **接管现有服务器目录** | ❌ | 没有 scan/confirm/register API；`mc-instance` 只处理自己创建的实例 |
| 实例重命名、收藏、归档、复制配置、删除前检查 | ❌ | 实例只支持 create/list/open；没有 update/rename/delete API |
| **重启命令** | ❌ | 只有 start/stop；TUI 没有 restart；CLI 也没有生命周期命令 |
| **`server.properties` 分类编辑、原始编辑、差异预览、外部修改冲突检测** | ❌ | 现有代码只在备份时读取 `level-name` 识别世界目录 |
| JVM 参数/服务端参数/环境变量编辑器 | ❌ | 只有 `min_memory_mb`/`max_memory_mb`，没有完整启动参数模型 |
| 托管 JDK 下载与校验 | ❌ | `mc-system/java` 只扫描本机；没有 Adoptium 下载/安装 |
| Java 兼容规则按服务端与 MC 版本维护 | 🟡 | 向导优先 Java 21，其余只做“≥21”兜底；没有规则表、插件/mod 需求预检 |
| 主机内存预算与多实例资源预测 | 🟡 | 单个实例有内存档位和端口检查；状态坞显示 CPU/RSS；没有“主机总量→系统预留→所有实例 Xmx→剩余预算” |
| 基础故障规则与可复制诊断报告 | 🟡 | `doctor` 覆盖 Java、数据目录、磁盘、默认端口；没有日志规则、崩溃归因、脱敏报告导出 |
| 日志暂停、搜索、等级过滤、复制行、跳转错误上下文 | ❌ | 只有跟随/滚动、增量读取和原始日志；没有搜索/过滤/复制 |
| 在线备份（`save-off`/`save-all`/一致性标记） | ❌ | 当前契约明确是 stopped-world v1；在线备份没有实现 |
| 简单定时备份 | ❌ | 没有 scheduler；任务只能手动提交 |
| 在线玩家、白名单、封禁、OP 管理 | ❌ | 没有玩家模型或命令菜单 |
| 用户级监督服务安装/卸载/开机启动 | 🟡 | Phase 0 已实现 `mc service install/start/stop/restart/status/uninstall`；launchd 在本机做过真实闭环，systemd 生成/单测/systemd-analyze 覆盖、尚无 Linux 主机安装证据；登录启动由 RunAtLoad/enable 提供，开机前启动仍需 linger/LaunchDaemon |
| 配置文件（User-Agent、数据目录、更新通道等） | ✅ | `mc-config` 提供 `config.toml`、默认 UA、state/socket/cache 解析；env/flag 可覆盖；服务单元注入配置路径。发布前仍需把默认项目 URL 换成真实联系方式 |
| 版本化安装、卸载、发布清单 | 🟡 | `scripts/package_release.py`、`scripts/install.sh`、`mc uninstall`、Homebrew formula 模板/渲染脚本已实现；未发布 tap/签名/公证/CI 多目标合并 manifest |
| CLI 的 start/stop/logs 等日常命令 | 🟡 | CLI 有 doctor/status/paper/instance/backup/task/modpack；`status` 仍是 Ping，没有 start/stop/log/console；`mc status` 已能读取 ServerInfo |
| 关闭终端后仍可管理 | ✅ | runner 不依赖终端；安装用户服务后 daemon 由 launchd/systemd 托管，未安装时仍需手工启动 |

## 2. P1：服务端专用优势

| 文档要求 | 状态 | 证据与差距 |
| --- | --- | --- |
| Fabric 服务端与 `.mrpack` 物化导入 | ❌ | `mc-modpack` 只读分析；没有文件下载、解压、loader 安装、首启验证 |
| 显式服务端包、普通 ZIP 导入 | ❌ | 没有本地包导入/安装路径 |
| 多实例调度、端口分配、内存预算、批量维护窗口 | ❌ | 实例可独立启动，但没有调度器/主机预算/批量窗口 |
| 运行观测历史（CPU、RSS、磁盘、网络、玩家、TPS/MSPT、堆/GC） | 🟡 | 状态坞每 2 秒读一次 CPU/RSS；没有采样历史、磁盘/网络、TPS/MSPT、JVM 指标 |
| 告警（内存、磁盘、频繁重启、启动超时、卡顿） | ❌ | 没有阈值/持续时间/冷却/告警模型 |
| 服务端构建更新与回滚预览 | ❌ | 只有创建时锁定 build；没有更新计划、快照、兼容性提示 |
| 镜像源 Base 1（官方 CDN、缓存、代理、有限重试、同源续传） | 🟡 | 官方 CDN、缓存、续传、来源 UI 有；transport 明确 `.no_proxy()`；没有自动重试；元数据无缓存 |
| 镜像源 Base 2（用户自定义镜像、手动切换、哈希验收隔离） | ❌ | 只有 `fill-data.papermc.io` 精确白名单 |
| 已审核镜像目录/智能排序/局域网缓存 | ❌ | 文档明确后续 |
| 首次真实建服 + 备份恢复验收矩阵 | 🟡 | PTY/fake daemon 充分；真实 Minecraft、真实网络、真实存档演练不足 |

## 3. P2：复杂运维与差异化

| 文档要求 | 状态 | 差距 |
| --- | --- | --- |
| SSH 远程主机与本地一致视图 | ❌ | 只有 Unix socket 本机 |
| NeoForge/Forge/Quilt、插件/mod 依赖图 | ❌ | 只有只读 `.mrpack` 分析 |
| 代理服/子服编排、模板共享 | ❌ | 无 |
| 玩家行为审计、细粒度权限 | ❌ | 无 |
| 外部通知、指标输出、计划任务、远端备份 | ❌ | 无 |
| AI 助手（只读解释/提案） | ❌ | 无；产品文档也明确是后续阶段 |

## 4. 契约文档逐项对照

### `UPSTREAM_DOWNLOAD_DESIGN.md`

- ✅ Paper Fill v3 元数据、稳定构建选择、Paper 产物下载。
- ✅ 统一请求/下载契约：HTTPS、精确 host、DNS 公网校验、重定向限制、Range 续传、长度/哈希、缓存、原子提交。
- ❌ Vanilla manifest/version JSON 与历史版本无服务端产物处理。
- ❌ 托管 JDK、Fabric、`.mrpack` 同契约扩展。
- 🟡 真实 macOS/Linux 网络环境核对、来源变更台账、逐来源联系/核对日期未形成。
- ❌ 自动重试/退避；当前失败返回可重试错误，由用户重试。
- ❌ 系统代理/显式代理；`transport.rs` 使用 `.no_proxy()`。

### `MIRROR_ACCELERATION_DESIGN.md`

- 🟡 Base 1：官方 CDN、缓存、同源续传、来源展示已有；代理、有限自动重试、元数据缓存缺失。
- ❌ Base 2：自定义镜像、手动切换、哈希验收隔离、来源隔离。
- ❌ 已审核镜像目录、智能候选排序、局域网共享缓存。
- ✅ 文档保持“未审核镜像不自动回退”的安全边界；代码也没有假装支持。

### `docs/MODPACK_IMPORT.md`

- ✅ `.mrpack` 只读分析：MC 版本、loader、`env.server`、路径穿越、哈希、下载域名、server-overrides。
- ✅ daemon API `AnalyzeModpack`、CLI `modpack inspect`、TUI 选择/报告。
- ❌ 物化导入任务、依赖下载、无符号链接/穿越解压、loader 安装、回滚边界、首启验证。

### `docs/BACKUP_RESTORE_DESIGN.md`

- ✅ stopped-world 发现、空间预检、staging copy、SHA-256 manifest、分页、校验、restore plan/token、事务恢复、journal/recovery。
- ❌ 在线备份（save-off/save-all）、定时备份、远端备份、真实世界恢复演练。
- 🟡 恢复失败/断电/磁盘满等真实故障注入主要靠事务测试，缺真实主机演练记录。

### `docs/SUPERVISION.md`

- ✅ per-instance runner、私有 Unix socket、进程标记、PID 身份核验、幂等启动、优雅停止、退出码、daemon 重启重连。
- 🟡 Linux `/proc` 分支已实现，但文档要求 CI/主机验证；当前 CI 还没有 Linux 矩阵。
- ✅ launchd/systemd 用户级服务安装、状态、启停、重启、卸载（`mc service`）；macOS 已真实验证，Linux 待主机/CI 安装验证。
- 🟡 登录启动：launchd RunAtLoad、systemd `enable` 已生成；Linux 开机前启动需要 `loginctl enable-linger`，当前未自动开启。
- ❌ runner 控制协议版本握手（当前只是无版本的 JSON line；热更新需要）。
- ❌ 更新期间的任务排空/守护进程优雅交接流程。

### `docs/INSTANCE_SCHEMA.md`

- ✅ schema v1 `instance.toml`/`lock.json`/可重建 SQLite index；staging quarantine；启动重建。
- ❌ 原地配置迁移（in-place migration）与迁移测试。
- ❌ 安全任务恢复（restart 后 queued/running 变 interrupted，不自动 replay；这是当前策略，不是恢复能力）。
- ✅ 任务取消、字节进度已实现；**文档中的“仅阶段、无字节、无取消”已过期，本次修正**。

### `docs/ASYNC_CONCURRENCY.md`

- ✅ 64 连接上限、队列上限、单实例锁、spawn_blocking、超时、事件游标、任务结果保留。
- ✅ 下载字节进度、任务取消、PTY/UI 反馈。
- ❌ 安全任务恢复/断点任务重放（只有下载文件续传，不是任务恢复）。
- ✅ 本次修正了“无字节进度/无取消”的过期描述。

### `docs/INTERACTION_DESIGN.md`

- ✅ 默认会话 shell、transcript/composer/status dock、命令补全/历史/光标编辑、帮助、主题、Steve、鼠标、向导、任务/实例/备份页。
- ⏸ 镜像源选择器按安全边界明确推迟；代码没有装饰性镜像下拉。
- 🟡 首次打开没有“后台服务未运行/一键安装服务”的 setup 引导；目前直接显示连接失败。

### `docs/TUI_TESTING.md`

- ✅ 74 个 `mc-tui` 测试、4 个 PTY 回归、80×24/40×12 渲染、浏览器截图、主题/Steve 证据。
- 🟡 Linux 真实终端/Linux `/proc` 尚无 CI 证据；macOS 是主要验证环境。
- 🟡 真实 Minecraft 日志/真实世界/真实网络未纳入回归。

## 5. 文档与代码的漂移（本次已修正）

| 位置 | 旧说法 | 实际 |
| --- | --- | --- |
| `docs/ASYNC_CONCURRENCY.md` | 进度只有阶段、下载无法报告字节 | `TaskProgress.transfer` 已有下载字节/总大小/速度/来源；TUI 显示真实进度 |
| `docs/ASYNC_CONCURRENCY.md` | 取消未暴露 | `CancelTask`、`mc-task` 协作取消、TUI 两次 `x`、CLI/TUI 已有 |
| `docs/INSTANCE_SCHEMA.md` | 进度只有阶段、无取消 | 同上 |
| `docs/SUPERVISION.md` | API protocol 6 | 当前协议 8 |
| `docs/TUI_TESTING.md` | 旧测试总数 | 已更新为 workspace 164（Linux 165）/ `mc-tui` 74（2026-10-19，Phase 0/1 新增 11 项测试；Linux 的 systemd-analyze 额外 1 项） |

## 6. 发布与分发阻塞项（按严重度）

1. **服务安装只完成了 Phase 0**：`mc service` 已有 launchd/systemd 实现，但 Linux 真实安装、登录/开机启动、更新时“不杀 runner”的服务级验证还没完成。
2. **发布产物/签名仍未完成**：Phase 1 已有本机可验证的 tar.gz、SHA-256、manifest、版本化安装器和 Homebrew formula 模板；但没有公开下载、签名/公证、实际 tap/apt/dnf 仓库、SBOM、changelog，CI 也未发布 Release。
3. **没有协议兼容窗口**：TUI/CLI 与 daemon 仍精确匹配 API v8；`ServerInfo` 已能报告范围，但 daemon 不接受 N/N-1。daemon 热更新会瞬间让旧 TUI 报协议错误。
4. **没有状态迁移/降级保护**：实例 schema 仅 v1；更新后无法安全回退，缺少迁移前快照。
5. **没有更新保护**：下载/备份/恢复任务运行时重启 daemon 会把这些任务标记为 interrupted；没有“排空/拒绝/等待”策略。
6. **没有 runner 协议版本**：新 daemon 管理旧 runner 只能靠当前无版本 JSON 协议；不能承诺跨版本兼容。
7. **Linux 发布验证不足**：CI 已加 macOS/Linux 构建测试，但 `/proc` 身份核验、`ps` 依赖、musl、systemd 实际安装还没有真实主机矩阵。
8. **文档承诺的“稳定版验收”没有真实演练**：真实 Paper/Vanilla、真实存档、真实升级/回滚、干净环境安装。
9. **没有安装/升级/卸载/支持文档**：PRODUCT_DESIGN 已明确这是公开发布门槛。
10. **User-Agent 默认指向项目仓库**：这是一个真实项目联系方式；fork 或共享部署仍应设置自己的联系 URL。

## 7. 建议顺序

### 先做（发布最小闭环）

1. ~~`mc` 稳定入口 + `config.toml`~~（已完成 Phase 0）。
2. ~~`mc service install/uninstall/status/restart`~~（launchd 已真实验证；systemd 待 Linux 主机验证）。
3. ~~版本/协议 `mc version --json` 与 `ServerInfo`~~（已完成；N/N-1 兼容窗口仍待 Phase 3）。
4. ~~CI 矩阵 + tarball + 校验和 + `install.sh`~~（Phase 1 已实现本机闭环；Homebrew tap 发布/多目标合并仍待完成）。
5. 更新前任务排空、原子版本目录、服务重启、健康检查、回滚。

### 并行补功能

- Vanilla provider + 接管现有目录（首版承诺）。
- `server.properties` 编辑 + Java 兼容规则 + 基础故障规则。
- 玩家管理 + 在线/定时备份。
- 镜像 Base 2、Fabric/.mrpack 物化导入。

### 发布门槛

- 在干净的 macOS 与 Linux 用户环境完成安装 → 建服 → 重启 daemon → 备份 → 恢复 → 热更新 → 回滚。
- 更新期间已运行 Java 进程 PID 不变，日志/控制台可继续。
- 卸载默认保留实例数据；`--purge` 必须二次确认。
- 真实 Paper/Vanilla 各一个实例，真实世界备份恢复一次。

---

分发的具体架构、macOS/Linux 打包、服务单元、签名和热更新流程见 [DISTRIBUTION.md](DISTRIBUTION.md)。
