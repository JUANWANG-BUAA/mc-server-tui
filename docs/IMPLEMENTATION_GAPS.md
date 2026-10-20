# 实现差距与发布阻塞项

> 核对日期：2026-10-20
> 代码版本：workspace `0.2.0`
> 协议版本：`API_PROTOCOL_MIN..=API_PROTOCOL_MAX = 8..=11`（runner 协议 v1）
> 核对方式：逐份阅读 `PRODUCT_DESIGN.md`、`BASE_IMPLEMENTATION_PLAN.md`、`MIRROR_ACCELERATION_DESIGN.md`、`UPSTREAM_DOWNLOAD_DESIGN.md` 和 `docs/` 契约，再对照 `crates/*` 的实际命令、API、测试和脚本。本文只记录可证实的状态；未来规划不当作已实现。

## 结论

1. **基础闭环是真实的**：Paper 创建、下载校验、断点续传缓存、持久任务、常驻 runner、启停、控制台、日志、停机世界备份/恢复、TUI 会话 shell 和 CLI 查询都已经有代码和测试。
2. **离文档定义的“第一个稳定版本”还有明显差距**：Vanilla、接管现有目录、完整 `server.properties` 管理、在线/定时备份和故障规则仍未完成；玩家管理、日志搜索、Fabric 建服、整合包导入、四字段属性编辑和受控重启已经落地。属性编辑器目前只覆盖 MOTD、人数上限、难度和视距，不能算完整的分类/原始编辑能力。用户级服务安装完成了 Phase 0，但 Linux 实机服务验证和更新保护仍待补。
3. **发布与分发仍处于起步**：已有基础 CI、Phase 0 的 `mc`/配置/服务安装，以及 Phase 1 的版本化安装器/manifest，并已公开 unsigned alpha 产物；但没有签名/公证、Homebrew tap/apt/dnf 渠道、合并后的多目标发布和自动更新器。
4. **热更新的关键底座已经具备，但仍缺状态迁移和更新保护**：per-instance runner 会独立于 daemon 存活，daemon 重启后可重连；已实现 API 8–11 兼容窗口、runner 控制协议 v1 和 `mc update rollback`。仍缺状态 schema 迁移快照、更新前排空任务、签名自动更新器和健康检查式自动回退。
5. **路线图需要显式选择**：较早的发布规划优先完成签名/包管理和更新保护；2026-10-19 产品评审则建议先验证一条完整 Paper 朋友服旅程，再把属性编辑和启动诊断做到可用，并在真实客户端验收受控重启。两条路线都成立，但当前文档没有正式决定哪个是下一里程碑，不能同时宣称二者都是“下一步优先项”。
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
| 真实 Minecraft 世界的备份/恢复演练 | 🟡 | 2026-10-19 本机 Paper 演练实际完成停服备份、校验、恢复、再次启动和目录树检查；没有 Minecraft 客户端进入恢复后的世界，玩家可见内容仍未核验 |
| **Vanilla 适配器** | ❌ | 只有 `provider-paper`；没有 Vanilla manifest/version JSON 解析 |
| **接管现有服务器目录** | ❌ | 没有 scan/confirm/register API；`mc-instance` 只处理自己创建的实例 |
| 实例重命名、收藏、归档、复制配置、删除前检查 | ❌ | 实例只支持 create/list/open；没有 update/rename/delete API |
| **重启命令** | ✅ | 协议 v11 `RestartServer`、TUI `/restart`/`R`、CLI `instance restart`；同一协调锁覆盖优雅停服等待和再次启动，停止不干净时不会自动启动 |
| **`server.properties` 分类编辑、原始编辑、差异预览、外部修改冲突检测** | 🟡 | `/config` 已实现 MOTD、人数上限、难度、视距四字段、范围校验、差异预览、SHA-256 修订冲突检测、未知键/注释保留、原子保存和 `/restart` 后续应用；缺完整分类字段、原始编辑和逐版本默认值/规则 |
| JVM 参数/服务端参数/环境变量编辑器 | ❌ | 只有 `min_memory_mb`/`max_memory_mb`，没有完整启动参数模型 |
| 托管 JDK 下载与校验 | ❌ | `mc-system/java` 只扫描本机；没有 Adoptium 下载/安装 |
| Java 兼容规则按服务端与 MC 版本维护 | 🟡 | 向导优先 Java 21，其余只做“≥21”兜底；没有规则表、插件/mod 需求预检 |
| 主机内存预算与多实例资源预测 | 🟡 | 单个实例有内存档位和端口检查；状态坞显示 CPU/RSS；没有“主机总量→系统预留→所有实例 Xmx→剩余预算” |
| 基础故障规则与可复制诊断报告 | 🟡 | `doctor` 覆盖 Java、数据目录、磁盘、默认端口；启动失败已识别 Java 版本、端口占用、EULA、内存不足和损坏/缺失 `server.jar`；仍缺完整日志规则、崩溃归因和脱敏报告导出 |
| 多页面工作区、稳定编号、日志过滤/位置保留 | ✅ | 顶栏页面列表、Ctrl-B 导航、`/pages`/`/page`/`/close`；关闭页面不停止服务端；实例日志过滤和读取位置随页面保留 |
| 日志暂停、搜索、等级过滤、复制行、跳转错误上下文 | 🟡 | 已实现级别着色、实时 `f` 过滤、横向滚动、滚动条跳转和 `/rg`（当前/上一份轮转日志，`-i`/`-e`）；仍缺暂停缓冲、复制行和错误上下文跳转 |
| 在线备份（`save-off`/`save-all`/一致性标记） | ❌ | 当前契约明确是 stopped-world v1；在线备份没有实现 |
| 简单定时备份 | ❌ | 没有 scheduler；任务只能手动提交 |
| 在线玩家、白名单、封禁、OP 管理 | 🟡 | `/players` 通过类型化控制台命令完成 list/op/deop/ban/pardon/kick/whitelist/LuckPerms；在线模式按玩家名操作依赖 Mojang 档案查询，国内网络下可能慢或失败；CLI 尚未暴露这些操作 |
| 用户级监督服务安装/卸载/开机启动 | 🟡 | Phase 0 已实现 `mc service install/start/stop/restart/status/uninstall`；launchd 在本机做过真实闭环，systemd 生成/单测/systemd-analyze 覆盖、尚无 Linux 主机安装证据；登录启动由 RunAtLoad/enable 提供，开机前启动仍需 linger/LaunchDaemon |
| 配置文件（User-Agent、数据目录、更新通道等） | ✅ | `mc-config` 提供 `config.toml`、默认 UA、state/socket/cache 解析；env/flag 可覆盖；服务单元注入配置路径。发布前仍需把默认项目 URL 换成真实联系方式 |
| 版本化安装、卸载、发布清单 | 🟡 | `scripts/package_release.py`、`scripts/install.sh`、`mc uninstall`、Homebrew formula 模板/渲染脚本已实现；本轮新增 `mc update status/rollback`，切换 `current`/`previous` 并重启服务；仍缺签名/公证、tap、CI 多目标合并 manifest 和更新前排空任务 |
| CLI 的 start/stop/logs 等日常命令 | 🟡 | CLI 已有 `instance start/stop/restart`，以及 doctor/status/paper/instance/backup/task/modpack；`status` 仍是 Ping，没有 log/console/config；玩家、日志、扩展、整合包和属性编辑只走 TUI |
| 关闭终端后仍可管理 | ✅ | runner 不依赖终端；安装用户服务后 daemon 由 launchd/systemd 托管，未安装时仍需手工启动 |

