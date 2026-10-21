# 立即可执行的快速改进清单

> 创建日期：2026-10-20
> 这些是不需要大规模重构就能立即改进的项目

## 🚀 文档改进（立即可做）

### 1. README 快速改进

**当前问题**:
- "为什么做这个项目"部分防御性强
- 功能列表没有突出核心价值
- 安装说明藏得太深

**立即改进**:
```markdown
# 在 README.md 顶部，"Why another server manager?" 之前添加：

## 核心优势

**为技术服主设计的可靠服务器管理工具**

- 🛡️ **可靠**: 常驻监督，daemon 重启不杀 Java
- 🔍 **可审计**: 每个下载都有 SHA-256/512 校验
- 💾 **可恢复**: 事务性备份，回滚有完整 journal
- 🔧 **终端优先**: SSH 管理多台服务器，无需浏览器

**最适合**: 管理 3-5 个服务器，数据重要，熟悉终端的服主
```

### 2. 更新功能状态表

**立即修改** README.md 中的功能表：
```markdown
| 功能 | 状态 | 说明 |
|------|------|------|
| Paper 服务端 | ✅ 完整支持 | 引导创建、版本选择、构建锁定 |
| Fabric 服务端 | ✅ 完整支持 | Loader 选择、扩展安装、依赖解析 |
| Vanilla 服务端 | 🔄 开发中 | 计划 v0.3.0 |
| 整合包导入 | ✅ 完整支持 | MRPACK/ZIP，停服导入 |
| 在线备份 | 🔄 开发中 | 当前仅停服备份 |
| 接管现有服务器 | 🔄 开发中 | 计划 v0.3.0 |
```

### 3. 添加"不适合的场景"

**在 README 中明确说明**:
```markdown
## 这个工具不适合

- ❌ 需要零门槛图形界面的新手
- ❌ 需要多用户权限管理的商业服务器
- ❌ 需要远程管理（暂时，v0.4.0 计划支持）
- ❌ 需要 Forge/NeoForge（暂时，考虑中）
```

---

## 🔧 代码快速改进（立即可做）

### 1. 改进错误信息（示例）

**位置**: `crates/mc-core/src/error.rs`

**当前** (假设):
```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Java version mismatch")]
    JavaVersionMismatch,
    
    #[error("Port occupied: {0}")]
    PortOccupied(u16),
}
```

**改进为**:
```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Java 版本不匹配\n当前: {current}\n需要: {required}\n→ 运行 'mc java list' 查看可用 Java")]
    JavaVersionMismatch {
        current: String,
        required: String,
    },
    
    #[error("端口 {port} 已被占用\n占用进程: {process}\n→ 停止该进程或选择其他端口")]
    PortOccupied {
        port: u16,
        process: String,
    },
}
```

### 2. 添加配置验证

**位置**: `crates/mc-config/src/lib.rs`

**添加**:
```rust
impl Config {
    pub fn validate(&self) -> Result<(), ConfigError> {
        // 检查路径是否存在
        if !self.state_dir.exists() {
            return Err(ConfigError::StateDirectoryNotFound);
        }
        
        // 检查权限
        if !self.state_dir.metadata()?.permissions().readonly() {
            return Ok(());
        } else {
            return Err(ConfigError::StateDirectoryReadOnly);
        }
    }
}
```

### 3. 改进 CLI 帮助信息

**位置**: `crates/mc-cli/src/main.rs`

**改进 clap 的 help 文本**:
```rust
#[derive(Parser)]
#[command(
    name = "mc",
    about = "Minecraft 服务器管理工具",
    long_about = "终端优先的 Minecraft Java 服务器管理器\n\
                  适合：SSH 管理、多服务器、重视数据安全的服主\n\n\
                  快速开始：\n\
                    mc setup        # 初始化配置\n\
                    mc              # 启动 TUI\n\
                    mc tui          # 同上\n\n\
                  常用命令：\n\
                    mc status       # 查看所有实例\n\
                    mc cli instance list  # 实例列表\n\
                    mc doctor       # 诊断问题"
)]
struct Cli {
    // ...
}
```

---

## 📝 文档结构改进（立即可做）

### 1. 创建 Quick Start Guide

**新文件**: `docs/QUICK_START.md`

```markdown
# 5 分钟快速开始

## 1. 安装 (30 秒)

### macOS
```bash
# 方式 1: 安装脚本
curl -fsSL https://github.com/JUANWANG-BUAA/mc-server-tui/releases/download/v0.2.1/install.sh | sh

# 方式 2: 源码 (需要 Rust)
git clone https://github.com/JUANWANG-BUAA/mc-server-tui.git
cd mc-server-tui && cargo build --release
```

### Linux
```bash
# Ubuntu/Debian
wget https://github.com/.../mc-server-tui_0.2.1_amd64.deb
sudo dpkg -i mc-server-tui_0.2.1_amd64.deb

# 或使用安装脚本
curl -fsSL https://github.com/.../install.sh | sh
```

## 2. 初始化 (30 秒)

```bash
mc setup
# 这会：
# - 创建配置文件
# - 检查 Java
# - 安装后台服务（可选）
```

## 3. 创建第一个服务器 (2 分钟)

```bash
mc  # 启动 TUI

