# 下一步行动清单

> 基于本次产品化优化分析的立即可执行任务

---

## 🔥 今天立即执行

### 1. 等待并审查 Agent 输出 (30-60 分钟)

目前有 **3 个 agents 仍在运行**：
- Agent #5: 首次使用体验优化
- Agent #2: 人性化错误处理
- Agent #7: 安装体验改进

**行动**：
- 等待他们完成（你会收到通知）
- 审查他们的设计方案
- 提取可立即实施的改进

---

### 2. 快速更新 README.md (15 分钟)

**在 "Who is this for?" 之前添加**：

```markdown
## 🎯 核心优势

**为技术服主设计的可靠服务器管理工具**

- 🛡️ **可靠**: 常驻监督，daemon 重启不杀 Java
- 🔍 **可审计**: 每个下载都有 SHA-256/512 校验  
- 💾 **可恢复**: 事务性备份，回滚有完整 journal
- 🔧 **终端优先**: SSH 管理多台服务器，无需浏览器

**最适合**: 管理 3-5 个服务器，数据重要，熟悉终端的服主

**不适合**: 需要零门槛图形界面、大型多用户商业面板
```

**更新功能表格** - 添加状态列：

```markdown
| 功能 | 状态 | 说明 |
|------|------|------|
| Paper 服务端 | ✅ 完整支持 | 引导创建、版本选择 |
| Fabric 服务端 | ✅ 完整支持 | Loader 选择、依赖解析 |
| Vanilla 服务端 | 🔄 开发中 | 计划 v0.3.0 |
| 整合包导入 | ✅ 完整支持 | MRPACK/ZIP |
| 在线备份 | 🔄 开发中 | 当前仅停服备份 |
| 接管现有服务器 | 🔄 开发中 | 计划 v0.3.0 |
```

**在 Quick start 前添加文档链接**：

```markdown
## 📚 开始使用

- 🚀 [5 分钟快速开始](docs/QUICK_START.md) - 从安装到第一次启动
- 🔧 [故障排查速查表](docs/TROUBLESHOOTING_CHEATSHEET.md) - 常见问题快速解决
- 📖 [完整用户指南](docs/USER_GUIDE.md) - 详细操作手册
```

---

### 3. 同步中文 README (10 分钟)

将上述改动同步到 `README.zh-CN.md`。

---

### 4. 创建 v0.3.0 里程碑 (10 分钟)

在 GitHub Issues 中：

1. 创建新里程碑 "v0.3.0 - Core Complete"
2. 目标日期：6 周后
3. 创建以下 Issues 并关联到里程碑：

```markdown
- [ ] Vanilla 服务端支持 #N
- [ ] 接管现有服务器目录 #N
- [ ] 在线备份支持 #N
- [ ] 完整 server.properties 编辑 #N
- [ ] 真实客户端连接验证 #N
- [ ] macOS 签名和公证 #N
- [ ] Homebrew tap 创建 #N
- [ ] Linux deb/rpm 包 #N
```

---

### 5. 提交今天的工作 (5 分钟)

```bash
git add .
git commit -m "docs: add productization optimization analysis and action plan

- Add comprehensive product and UX analysis
- Create 22-task optimization roadmap with 7 phases
- Add Quick Start Guide and Troubleshooting Cheatsheet
- Clarify product positioning: reliable tool for technical admins
- Identify core gaps: Vanilla support, online backup, takeover
- Launch 8 parallel optimization tasks

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"

git push
```

---

## 📅 本周执行

### Monday-Tuesday: 快速改进

1. **改进错误信息**（基于 Agent #2 的方案）
   - 重写 Java 版本不匹配错误
   - 重写端口被占用错误
   - 添加三段式格式模板

2. **优化 CLI 帮助**
   ```rust
   // crates/mc-cli/src/main.rs
   #[command(
       long_about = "终端优先的 Minecraft 服务器管理器\n\
                     适合：SSH 管理、多服务器、重视数据安全\n\n\
                     快速开始：mc setup → mc → /create"
   )]
   ```

3. **添加配置验证**
   ```rust
   // crates/mc-config/src/lib.rs
   impl Config {
       pub fn validate(&self) -> Result<(), ConfigError> {
           // 检查路径存在性和权限
       }
   }
   ```

---

### Wednesday-Friday: Vanilla 支持调研和开始实现

1. **审查 Agent #4 的 Vanilla 调研报告**
2. **创建 provider-vanilla crate**
   ```bash
   cargo new --lib crates/provider-vanilla
   ```

3. **实现 manifest 解析器**
   - Mojang 官方 manifest API
   - version JSON 下载和解析
   - 与 Paper 的差异处理

4. **编写单元测试**

5. **在创建向导中添加 Vanilla 选项**

---

## 📅 Week 2-3: 核心功能开发

### Priority 1: 在线备份