## 2. P1：服务端专用优势

| 文档要求 | 状态 | 证据与差距 |
| --- | --- | --- |
| Fabric 服务端与 `.mrpack` 物化导入 | 🟡 | `/fabric` 命令建服、Modrinth 扩展下载与必需依赖、MRPACK/ZIP 预览和停服导入已实现；Forge/NeoForge、CurseForge 文件 ID 授权下载和任意第三方依赖仍未实现 |
| 显式服务端包、普通 ZIP 导入 | ✅ | `/import` 支持 ZIP/MRPACK 预览与 `--apply`，限制 10000 项/512 MiB/单文件 256 MiB，拒绝越界路径和符号链接，不覆盖世界与身份数据 |
| 多实例调度、端口分配、内存预算、批量维护窗口 | ❌ | 实例可独立启动，创建时会跳过已占用端口；没有调度器/主机预算/批量窗口 |
| 运行观测历史（CPU、RSS、磁盘、网络、玩家、TPS/MSPT、堆/GC） | 🟡 | 状态坞每 2 秒读一次 CPU/RSS，保留最近 60 次采样趋势；没有磁盘/网络、TPS/MSPT、JVM 指标 |
| 告警（内存、磁盘、频繁重启、启动超时、卡顿） | 🟡 | 已实现异常退出、新 ERROR/Exception、单核 90% 持续 6 次采样、RSS 连续 3 次超过 Xmx；不会自动重启或杀进程；仍缺磁盘、启动超时、卡顿和冷却模型 |
| 服务端构建更新与回滚预览 | ❌ | 只有创建时锁定 build；没有更新计划、快照、兼容性提示 |
| 镜像源 Base 1（官方 CDN、缓存、代理、有限重试、同源续传） | 🟡 | 官方 CDN、缓存、续传、来源 UI 有；transport 明确 `.no_proxy()`；没有自动重试；元数据无缓存 |
| 镜像源 Base 2（用户自定义镜像、手动切换、哈希验收隔离） | ❌ | 只有 `fill-data.papermc.io` 精确白名单 |
| 已审核镜像目录/智能排序/局域网缓存 | ❌ | 文档明确后续 |
| 首次真实建服 + 重启/备份恢复验收矩阵 | 🟡 | 真实 Paper 1.21.11 已完成建服、运行、受控重启（PID 3482 → 3565）、再次就绪和退出码 0 停服；仍缺客户端进入世界、真实网络与完整备份恢复矩阵 |