# 在 TUI 中：
/create
# 跟随向导：选择 Paper -> 版本 -> Java -> 内存 -> 完成
```

## 4. 启动服务器 (1 分钟)

```bash
# 在 TUI 中：
/start
# 等待就绪提示

# 或使用 CLI：
mc cli instance start <实例名>
```

## 5. 连接测试 (1 分钟)

```bash
# 在 Minecraft 客户端添加服务器：
# 地址: localhost:25565 (默认端口)
```

---

## 常见问题

### Java 未找到？
```bash
mc doctor  # 诊断问题
# 然后安装 Java 21+
```

### 端口被占用？
```bash
# 在创建时选择其他端口
# 或停止占用端口的程序
```

### 需要帮助？
- 按 F1 查看帮助
- 输入 /help 查看命令
- 查看完整文档: docs/USER_GUIDE.md
```

### 2. 创建常见问题速查表

**新文件**: `docs/TROUBLESHOOTING_CHEATSHEET.md`

```markdown
# 故障排查速查表

## 🔥 最常见问题

### 1. "Java not found" 或版本不对

**症状**: 启动失败，提示 Java 相关错误

**解决**:
```bash
# 检查 Java
mc doctor

# 列出可用 Java
java --version

# 如果没有 Java 21+，安装：
# macOS:
brew install openjdk@21

# Ubuntu:
sudo apt install openjdk-21-jdk
```

---

### 2. "Port already in use"

**症状**: 启动失败，提示端口被占用

**解决**:
```bash
# 查看谁占用了端口
lsof -i :25565  # macOS/Linux

# 停止占用进程或在 TUI 中选择其他端口
/config  # 修改 server-port
```

---

### 3. 服务器启动后无法连接

**检查清单**:
- [ ] 服务器真的启动了吗？(`/status`)
- [ ] 端口正确吗？(默认 25565)
- [ ] 防火墙开放端口了吗？
- [ ] 客户端版本匹配吗？

---

### 4. 备份失败

**可能原因**:
- 磁盘空间不足
- 服务器正在运行（当前只支持停服备份）
- 权限问题

**解决**:
```bash
# 检查磁盘空间
df -h

# 停止服务器
/stop

# 手动备份
/backup
```

---

### 5. daemon 无法启动

**检查**:
```bash
mc service status

# 如果失败，查看日志
journalctl -u mc-server-tui --user -n 50  # Linux
# 或检查 ~/Library/Logs/mc-server-tui/  # macOS
```

---

## 📞 获取帮助

1. 查看完整文档: `docs/USER_GUIDE.md`
2. 运行诊断: `mc doctor`
3. 提交 Issue: https://github.com/JUANWANG-BUAA/mc-server-tui/issues
```

---

## 🎯 用户体验改进（需要代码修改）

### 1. 添加首次运行向导

**触发条件**: 第一次运行 `mc` 且没有配置文件

**向导流程**:
1. 欢迎界面
2. 检查 Java (如果没有，提供安装指南)
3. 选择数据目录 (默认 ~/.mc-server-tui)
4. 是否安装后台服务
5. 完成，进入 TUI

### 2. 改进 TUI 底部状态栏

**当前**: 显示不明显或信息不足

**改进**:
```
[当前: 生存服 (运行中)] [/命令 ?帮助 F2主题] [玩家: 5/20] [CPU: 32%] [上次备份: 2h前]
```

### 3. 添加"新手提示"模式

**实现**: 在 TUI 中添加可切换的提示模式

```rust
// 配置项
pub struct TuiConfig {
    pub show_hints: bool,  // 默认 true，可用 /hints off 关闭
}

// 在界面上显示上下文提示
match current_screen {
    Screen::InstanceList => "提示: 按 Enter 查看详情  按 / 输入命令  按 ? 查看帮助",
    Screen::InstanceDetail => "提示: 按 s 启停服务器  按 l 查看日志  按 b 备份",
    // ...
}
```

---

## 📊 监控和遥测（后期）

### 1. 本地匿名统计

**收集内容** (仅本地，不上传):
- 安装成功/失败
- 首次创建服务器成功/失败
- 常用命令频率
- 错误类型分布

**用途**:
- 了解用户行为
- 识别高频问题
- 优化功能优先级

### 2. 可选的遥测上传

**要求**:
- 必须用户明确同意
- 可随时关闭
- 数据匿名化
- 开源上传内容

---

## ✅ 完成标准

每项改进都应该：
1. ✅ 有清晰的 git commit
2. ✅ 更新相关文档
3. ✅ 如果是代码改动，添加测试
4. ✅ 在 CHANGELOG 中记录

---

## 🎯 优先级

### 立即做（今天）
1. ✅ README 快速改进
2. ✅ 创建 Quick Start Guide
3. ✅ 创建故障排查速查表
4. ✅ 更新功能状态表

### 本周做
1. 改进错误信息
2. 改进 CLI 帮助
3. 添加配置验证

### 下周做
1. 首次运行向导
2. 改进状态栏
3. 新手提示模式

---

最后更新：2026-10-20
