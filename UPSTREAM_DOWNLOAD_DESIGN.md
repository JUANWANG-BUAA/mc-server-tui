# 上游数据与下载设计

> 版本：0.2 · 2026-10-17 · 对应 [产品设计](PRODUCT_DESIGN.md) 与 [Base 版落地方案](BASE_IMPLEMENTATION_PLAN.md)

镜像加速、手动切换与候选源排序的细节见 [镜像源与下载加速设计](MIRROR_ACCELERATION_DESIGN.md)。

本文是下载模块的实现契约。首个可交付闭环只接 **Paper 服务端**；随后接 Vanilla。托管 Java 是可选能力，优先让用户选择已安装的 Java。Fabric 和整合包来源先预留模型，待基础下载链稳定后实现。

## 1. 来源边界与选择

| 产物 | 元数据权威来源 | 实际二进制来源 | 校验依据 | 阶段 |
| --- | --- | --- | --- | --- |
| Paper 服务端 JAR | [PaperMC Fill v3](https://docs.papermc.io/misc/downloads-service/) 的 `GET https://fill.papermc.io/v3/projects/paper` 与 `GET /v3/projects/paper/versions/{mc_version}/builds` | 构建对象的 `downloads["server:default"].url`，通常为 `fill-data.papermc.io` | 同一下载对象的 `checksums.sha256` 和 `size` | Base 首个适配器 |
| Vanilla 服务端 JAR | Mojang 的 [`version_manifest_v2.json`](https://piston-meta.mojang.com/mc/game/version_manifest_v2.json) → 指定版本的 JSON | 版本 JSON 的 `downloads.server.url`，通常为 `piston-data.mojang.com` | 清单条目的版本 JSON SHA-1，再核对 `downloads.server.sha1` 与 `size` | Base 第二适配器 |
| Eclipse Temurin JDK | [Adoptium API v3](https://api.adoptium.net/) 的 GA 发行信息，按 Java 主版本、OS、架构、`jdk` 精确查询 | API 给出的包链接；官方[下载与校验说明](https://adoptium.net/installation/ci-scripts/)也提供稳定的 `/v3/binary/latest/...` 路径 | 官方 SHA-256；如做签名验证，明确维护可信公钥 | 后续可选托管 Java |
| Fabric 服务端 | [Fabric Meta v2](https://github.com/FabricMC/fabric-meta/blob/master/README.md) 的游戏、loader、installer 版本信息 | Fabric 官方服务端安装器或官方提供的服务端 JAR 接口 | 需要核对具体安装链每一层的哈希；未满足前不承诺全自动安装 | 下一阶段 |
| `.mrpack` 内的模组文件 | 用户导入包内 `modrinth.index.json`；后续可结合 Modrinth 项目元数据 | 文件条目的 HTTPS `downloads[]`，逐个应用来源策略 | 文件条目必须包含 SHA-1 与 SHA-512，优先核对 SHA-512 | 下一阶段 |

来源地址是**解析入口**，不能把当前返回的二进制 URL 当永久 API。每次新建或升级先从权威元数据解析**用户选择的准确版本与构建**，然后锁定产物。已安装实例不会因为上游出现新版本而自动改用新 JAR。客户端 JAR、光影和资源包不属于基础服务端下载范围。

### 界面里其他数据从哪里来

| 数据 | 主要来源 | 呈现规则 |
| --- | --- | --- |
| 可选 MC 版本、Paper 构建、文件体积和校验值 | 对应上游元数据 | 显示来源和获取时间；过期缓存要有标记 |
| Java 兼容提示 | 各服务端官方要求文档 + 本项目版本化规则表 | 显示“规则更新日期”；不把推测写成官方硬性要求 |
| `server.properties` 字段解释与默认值 | Minecraft/Paper 官方文档 + 本项目按版本维护的字段库 | 未收录字段保留原值并进入原始编辑，不擅自删除 |
| CPU、RSS、磁盘、端口、进程状态 | 本机操作系统与监督服务 | 标明采样时间；RSS、JVM 堆、TPS/MSPT 分开展示 |
| 玩家、就绪事件、报错和崩溃分析 | 服务端控制台、日志、崩溃报告 | 分清“原始证据”和“规则/AI 的解释”；给出可跳转的证据行 |
| AI 建议 | 用户明确启用后的模型或本地 harness | 输入范围可预览；建议仅生成待审核计划，来源不能伪装成官方结论 |

Base 的建服流程不依赖第三方聚合版本 API。未来若引入插件或模组搜索，须为平台 API 单独记录授权、限流、来源链和撤回方式，不直接复用“官方服务端 JAR 已验证”的标签。

上述 Mojang 清单和版本 JSON 的字段已用实际响应核对。Mojang 的 SHA-1 属于上游提供的完整性字段，不能宣传成抗恶意篡改的强签名；下载链同时依赖 HTTPS、限定来源，并在本地另算 SHA-256 供缓存识别。Paper 的字段和 User-Agent 要求来自官方文档。Adoptium/Fabric 的具体安装链在适配器开工时还需核对实际响应、重定向和产物校验字段。

## 2. Base 版解析流程

### Paper

1. 请求项目清单，只展示其中确实可解析的 MC 版本；版本字符串作为字符串处理，不假设 Minecraft 永远采用三段语义版本。
2. 请求所选版本的构建列表。默认仅提供 `channel == STABLE` 且存在 `downloads["server:default"]`、`checksums.sha256`、合理大小的构建。实验构建放在显式高级入口，显示上游风险说明。
3. 用户看到 `Paper / MC 版本 / 构建 ID / 通道 / 文件大小 / 来源`，选择后生成不可变下载计划。若该 MC 版本没有稳定构建，显示“此版本暂无稳定 Paper 构建”，由用户重新选择；**不私自降到另一个 MC 版本**。
4. 所有 Fill 请求使用标识产品名称、版本以及项目联系地址的真实 `User-Agent`；不能以 `curl`、库默认 UA 或假借别的项目身份请求。项目公开地址定下之前，开发构建用开发者控制的真实联系地址。
5. 下载时验证大小和 SHA-256；构建 ID、通道与哈希写入实例锁文件。后续“检查更新”只生成候选与差异，不替换运行中的构建。[Paper 官方明确不推荐生产环境使用自动更新器或不稳定构建](https://docs.papermc.io/misc/downloads-service/)。

### Vanilla

1. 读取 Mojang v2 版本清单，默认只列 `type == release`；snapshot 有独立入口并标明风险。
2. 用户选定 `id` 后，下载该条目的版本 JSON，先校验清单给出的 SHA-1；再读取 `downloads.server`。缺少 `server` 的历史版本标为不可安装，不试探客户端 JAR。
3. 下载服务端 JAR，验证 `size` 和 SHA-1，并算本地 SHA-256；写入版本 ID、清单 URL、版本 JSON URL 和两个哈希。
4. 保留用户主动导入本地 JAR 的独立路径，但标明“本地文件、来源由用户确认”；不能伪装成官方已验证下载。

### Java

Base 先扫描系统已有 Java，执行 `java -version` 与架构检查，按实例绑定精确可执行文件路径。兼容规则应是带来源、更新时间和适用范围的数据，不能用“最新 Java 一定兼容”代替。Paper 的[当前要求表](https://docs.papermc.io/paper/getting-started/)可作为 Paper 规则来源；其他服务端各自建规则。

托管 JDK 开工时，使用 Adoptium GA 元数据挑选明确的**发行版本、OS、架构、JDK 类型和镜像链接**，再锁定 SHA-256，不把 `/latest/` 当作实例的持久身份。只安装进本工具数据目录，不修改系统 Java 或全局 `PATH`。解压时执行路径与符号链接检查，安装后再次运行 `java -version`。若目标平台无对应资产，解释并引导使用本机 Java。

## 3. 统一请求与下载契约

```text
Provider.resolve(selection)
  → ArtifactPlan { provider, exact_version, build, channel, kind,
                   metadata_origin, download_url, expected_size,
                   upstream_hash, allowed_redirects, resolved_at }
DownloadManager.fetch(plan)
  → VerifiedBlob { cache_key_sha256, size, upstream_hash_verified }
Installer.stage(blob, instance_plan)
  → 原子提交实例文件与 lock.json
```

`resolve` 只读、可取消；`fetch` 不改实例；`stage` 在独立临时目录进行。任一步失败都不把实例标成“已安装”。UI/CLI 共用同一任务与错误模型。

### HTTP 策略

- HTTPS 且启用标准证书验证。元数据入口按 provider 固定；下载 URL 必须由可信元数据返回。拒绝 `file:`、明文 HTTP、内网/本机地址、带用户信息的 URL；重定向每一跳重新校验 scheme、主机和地址，限制跳数与目标域名。Adoptium 的实际 CDN/GitHub 重定向域名要核对后写入来源配置。
- 元数据请求采用短连接与总超时，二进制采用连接超时、空闲超时和总耗时上限；流式写入、设响应体大小上限，不把 JAR 整个读入内存。限并发和每来源请求速率。
- 对 429、临时 5xx、连接中断按有限次数退避重试，遵守 `Retry-After`；对 400/401/403/404、数据结构不符、哈希错误不盲目循环。403 可能是来源政策或 UA 问题，显示可诊断信息，不自动切换未知镜像。
- 条件请求仅在上游支持时用 `ETag`/`Last-Modified`。缓存的元数据有获取时间和过期状态；在线解析失败可展示旧列表并标注“缓存，无法保证仍可下载”。执行下载前重新确认**同一精确产物**；不能因刷新而换成新 build。
- `.part` 文件记录 URL、期望大小、上游哈希和 ETag/Last-Modified。仅在服务端支持范围请求、身份未变且返回正确 `206`/`Content-Range` 时续传；否则从头下载。取消任务保留可识别的安全临时文件，后台清理按年龄与容量执行。
- 先校验长度与上游哈希，再计算本地 SHA-256，以 `sha256` 内容地址进入共享缓存；缓存命中也复核文件仍存在、大小与哈希。安装采用复制或安全硬链接到暂存区，写入并同步锁文件后原子提交；不依赖缓存未来永远存在。
- 下载前检查预期产物、临时文件和安装目标所需空间；未知大小使用保守上限并显示估计。元数据和文件解析有大小及深度限制，避免异常响应拖垮后台服务。

下载源的域名限制不能只匹配字符串后缀；需解析 URL 和 DNS 地址，防止伪装域名及重定向到私网。对 `.mrpack` 中作者提供的链接，采用单独更严格的规则，符合[Modrinth 格式规范](https://support.modrinth.com/en/articles/8802351-modrinth-modpack-format-mrpack)的 HTTPS、哈希与允许域名要求；不把包内 URL 当成与官方服务端元数据同等可信。

## 4. 锁文件、可追溯性与界面

`lock.json` 最少记录：`schema_version`、provider ID、项目、MC 版本、构建/发行 ID、通道、产物类型、解析元数据地址、解析时间、上游哈希算法与值、预期大小、本地 SHA-256，以及安装时使用的 Java 绑定和兼容规则版本。下载 URL 可作为审计字段，但更新或重试时必须重新解析，不能假定签名/CDN URL 永久有效。锁文件写入采取临时文件与原子替换；实例配置与锁文件的提交状态要一致。

新建页展示“来源 → 版本/构建 → 验证 → 安装位置”，计划预览列出精确来源、体积、Java 要求和磁盘占用。下载阶段依次显示 `解析来源 / 等待下载 / 下载中 / 校验中 / 安装中 / 完成`，给出速率与进度；不知道总大小时显示已下载量。失败卡片展示来源、HTTP 状态或错误码、是否可重试、缓存是否可用、下一步操作。长期运行时上游失联只影响新安装和更新，不影响已有服务器启停。

建议错误码：`SOURCE_UNAVAILABLE`、`SOURCE_RATE_LIMITED`、`SOURCE_SCHEMA_CHANGED`、`NO_STABLE_BUILD`、`SERVER_ARTIFACT_MISSING`、`UNSUPPORTED_PLATFORM`、`DOWNLOAD_INTERRUPTED`、`DOWNLOAD_TOO_LARGE`、`HASH_MISMATCH`、`DISK_SPACE_LOW`、`REDIRECT_BLOCKED`。日志保留请求 ID、provider、endpoint 类别、HTTP 状态、耗时、重试次数与最终校验结果；脱敏 URL 查询串、令牌及用户目录。用户可导出不含敏感信息的来源诊断。

## 5. 不直接接入的来源

- **未经审核的第三方镜像**：不作为默认源，也不在官方源失败时静默切换。用户自定义镜像及已审核来源的切换策略见[镜像源与下载加速设计](MIRROR_ACCELERATION_DESIGN.md)；官方哈希可用时仍以官方哈希验证。
- **网页抓取**：不把下载页 HTML 当稳定 API；来源接口变化时明确报错并更新适配器。
- **CurseForge 平台包**：API 凭证、分发权限和文件可下载性需要在接入前逐项核实。Base 不承诺直接搜索或拉取 CurseForge 文件；用户已有的服务端包可走本地导入。
- **未知来源自动回退**：官方源报错时不自动从论坛、网盘或不明 CDN 拉取同名 JAR。

## 6. 开发验收顺序

1. Paper 元数据解析和稳定构建选择；用锁定的 fixtures 覆盖“无稳定构建、缺少服务端产物、字段变化”。
2. 统一下载器的中断、续传、重定向、长度/哈希校验、缓存和原子安装契约。
3. 接 Vanilla 清单与版本 JSON；处理历史版本无服务端下载项。
4. 在真实 macOS/Linux 网络环境核对实际响应、User-Agent、限流、CDN 重定向和错误文案，并记录来源文档核对日期。
5. 已安装 Java 的兼容预检；托管 JDK、Fabric、`.mrpack` 按同一来源契约逐项扩展。

上线前需建立来源变更台账：`provider / API 文档 / 字段映射 / 联系要求 / 最后核对日期 / 兼容 fixtures / 故障处置人`。上游格式变化时，优先维持旧实例可运行与可备份；新建/更新报出清楚的来源错误，发布适配器修复后再恢复。