## 3. P2：复杂运维与差异化

| 文档要求 | 状态 | 差距 |
| --- | --- | --- |
| SSH 远程主机与本地一致视图 | ❌ | 只有 Unix socket 本机 |
| NeoForge/Forge/Quilt、插件/mod 依赖图 | 🟡 | Fabric 已解析目录中的必需依赖并识别 JAR 内的 Fabric API 模块；Forge/NeoForge/Quilt 和任意第三方依赖图未实现 |
| 代理服/子服编排、模板共享 | ❌ | 无 |
| 玩家行为审计、细粒度权限 | ❌ | 无 |
| 外部通知、指标输出、计划任务、远端备份 | ❌ | 无 |
| AI 助手（只读解释/提案） | ❌ | 无；产品文档也明确是后续阶段 |

## 4. 契约文档逐项对照

### `UPSTREAM_DOWNLOAD_DESIGN.md`

- ✅ Paper Fill v3 元数据、稳定构建选择、Paper 产物下载。
- ✅ 统一请求/下载契约：HTTPS、精确 host、DNS 公网校验、重定向限制、Range 续传、长度/哈希、缓存、原子提交。
- ❌ Vanilla manifest/version JSON 与历史版本无服务端产物处理。
- 🟡 Fabric 元数据、Modrinth 版本/依赖和 `.mrpack` 下载已复用 HTTPS、精确 host、大小与 SHA-512 校验；托管 JDK 仍未实现。
- 🟡 真实 macOS/Linux 网络环境核对、来源变更台账、逐来源联系/核对日期未形成。
- ❌ 自动重试/退避；当前失败返回可重试错误，由用户重试。国内网络下 Modrinth/Fabric 没有镜像或代理回退。
- ❌ 系统代理/显式代理；`transport.rs` 使用 `.no_proxy()`。

### `MIRROR_ACCELERATION_DESIGN.md`

- 🟡 Base 1：官方 CDN、缓存、同源续传、来源展示已有；代理、有限自动重试、元数据缓存缺失。
- ❌ Base 2：自定义镜像、手动切换、哈希验收隔离、来源隔离。
- ❌ 已审核镜像目录、智能候选排序、局域网共享缓存。
- ✅ 文档保持“未审核镜像不自动回退”的安全边界；代码也没有假装支持。

### `docs/MODPACK_IMPORT.md`

