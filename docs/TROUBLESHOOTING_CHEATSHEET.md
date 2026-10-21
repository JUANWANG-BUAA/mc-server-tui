# 故障排查速查

适用于 v0.2.1 alpha。先记录错误原因、证据和下一步建议，再决定是否重试。
完整说明见[中文手册](USER_GUIDE.zh-CN.md)和 [FAQ](FAQ.md)。

## 找不到命令或后台服务

```sh
~/.local/bin/mc version
export PATH="$HOME/.local/bin:$PATH"
mc setup --dry-run
mc service status
mc status --json
```

`mc setup --dry-run` 用于确认配置与服务位置。不要根据其他平台的示例猜测路径。
已安装服务但 daemon 无响应时，可运行 `mc service restart`；此操作保留常驻 runner 和 Java。
需要观察前台日志时先运行 `mc service stop`，再执行 `mc daemon run`，另开终端 `mc tui`。
不要删除正在使用的 socket 或数据目录。

Linux 用户服务还可使用 `journalctl --user -u mc-server-tui -n 100` 查看日志。
没有 systemd 用户会话时使用 `mc setup --no-service` 和前台 daemon。

## Java、端口与启动失败

```sh
java -version
mc doctor --json
```

1. 核对服务端精确版本与 Java 主版本：1.21.x 使用 Java 21，26.x 使用 Java 25+。
2. Paper 向导中 F3 展开运行设置，检查检测到的 Java 路径和端口。
3. 在会话执行 `/open` 检查启动诊断和日志；`/logs` 查看最近输出。
4. 端口冲突时，确认占用者后停止对应服务，或更换实例端口。

`/start` 提交成功不等于游戏就绪。应用显示就绪后，再用真实 Minecraft 客户端验证加入。

## 下载或后台任务失败

使用 `/tasks`、`/task <序号>` 查看失败原因、下载源和事件证据。
检查网络、磁盘空间和校验错误。`/cancel [序号]` 只适用于可取消的下载任务。
保留错误信息，按页面建议恢复；不要手工修改锁文件或跳过校验。

## 备份与恢复

当前只支持停服世界备份。先 `/stop`，确认已停止后 `/backup`。
`/backups` 中先校验并审查恢复影响，再输入要求的确认文本。
恢复完成后需显式 `/start`，并进入游戏检查世界内容。
遇到 recovery journal 或回滚失败提示时，先保留现场与独立备份，按错误证据处理。

## 导出诊断信息

```sh
mc version --json > version-info.json
mc status --json > status-info.json
mc doctor --json > doctor-report.json
```

提交问题前审查输出中的本机路径、地址和日志，移除私密内容。
源码未发布改动中的错误对话框支持 `d` 展开证据、方向键滚动、Esc 关闭；
当前没有剪贴板复制和自动重试按钮。已发布 v0.2.1 仍使用页面内错误提示。
