# 5 分钟快速开始指南

> 适合：熟悉终端、想要可靠服务器管理的技术服主

---

## 前置要求

- macOS 13+ (Apple Silicon 或 Intel) 或 Linux (x86_64/arm64)
- Java 21+ (1.21.x) 或 Java 25+ (26.x)
- 终端至少 80×24
- 首次下载需要网络

---

## 第 1 步: 安装 (30 秒)

### macOS

```bash
# 使用官方安装脚本（推荐）
TAG=v0.2.1
curl -fsSL "https://github.com/JUANWANG-BUAA/mc-server-tui/releases/download/$TAG/install.sh" | sh
```

⚠️ **注意**: v0.2.1 是未签名的 alpha 版本。macOS 可能显示"无法验证开发者"警告。正式版将提供签名和 Homebrew 安装。

### Linux

```bash
# Ubuntu/Debian/Fedora/Arch
TAG=v0.2.1
curl -fsSL "https://github.com/JUANWANG-BUAA/mc-server-tui/releases/download/$TAG/install.sh" | sh
```

### 从源码构建

```bash
git clone https://github.com/JUANWANG-BUAA/mc-server-tui.git
cd mc-server-tui
cargo build --release
./target/release/mc --version
```

---

## 第 2 步: 初始化配置 (30 秒)

```bash
mc setup
```

这个命令会：
- ✅ 创建配置文件 `~/.config/mc-server-tui/config.toml`
- ✅ 检查 Java 是否安装
- ✅ 安装后台管理服务（launchd/systemd 用户级服务）
- ✅ 启动 daemon
- ✅ 显示下一步操作

**不想安装后台服务？**

```bash
mc setup --no-service

# 然后在两个终端分别运行：
mc daemon run   # 终端 1 - 管理服务
mc tui          # 终端 2 - 用户界面
```

---

## 第 3 步: 创建第一个服务器 (2 分钟)

```bash
# 启动 TUI
mc
```

在 TUI 中输入：

```
/create
```

然后跟随向导：
1. **选择服务端类型**: Paper (推荐) 或 Fabric
2. **选择 Minecraft 版本**: 如 1.21.11
3. **选择构建**: 使用推荐的最新稳定构建
4. **检查 Java**: 自动检测或手动选择 Java 21+
5. **设置内存**: 如 4GB (根据你的机器配置)
6. **设置端口**: 默认 25565
7. **接受 EULA**: 必须明确同意 Minecraft 的最终用户许可协议

等待下载和初始化完成（首次下载约 1-2 分钟）。

---

## 第 4 步: 启动服务器 (1 分钟)

在 TUI 中：

```
/start
```

或使用快捷键 `s`。

等待启动提示：
- 🟡 "启动中..." - 正在加载
- 🟢 "运行中" - 就绪，可以连接

**使用 CLI 启动**:

```bash
mc cli instance list                    # 查看所有实例
mc cli instance start <实例名>         # 启动指定实例
mc status                               # 查看状态
```

---

## 第 5 步: 连接测试 (1 分钟)

打开 Minecraft Java 版客户端：

1. 点击"多人游戏"
2. 点击"添加服务器"
3. 服务器地址输入: `localhost:25565`
4. 点击"完成"，然后加入服务器

**局域网其他设备连接**:

```bash
# 在 TUI 中查看连接信息
/connect
```

显示本机 IP 地址，其他设备使用 `<你的IP>:25565` 连接。

⚠️ **外网访问需要**:
- 路由器端口映射 (25565 TCP)
- 防火墙放行端口
- 可选：使用 frp/ngrok 等内网穿透

---

## 常用操作

### 查看日志

```bash
# 在 TUI 中
/logs

# 或查看实例详情
/open
# 切换到"日志"标签
```

### 发送控制台命令

```bash
# 在 TUI 中直接输入命令（不需要 /）
say Hello, world!
list
op PlayerName

# 或使用 /console
/console say Hello!
```

### 停止服务器

```bash
# 在 TUI 中
/stop

# 这是优雅停止，会先保存世界
# 强制停止：Ctrl+C (不推荐)
```

### 备份世界

```bash
# 在 TUI 中（需要先停止服务器）
/stop
/backup

# 查看备份列表
/backups
```

### 恢复备份

```bash
# 在备份管理器中
/backups
# 选择备份 → 查看详情 → 确认恢复
```

---

## 🆘 遇到问题？

### Java 未找到

```bash
# 诊断
mc doctor

# 安装 Java (macOS)
brew install openjdk@21

# 安装 Java (Ubuntu/Debian)
sudo apt install openjdk-21-jdk

# 安装 Java (Fedora)
sudo dnf install java-21-openjdk
```

### 端口被占用

```bash
# 查看谁占用了 25565 端口
lsof -i :25565  # macOS/Linux

# 方案 1: 停止占用端口的程序
# 方案 2: 在创建服务器时选择其他端口
```

### 服务启动失败

```bash
# 查看详细错误
/open
# 切换到"诊断"标签

# 或使用 CLI
mc doctor --json
```

### 需要帮助

- 📖 完整文档: [docs/USER_GUIDE.md](docs/USER_GUIDE.md)
- 📖 中文文档: [docs/USER_GUIDE.zh-CN.md](docs/USER_GUIDE.zh-CN.md)
- 🌐 在线文档: https://juanwang-buaa.github.io/mc-server-tui/docs/
- ❓ FAQ: [docs/FAQ.md](docs/FAQ.md)
- 🐛 报告问题: https://github.com/JUANWANG-BUAA/mc-server-tui/issues

---

## 下一步学习

- ✅ [配置服务器属性](docs/USER_GUIDE.md#配置管理)
- ✅ [管理玩家和权限](docs/USER_GUIDE.md#玩家管理)
- ✅ [安装模组和插件](docs/USER_GUIDE.md#模组和插件)
- ✅ [导入整合包](docs/USER_GUIDE.md#整合包导入)
- ✅ [设置自动备份](docs/USER_GUIDE.md#备份策略)
- ✅ [多服务器管理](docs/USER_GUIDE.md#多实例管理)

---

## 🎓 关键概念

### TUI vs CLI

- **TUI (文本用户界面)**: `mc` 或 `mc tui` - 交互式终端界面
- **CLI (命令行界面)**: `mc cli ...` - 脚本友好的命令

### 实例和 Daemon

- **实例 (Instance)**: 一个 Minecraft 服务器配置
- **Daemon**: 后台管理服务，负责监督所有实例
- **Runner**: 每个实例的独立监督进程

关系：
```
TUI/CLI → Daemon → Runner(实例A) → Java(Paper)
                 → Runner(实例B) → Java(Fabric)
```

**重要**: 关闭 TUI 不会停止服务器！服务器由 runner 和 daemon 管理。

---

## ⚠️ Alpha 版本限制

v0.2.1 是**未签名的 alpha 预发布版**，存在以下限制：

- ❌ 无代码签名和 macOS 公证
- ❌ 不支持 Vanilla 官方服务端（计划 v0.3.0）
- ❌ 不能接管现有服务器目录（计划 v0.3.0）
- ❌ 备份必须停服（在线备份计划 v0.3.0）
- ⚠️ Linux 真实主机验证不完整
- ⚠️ 请为重要世界保留独立备份

---

## 📜 许可证

MIT OR Apache-2.0 双许可，任选其一。

本项目独立开发，与 Mojang、Microsoft、PaperMC 无关。  
Minecraft 是 Mojang Synergies AB 的商标。

---

**准备好了？运行 `mc` 开始吧！** 🚀