- ✅ `.mrpack` 分析：MC 版本、loader、`env.server`、路径穿越、哈希、下载域名、server-overrides。
- ✅ daemon API `AnalyzeModpack`、CLI `modpack inspect`、TUI 选择/报告。
- ✅ ZIP/MRPACK 停服导入：暂存、校验、相同内容跳过、冲突拒绝、不覆盖世界/身份数据/server.jar/server.properties、不执行包内脚本；支持 `overrides`/`server-overrides` 优先级。
- ❌ 非 Fabric loader 安装（Forge/NeoForge）、CurseForge 文件 ID 下载、导入后首启验证和自动回滚。

### `docs/BACKUP_RESTORE_DESIGN.md`

- ✅ stopped-world 发现、空间预检、staging copy、SHA-256 manifest、分页、校验、restore plan/token、事务恢复、journal/recovery。
- ❌ 在线备份（save-off/save-all）、定时备份、远端备份、恢复后客户端世界核验。
- 🟡 恢复失败/断电/磁盘满等真实故障注入主要靠事务测试，缺真实主机演练记录。

### `docs/SUPERVISION.md`

- ✅ per-instance runner、私有 Unix socket、进程标记、PID 身份核验、幂等启动、优雅停止、退出码、daemon 重启重连。
- 🟡 Linux `/proc` 分支已在 Ubuntu CI 的 workspace 测试覆盖；仍缺真实 Linux 主机上的服务安装、终端交互和完整维护演练。
- ✅ launchd/systemd 用户级服务安装、状态、启停、重启、卸载（`mc service`）；macOS 已真实验证，Linux 待主机/CI 安装验证。
- 🟡 登录启动：launchd RunAtLoad、systemd `enable` 已生成；Linux 开机前启动需要 `loginctl enable-linger`，当前未自动开启。
- ✅ runner 控制协议 v1：请求/响应携带 `protocol`，缺失字段按 v1 处理；新 daemon 拒绝高于自身版本的请求。
- ✅ API 兼容窗口：daemon 接受协议 8–11，回包使用请求版本；旧版请求看不到高于自身版本的任务结果。v10 属性编辑和 v11 受控重启按协商版本拒绝。
- ❌ 更新期间的任务排空/守护进程优雅交接流程；状态 schema 迁移仍无快照。

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

- ✅ 89 个 `mc-tui` 测试、4 个 PTY 回归、80×24/40×12 渲染、浏览器截图、主题/Steve 证据；受控重启已纳入实例详情 PTY，属性编辑仍只有状态/渲染测试。
- 🟡 CI 在 Ubuntu 运行 workspace 测试和一项 PTY smoke，但 Linux 真实主机服务安装、终端手工验收和完整维护演练仍缺。
- 🟡 真实 Minecraft 日志/真实世界/真实网络未纳入回归。

## 5. 文档与代码的漂移（本次已修正）

| 位置 | 旧说法 | 实际 |
| --- | --- | --- |
| `docs/ASYNC_CONCURRENCY.md` | 进度只有阶段、下载无法报告字节 | `TaskProgress.transfer` 已有下载字节/总大小/速度/来源；TUI 显示真实进度 |
| `docs/ASYNC_CONCURRENCY.md` | 取消未暴露 | `CancelTask`、`mc-task` 协作取消、TUI 两次 `x`、CLI/TUI 已有 |
| `docs/INSTANCE_SCHEMA.md` | 进度只有阶段、无取消 | 同上 |
| `docs/SUPERVISION.md` | API protocol 8–9（current 9） | 当前协议范围 8–11（current 11），runner 协议 v1 |
| `docs/TUI_TESTING.md`、`README*` | 旧测试总数 | 已更新为 macOS workspace 217 / `mc-tui` 91（2026-10-20 本地彩色测试运行） |
| `README*`、`docs/FAQ.md`、`docs/MODPACK_IMPORT.md` | `.mrpack` 只读、只有 Paper | 已实现 Paper/Fabric、ZIP/MRPACK 停服导入、`/players` 类型化操作和 Modrinth 扩展安装 |
| `docs/IMPLEMENTATION_GAPS.md`、`docs/INTERACTION_DESIGN.md` | 没有属性编辑入口 | `/config` 已提供四字段编辑、差异检查、冲突检测和原子保存；完整属性目录和原始编辑仍待补 |
| `README*`、`docs/INTERACTION_DESIGN.md` | 只有 start/stop、启动失败只显示通用错误 | 已加入受控重启、TUI `/restart`/`R`、CLI 生命周期命令和五类启动失败诊断 |

