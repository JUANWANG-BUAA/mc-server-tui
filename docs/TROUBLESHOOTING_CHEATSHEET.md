# 故障排查速查表

> 快速诊断和解决 mc-server-tui 常见问题

---

## 🔥 紧急问题（立即修复）

### 服务器无法启动

#### 症状：点击启动后，服务器立即失败或卡在"启动中"

**诊断步骤**：
```bash
# 1. 检查详细错误
在 TUI 中: /open → 切换到"诊断"标签
或运行: mc doctor

# 2. 查看日志
在 TUI 中: /logs
或直接查看: tail -f ~/.local/share/mc-server-tui/instances/<实例名>/logs/latest.log
```

**常见原因和解决方案**：

| 错误信息 | 原因 | 解决方案 |
|---------|------|---------|
| `UnsupportedClassVersionError` | Java 版本太低 | 安装 Java 21+ (1.21.x) 或 Java 25+ (26.x) |
| `Address already in use` 或 `port ... occupied` | 端口被占用 | 运行 `lsof -i :25565` 查看占用进程，停止它或更换端口 |
| `EULA not accepted` | 未接受 EULA | 在 TUI 中明确接受，或手动编辑 `eula.txt` |
| `OutOfMemoryError` | 内存不足 | 降低 Xmx 设置或增加系统可用内存 |
| `Failed to verify ... checksum` | 文件损坏 | 删除 `~/.cache/mc-server-tui/` 下的缓存，重新下载 |

---

### 无法连接到服务器

#### 症状：Minecraft 客户端显示"无法连接到服务器"

**检查清单**：

1. **服务器真的在运行吗？**
   ```bash
   # 在 TUI 中
   /status
   
   # 应该显示"运行中"，不是"已停止"或"启动中"
   ```

2. **端口正确吗？**
   ```bash
   # 在 TUI 中
   /open → 查看"概览"
   
   # 或检查配置
   /config
   # 查看 server-port 设置（默认 25565）
   ```

3. **防火墙开放了吗？**
   ```bash
   # macOS
   系统设置 → 网络 → 防火墙 → 确保允许传入连接
   
   # Linux (Ubuntu/Debian)
   sudo ufw allow 25565/tcp
   sudo ufw status
   
   # Linux (Fedora/RHEL)
   sudo firewall-cmd --add-port=25565/tcp --permanent
   sudo firewall-cmd --reload
   ```

4. **客户端版本匹配吗？**
   ```bash
   # 检查服务器版本
   在 TUI 中: /status
   
   # 确保客户端版本相同或兼容
   # 例如：服务器 1.21.11，客户端也要 1.21.x
   ```

5. **本地连接 vs 局域网 vs 外网**
   ```bash
   # 本地测试（同一台机器）
   localhost:25565
   
   # 局域网连接
   在 TUI 中: /connect
   # 使用显示的局域网 IP，如 192.168.1.100:25565
   
   # 外网连接
   需要：
   - 公网 IP 或 DDNS
   - 路由器端口映射（外部 25565 → 内部 25565）
   - 防火墙放行
   ```

---

## ⚙️ 配置和管理问题

### Java 未找到或版本不对

#### 症状：`mc setup` 或创建服务器时提示 Java 问题

**解决**：

```bash
# 1. 检查 Java
java -version

# 如果没有输出或版本 < 21，安装 Java
```

**安装 Java**：

```bash
# macOS (Homebrew)
brew install openjdk@21
# 添加到 PATH (可选)
echo 'export PATH="/opt/homebrew/opt/openjdk@21/bin:$PATH"' >> ~/.zshrc

# Ubuntu/Debian
sudo apt update
sudo apt install openjdk-21-jdk

# Fedora/RHEL
sudo dnf install java-21-openjdk

# Arch Linux
sudo pacman -S jdk21-openjdk

# 验证安装
java -version
# 应该显示 "openjdk version "21.x.x"
```

**多个 Java 版本共存**：

```bash
# macOS - 使用 jenv
brew install jenv
jenv add /Library/Java/JavaVirtualMachines/openjdk-21.jdk/Contents/Home
jenv global 21

# Ubuntu/Debian - 使用 update-alternatives
sudo update-alternatives --config java

# 在 mc-server-tui 中手动选择
在创建服务器时选择正确的 Java 路径
```

---

### 备份失败

#### 症状：点击备份后显示失败

**常见原因**：

1. **磁盘空间不足**
   ```bash
   # 检查空间
   df -h
   
   # 清理旧备份
   在 TUI 中: /backups → 选择旧备份 → 删除
   ```

2. **服务器正在运行**（当前版本只支持停服备份）
   ```bash
   # 先停止服务器
   /stop
   
   # 等待完全停止
   /status  # 确认显示"已停止"
   
   # 再备份
   /backup
   ```

3. **权限问题**
   ```bash
   # 检查目录权限
   ls -la ~/.local/share/mc-server-tui/
   
   # 如果权限不对，修复
   chmod -R u+rwX ~/.local/share/mc-server-tui/
   ```

4. **世界文件损坏**
   ```bash
   # 检查世界目录
   cd ~/.local/share/mc-server-tui/instances/<实例名>/world/
   ls -lh
   
   # 如果文件看起来不对，这可能需要从更早的备份恢复
   ```

---

### Daemon 无法启动或连接失败

#### 症状：运行 `mc` 后显示"无法连接到管理服务"

**诊断**：

