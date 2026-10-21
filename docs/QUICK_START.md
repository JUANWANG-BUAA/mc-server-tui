# 快速开始

适用于 v0.2.2 未签名 alpha。重要世界请保留独立备份。
完整操作见[中文手册](USER_GUIDE.zh-CN.md)，英文见 [User guide](USER_GUIDE.md)。

## 安装与初始化

发布产物支持 macOS 13+ Apple Silicon、Linux x86_64/arm64；本版本不发布 Intel macOS 产物。
先安装与游戏版本兼容的 Java：1.21.x 使用 Java 21，26.x 使用 Java 25+。
建议终端至少 80×24。首次下载需要访问上游网络。

```sh
TAG=v0.2.2
curl -fsSL "https://github.com/JUANWANG-BUAA/mc-server-tui/releases/download/$TAG/install.sh" | sh
mc setup
mc
```

找不到 `mc` 时，在当前终端执行 `export PATH="$HOME/.local/bin:$PATH"`，
或使用 `~/.local/bin/mc setup` 和 `~/.local/bin/mc`。
平台细节见 [macOS](installation/macos.md) / [Linux](installation/linux.md)。
配置与数据路径以 `mc setup --dry-run` 和 `mc doctor --json` 输出为准。

不安装后台用户服务时，先执行 `mc setup --no-service`，再分别在两个终端运行
`mc daemon run` 和 `mc tui`。

源码构建使用仓库固定的 Rust 工具链，运行 `cargo build --workspace --release --locked`，
然后使用 `./target/release/mc setup`、`./target/release/mc`。

## 创建服务器

默认界面是会话 shell。输入以下命令并按 Enter 打开 **Paper** 向导：

```text
/create
```

依次选择实例名、精确游戏版本，确认检测到的 Java、内存、端口，最后明确接受 EULA。
运行设置默认显示摘要，F3 展开编辑。应用检测本机 Java；Paper 向导不会自动安装 Java。
提交后用 `/tasks` 查看下载与安装；完成后用 `/instances`、`/use <序号或名称>` 选择实例。

Fabric 使用独立命令，不在 Paper 向导中。阅读 Minecraft EULA 并同意后执行：

```text
/fabric MyFabricServer 1.21.11 --accept-eula
```

该路径会解析稳定 Loader 并准备兼容 Java。`/import` 检查 ZIP/MRPACK；
检查通过不等于已导入，具体应用条件见[整合包说明](MODPACK_IMPORT.md)。

## 启动与连接

```text
/start
/connect
```

等待所选服务器显示就绪后，使用匹配版本的 Minecraft Java 客户端连接。
同机通常使用 `localhost:25565`；远程服务器使用 `/connect` 显示的地址。
局域网、端口映射和防火墙需要分别检查；就绪不代表外网已验证可加入。

CLI 启动参数必须使用实例列表中的 UUID：

```sh
mc cli instance list
mc cli instance start <实例UUID>
```

## 日常操作

| 会话命令 | 用途 |
| --- | --- |
| `/open`、`/logs` | 实例详情、最近日志 |
| `/console say Hello` | 发送服务端控制台命令 |
| `/config` | 编辑四个常用服务端属性 |
| `/stop` | 优雅停止，等待停止后再备份 |
| `/backup`、`/backups` | 停服备份、校验、恢复预览和确认 |
| `/quit` | 退出 TUI，服务端继续运行 |

普通文本会发送到当前实例控制台。会话中 `s` 是输入文字；使用 `/start` 启动。
Ctrl-C 退出 TUI，不会强制停止服务器。

遇到问题见[故障排查](TROUBLESHOOTING_CHEATSHEET.md)和 [FAQ](FAQ.md)。
当前不支持 Vanilla provider、接管现有目录、在线或定时备份；后续能力以
[实施差距](IMPLEMENTATION_GAPS.md)为准，暂不承诺发布日期。
