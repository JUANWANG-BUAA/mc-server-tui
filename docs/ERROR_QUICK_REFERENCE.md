# 错误处理快速参考 | Error Handling Quick Reference

## 常见场景速查 | Common Scenarios Cheat Sheet

### 1️⃣ Java 版本不匹配

**症状**: 服务器无法启动，提示 Java 版本问题

**错误消息**:
```
✗ Java 版本不匹配
服务器需要 Java 21，但发现 Java 17
```

**解决方案**:
```bash
# macOS/Linux
# 1. 下载并安装 Java 21
wget https://adoptium.net/...

# 2. 设置环境变量
export JAVA_HOME=/path/to/java21

# 3. 或在 TUI 中配置
# 设置 -> Java 路径 -> /path/to/java21/bin/java
```

---

### 2️⃣ 端口被占用

**症状**: 服务器启动失败，提示端口冲突

**错误消息**:
```
✗ 端口已被占用
无法启动服务器，因为端口 25565 已被占用
```

**解决方案**:
```bash
# macOS/Linux - 查找占用进程
lsof -i :25565
# 或
netstat -an | grep 25565

# 杀死进程
kill -9 <PID>

# Windows - 查找占用进程
netstat -ano | findstr :25565

# 杀死进程
taskkill /PID <PID> /F

# 或者修改端口
# 编辑 server.properties
server-port=25566
```

---

### 3️⃣ 磁盘空间不足

**症状**: 备份失败或下载中断

**错误消息**:
```
✗ 磁盘空间不足
需要 500 MB，但只有 100 MB 可用
```

**解决方案**:
```bash
# 检查磁盘使用
df -h

# 清理空间
# 1. 删除旧备份
rm -rf /path/to/backups/old-backup-*

# 2. 清理日志
find /path/to/logs -name "*.log" -mtime +30 -delete

# 3. 清理临时文件
rm -rf /tmp/*

# 4. 或移动到更大的磁盘
mv /path/to/instance /mnt/larger-disk/
```

---

### 4️⃣ 下载失败

**症状**: 无法下载服务器核心或插件

**错误消息**:
```
✗ 下载失败
无法下载服务器文件：连接超时
```

**解决方案**:
```bash
# 1. 检查网络连接
ping api.papermc.io

# 2. 测试下载速度
curl -o /dev/null https://api.papermc.io/...

# 3. 使用代理（如果需要）
export HTTP_PROXY=http://proxy.example.com:8080
export HTTPS_PROXY=http://proxy.example.com:8080

# 4. 手动下载后放置
# 下载文件到 downloads/ 目录
# TUI 会自动检测并使用
```

---

### 5️⃣ 服务器启动失败

**症状**: 服务器进程立即退出

**错误消息**:
```
✗ 服务器启动失败
Minecraft 服务器进程意外退出
```

**检查清单**:
- [ ] 查看最新日志: `logs/latest.log`
- [ ] 检查 EULA: `eula=true` in `eula.txt`
- [ ] 验证 Java 版本
- [ ] 检查端口占用
- [ ] 验证世界文件完整性
- [ ] 检查可用内存

**常见日志错误**:
```bash
# EULA 未接受
"You need to agree to the EULA"
→ 编辑 eula.txt，设置 eula=true

# 端口冲突
"Address already in use"
→ 见"端口被占用"部分

# 内存不足
"java.lang.OutOfMemoryError"
→ 增加 JVM 内存: -Xmx4G

# 世界损坏
"Exception reading chunk"
→ 恢复备份或使用工具修复
```

---

### 6️⃣ 备份失败

**症状**: 无法创建世界备份

**错误消息**:
```
✗ 备份失败
无法创建世界备份：磁盘空间不足
```

**解决方案**:
```bash
# 1. 确保服务器已停止
# 在 TUI 中点击"停止"按钮

# 2. 检查磁盘空间
df -h /path/to/backups

# 3. 检查权限
ls -la /path/to/backups
chmod 755 /path/to/backups

# 4. 手动备份（测试）
tar -czf backup-test.tar.gz /path/to/world

# 5. 如果成功，在 TUI 中重试
```

---

### 7️⃣ 网络超时

**症状**: 操作超时，无响应