```bash
# 检查 daemon 状态
mc service status

# 如果显示未运行，启动它
mc service start

# 如果启动失败，查看日志
# macOS
tail -f ~/Library/Logs/mc-server-tui/daemon.log

# Linux
journalctl -u mc-server-tui --user -n 50 -f
```

**常见问题**：

1. **Socket 文件被锁定**
   ```bash
   # 检查 socket
   ls -la /tmp/mc-server-tui-*.sock
   
   # 如果有陈旧的 socket，删除它
   rm /tmp/mc-server-tui-*.sock
   
   # 重启 daemon
   mc service restart
   ```

2. **权限问题**
   ```bash
   # 确保用户有正确权限
   chmod 700 ~/.local/share/mc-server-tui/
   chmod 700 ~/.config/mc-server-tui/
   ```

3. **服务未安装**
   ```bash
   # 重新安装服务
   mc service install
   mc service start
   ```

---

## 🚀 性能问题

### 服务器卡顿或 TPS 低

#### 症状：玩家反馈延迟高，方块更新慢

**检查步骤**：

```bash
# 1. 查看资源使用
在 TUI 中: /open → 查看 CPU/内存趋势图

# 2. 检查在线玩家数
/players list

# 3. 查看警告
/open → 切换到"概览" → 查看警告信息
```

**优化建议**：

1. **增加内存**
   ```bash
   # 当前仅支持创建时设置
   # 需要重新创建实例或手动编辑配置
   ```

2. **减少视距**
   ```bash
   在 TUI 中: /config
   # 修改 view-distance (默认 10，可降低到 6-8)
   ```

3. **优化服务端设置**
   ```bash
   # 编辑 server.properties
   /config
   
   # 关键设置：
   # - view-distance: 8 (默认 10)
   # - simulation-distance: 8 (默认 10)  
   # - max-players: 根据实际需要设置
   ```

4. **检查插件/模组**
   ```bash
   # 移除不必要的插件
   # 更新插件到最新版本
   # 使用 spark 等工具分析性能
   ```

---

### 内存溢出 (OutOfMemoryError)

#### 症状：服务器崩溃，日志显示 OutOfMemoryError

**解决**：

1. **检查当前内存设置**
   ```bash
   在 TUI 中: /open → "概览" → 查看 Xmx/Xms
   ```

2. **增加堆内存**（需要重新创建实例或手动编辑）
   ```bash
   # 当前版本：在创建时设置
   # 未来版本：/config runtime → 调整内存
   ```

3. **检查系统可用内存**
   ```bash
   # macOS
   vm_stat
   
   # Linux
   free -h
   ```

4. **临时缓解**
   ```bash
   # 减少在线玩家
   # 减少加载区块（降低视距）
   # 重启服务器清理内存
   ```

---

## 🔧 安装和更新问题

### 安装脚本失败

#### 症状：`curl | sh` 执行失败

**常见原因**：

1. **网络问题**
   ```bash
   # 测试连接
   curl -I https://github.com/JUANWANG-BUAA/mc-server-tui/releases
   
   # 如果超时，尝试：
   # - 检查代理设置
   # - 使用镜像（如果有）
   # - 手动下载安装包
   ```

2. **权限不足**
   ```bash
   # 不要用 sudo 运行用户级工具
   # 确保 ~/.local/bin 在 PATH 中
   echo $PATH | grep .local/bin
   ```

3. **macOS Gatekeeper 阻止**
   ```bash
   # v0.2.1 未签名，可能被阻止
   # 方案 1: 系统设置 → 隐私与安全 → 允许
   # 方案 2: 从源码构建
   git clone https://github.com/JUANWANG-BUAA/mc-server-tui.git
   cd mc-server-tui
   cargo build --release
   ```

---

### 更新后出现问题

#### 症状：更新到新版本后无法启动

**回滚方案**：

```bash
# 检查当前版本
mc version

# 回滚到之前的版本
mc update rollback

# 如果 rollback 命令不存在（v0.2.1），手动恢复：
# 1. 重新运行旧版本的安装脚本
# 2. 或保留之前的二进制文件备份
```

---

## 🐛 报告问题

如果以上方法都无法解决，请提交 Issue：

### 提交前准备

1. **收集诊断信息**
   ```bash
   mc doctor --json > doctor-report.json
   mc version --json > version-info.json
   mc status --json > status-info.json
   ```

2. **收集日志**（脱敏后）
   ```bash
   # Daemon 日志
   # macOS: ~/Library/Logs/mc-server-tui/daemon.log
   # Linux: journalctl -u mc-server-tui --user -n 100
   
   # 实例日志
   # ~/.local/share/mc-server-tui/instances/<实例名>/logs/latest.log
   ```

3. **描述问题**
   - 操作系统和版本
   - mc-server-tui 版本
   - 服务端类型和版本
   - 复现步骤
   - 预期行为 vs 实际行为

### 提交 Issue

https://github.com/JUANWANG-BUAA/mc-server-tui/issues/new

使用 Bug Report 模板，填写所有必需字段。

---

## 📚 更多资源

- 📖 完整用户指南: [docs/USER_GUIDE.md](USER_GUIDE.md)
- 📖 中文用户指南: [docs/USER_GUIDE.zh-CN.md](USER_GUIDE.zh-CN.md)
- 🌐 在线文档: https://juanwang-buaa.github.io/mc-server-tui/docs/
- ❓ FAQ: [docs/FAQ.md](FAQ.md)
- 🔧 实现差距: [docs/IMPLEMENTATION_GAPS.md](IMPLEMENTATION_GAPS.md)

---

最后更新：2026-10-20  
适用版本：v0.2.1