**基于 Agent #6 的方案**：
1. 实现 `save-off` / `save-all` 控制台命令序列
2. 添加一致性标记逻辑
3. 实现失败时的状态恢复
4. 更新 `crates/mc-backup/` 的 API
5. 添加集成测试

**验证**：
- 在运行中的服务器执行备份
- 模拟失败场景
- 恢复后客户端可连接

---

### Priority 2: 接管现有服务器

**设计**：
1. 扫描目录：识别 server.jar、world、配置
2. 解析启动脚本（如果有）
3. 验证完整性
4. 提供 TUI 接管向导 `/import-existing`
5. 处理不完整目录（标记为"需修复"）

**验证**：
- 准备一个手动创建的 Paper 服务器
- 使用接管功能导入
- 启动并验证可用

---

### Priority 3: 完整 server.properties 编辑

**扩展到 20+ 字段**：
- 连接与安全：white-list, max-players, online-mode, pvp
- 世界：level-seed, generate-structures, spawn-protection
- 玩法：difficulty, hardcore, gamemode
- 性能：view-distance, simulation-distance

**实现分类视图**：
```
/config
  → 连接与安全
  → 世界设置
  → 玩法规则
  → 性能优化
  → 原始编辑（高级）
```

---

## 📅 Week 4: 分发优化

### 申请 Apple Developer Program

**成本**: $99/年  
**时间**: 审批可能需要 2-3 天

**行动**：
1. 注册 Apple Developer Program
2. 创建证书和配置文件
3. 配置 notarization

---

### 实现签名和公证

```bash
# scripts/sign-macos.sh
codesign --sign "Developer ID Application: ..." \
         --options runtime \
         --timestamp \
         target/release/mc

# notarization
xcrun notarytool submit mc.zip \
      --apple-id ... \
      --team-id ... \
      --wait
```

---

### 创建 Homebrew Tap

```bash
# 创建仓库
gh repo create homebrew-mc-server-tui --public

# Formula 模板
class Mc < Formula
  desc "Terminal-first Minecraft Java server manager"
  homepage "https://juanwang-buaa.github.io/mc-server-tui/"
  url "https://github.com/JUANWANG-BUAA/mc-server-tui/releases/download/v0.3.0/..."
  sha256 "..."
  
  def install
    bin.install "mc"
  end
end
```

---

### Linux 包

**deb 包**：
```bash
# debian/control
Package: mc-server-tui
Version: 0.3.0
Architecture: amd64
Depends: libc6, libssl3
Description: Minecraft server manager for Linux
```

**rpm 包**：
```bash
# mc-server-tui.spec
Name: mc-server-tui
Version: 0.3.0
Release: 1
Summary: Minecraft server manager
```

---

## 🎯 成功验证清单

完成每个功能后，运行此清单：

### Vanilla 支持
- [ ] 可以创建 Vanilla 服务器
- [ ] 可以选择 1.21.x 版本
- [ ] 下载校验通过
- [ ] 服务器可以启动
- [ ] 客户端可以连接

### 在线备份
- [ ] 在服务器运行时执行备份
- [ ] 备份文件完整
- [ ] 恢复后可以连接
- [ ] 失败时状态正确恢复

### 接管现有服务器
- [ ] 可以扫描并识别现有服务器
- [ ] 导入后配置正确
- [ ] 可以正常启动和管理

### macOS 签名
- [ ] 双击打开无 Gatekeeper 警告
- [ ] 通过 brew install 安装成功
- [ ] 运行 mc --version 正常

### Linux 包
- [ ] apt install 成功（Ubuntu/Debian）
- [ ] dnf install 成功（Fedora）
- [ ] 服务自启动正常

---

## 📊 进度追踪

每周更新 `PROJECT_STATUS_DASHBOARD.md`：

```markdown
## 进度更新 - Week 2

Phase 1: 核心功能完整性  [███▱▱▱▱▱▱▱] 3/5   (60%)
  ✅ Vanilla 支持实现
  ✅ 在线备份实现
  🔄 接管现有目录（进行中）
  ⏸️ 完整属性编辑
  ⏸️ 客户端验证

本周完成：
- Vanilla manifest 解析
- 在线备份 save-off/save-all 序列
- 添加 52 个单元测试

下周计划：
- 完成接管功能
- 开始属性编辑扩展
```

---

## 🎓 记住这些原则

1. **核心优先** - 先完成核心场景，再扩展
2. **用户验证** - 每个功能都要真实测试
3. **分发同步** - 不要让分发落后代码
4. **克制扩张** - 拒绝锦上添花
5. **清晰定位** - 服务明确的目标用户

---

## 📞 需要帮助？

- 查看 `OPTIMIZATION_TODOLIST.md` 了解完整任务
- 查看 `PROJECT_STATUS_DASHBOARD.md` 了解当前进度
- 查看 `docs/QUICK_WINS.md` 了解快速改进
- 查看 agents 的输出获取设计方案

---

**加油！6 周后见到一个真正可以正式发布的产品！** 🚀