**错误消息**:
```
✗ 网络超时
操作超时：下载服务器核心
```

**解决方案**:
```bash
# 1. 检查基本连接
ping 8.8.8.8

# 2. 检查 DNS
nslookup api.papermc.io

# 3. 测试 HTTPS 连接
curl -v https://api.papermc.io

# 4. 增加超时时间
# 在配置文件中设置更长的超时

# 5. 使用国内镜像（如果在中国）
# 编辑配置使用镜像源
```

---

### 8️⃣ 哈希校验失败

**症状**: 文件下载完成但验证失败

**错误消息**:
```
✗ 文件完整性检查失败
下载文件的哈希值与预期不符
```

**解决方案**:
```bash
# 1. 删除损坏的文件
rm /path/to/downloads/paper-*.jar

# 2. 清理缓存
rm -rf ~/.cache/mc-server-tui/downloads/*

# 3. 重新下载
# 在 TUI 中重试下载

# 4. 如果持续失败，检查网络质量
# 可能是网络不稳定导致
```

---

### 9️⃣ 权限被拒绝

**症状**: 无法读写文件或目录

**错误消息**:
```
✗ 权限被拒绝
无法写入：/var/minecraft/world
```

**解决方案**:
```bash
# 检查当前用户
whoami

# 检查文件权限
ls -la /var/minecraft/world

# 修改所有者
sudo chown -R $USER:$USER /var/minecraft

# 修改权限
chmod -R 755 /var/minecraft

# 或使用 sudo 运行（不推荐）
sudo mc-server-tui
```

---

## 快速操作 | Quick Actions

### 在错误对话框中

| 按键 | 操作 |
|------|------|
| `d` | 切换技术细节显示 |
| `↑/↓` | 在操作建议间导航 |
| `c` | 复制诊断报告 |
| `r` | 重试操作（如果可重试）|
| `Esc` | 关闭对话框 |

### 复制诊断报告

1. 错误对话框出现时按 `c`
2. 报告已复制到剪贴板
3. 粘贴到 GitHub Issue 或支持请求

### 查看详细日志

```bash
# 服务器日志
tail -f /path/to/instance/logs/latest.log

# TUI 日志（如果启用）
tail -f ~/.local/state/mc-server-tui/logs/tui.log

# 系统日志
journalctl -u mc-server-tui -f
```

---

## 预防措施 | Prevention

### 定期维护

```bash
# 每周
- [ ] 检查磁盘空间: df -h
- [ ] 清理旧日志
- [ ] 验证备份完整性

# 每月
- [ ] 更新 Java 版本
- [ ] 更新服务器核心
- [ ] 测试备份恢复

# 每季度
- [ ] 清理无用世界
- [ ] 优化存储
- [ ] 审查配置
```

### 最佳实践

1. **定期备份**
   - 至少每天一次
   - 保留最近 7 天
   - 测试恢复流程

2. **监控资源**
   - 磁盘空间 > 20% 可用
   - 内存使用 < 80%
   - CPU 使用 < 90%

3. **保持更新**
   - 定期更新 TUI
   - 及时更新服务器核心
   - 关注安全公告

4. **文档记录**
   - 记录配置更改
   - 保存错误报告
   - 维护运维日志

---

## 获取帮助 | Getting Help

### 报告问题

1. **复制诊断报告** (按 `c`)
2. **收集日志文件**
3. **创建 GitHub Issue**: https://github.com/your-repo/issues
4. **提供以下信息**:
   - 操作系统版本
   - TUI 版本
   - 服务器版本
   - 完整的错误消息
   - 重现步骤

### 社区支持

- Discord: https://discord.gg/...
- 论坛: https://forum.example.com
- 文档: https://docs.example.com

### 紧急情况

如果服务器完全无法访问：

```bash
# 1. 检查进程
ps aux | grep java

# 2. 检查端口
netstat -tuln | grep 25565

# 3. 手动启动（调试）
cd /path/to/instance
java -jar server.jar nogui

# 4. 查看错误输出
# 根据错误消息采取行动
```

---

**提示**: 使用 `Ctrl+F` 在本文档中搜索您遇到的错误关键词。

**最后更新**: 2024-10-21