## 6. 发布与分发阻塞项（按严重度）

1. **服务安装只完成了 Phase 0**：`mc service` 已有 launchd/systemd 实现，但 Linux 真实安装、登录/开机启动、更新时“不杀 runner”的服务级验证还没完成。
2. **发布产物已有，但签名/渠道仍未完成**：`v0.2.0` 作为 unsigned alpha pre-release 发布 Apple Silicon 与 Linux x86_64/arm64 的 tar.gz、SHA-256、manifest、install.sh 和 Homebrew formula；仍没有签名/公证、实际 tap/apt/dnf 仓库和 SBOM。
3. ~~没有协议兼容窗口~~：已实现 API 8–11 协商、按请求版本回包和旧版本任务过滤；runner 控制协议为 v1。仍需自动更新器在切换前后做协议/状态检查。
4. **没有状态迁移/降级保护**：实例 schema 仅 v1；更新后无法安全回退，缺少迁移前快照。
5. **没有更新保护**：下载/备份/恢复任务运行时重启 daemon 会把这些任务标记为 interrupted；没有“排空/拒绝/等待”策略。
6. ~~没有 runner 协议版本~~：runner 控制协议 v1 已落地；旧帧缺少 `protocol` 时按 v1 处理，新 runner 拒绝更高版本请求。
7. **Linux 发布验证不足**：CI 已加 macOS/Linux 构建测试，但 `/proc` 身份核验、`ps` 依赖、musl、systemd 实际安装还没有真实主机矩阵。
8. **稳定版验收矩阵仍不完整**：本机 Paper 已完成建服、备份、恢复和再次启动，但缺客户端进入世界、Vanilla、真实升级/回滚和干净 Linux 环境安装。
9. **安装/升级/卸载文档仍分散**：README、FAQ 和签名前的发布说明覆盖部分路径，但缺面向稳定版的统一支持矩阵、升级/回退说明和故障排查手册。
10. **User-Agent 默认指向项目仓库**：这是一个真实项目联系方式；fork 或共享部署仍应设置自己的联系 URL。

## 7. 建议顺序

### 先做（发布最小闭环）

1. ~~`mc` 稳定入口 + `config.toml`~~（已完成 Phase 0）。
2. ~~`mc service install/uninstall/status/restart`~~（launchd 已真实验证；systemd 待 Linux 主机验证）。
3. ~~版本/协议 `mc version --json` 与 `ServerInfo`~~（已完成；API 8–11 协商与 runner v1 握手已落地）。
4. ~~CI 矩阵 + tarball + 校验和 + `install.sh`~~（Phase 1 已实现本机闭环；Homebrew tap 发布/多目标合并仍待完成）。
5. ~~`mc update status/rollback`~~（已完成 symlink 切换 + 服务重启）；仍需更新前任务排空、状态迁移快照、签名 updater、健康检查自动回退。

### 并行补功能

- Vanilla provider + 接管现有目录（首版承诺）。
- 完整 `server.properties` 分类/原始编辑 + Java 兼容规则 + 基础故障规则。
- 在线/定时备份；玩家操作的 CLI 接入和结构化结果。
- 镜像 Base 2、Forge/NeoForge 与 CurseForge 导入。

### 发布门槛

- 在干净的 macOS 与 Linux 用户环境完成安装 → 建服 → 重启 daemon → 备份 → 恢复 → 热更新 → 回滚。
- 更新期间已运行 Java 进程 PID 不变，日志/控制台可继续。
- 卸载默认保留实例数据；`--purge` 必须二次确认。
- 真实 Paper/Vanilla 各一个实例，真实世界备份恢复一次。

---

分发的具体架构、macOS/Linux 打包、服务单元、签名和热更新流程见 [DISTRIBUTION.md](DISTRIBUTION.md)。
