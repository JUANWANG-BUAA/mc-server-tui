# 错误处理改进 | Error Handling Improvements

## 概述 | Overview

本文档记录了 Minecraft 服务器 TUI 的错误处理改进，旨在提供更人性化、更可操作的错误信息。

This document tracks the error handling improvements for the Minecraft server TUI, aiming to provide more user-friendly and actionable error messages.

## 三段式错误格式 | Three-Part Error Format

每个错误消息遵循三段式结构：

Each error message follows a three-part structure:

### 1. 发生了什么 | What Happened
- 用户友好的摘要
- 避免技术术语
- 简洁明了

User-friendly summary without technical jargon.

**示例 | Example:**
- ✅ "磁盘空间不足" / "Insufficient disk space"
- ❌ "Error: ENOSPC"

### 2. 为什么发生 | Why It Happened
- 具体原因
- 技术细节（可折叠）
- 诊断证据

Specific reason with technical details (collapsible) and diagnostic evidence.

**示例 | Example:**
- "需要 500 MB，但只有 100 MB 可用"
- "Need 500 MB, but only 100 MB available"
- 技术细节: `required_bytes=524288000, available_bytes=104857600`

### 3. 下一步做什么 | What to Do Next
- 可操作的步骤
- 优先级排序
- 具体指导

Actionable steps with priority order and specific guidance.

**示例 | Example:**
1. 删除不必要的文件以释放磁盘空间
2. 删除旧备份或世界存档
3. 将服务器实例移动到空间更大的磁盘

## 关键错误场景模板 | Key Error Scenario Templates

### 1. Java 版本不匹配 | Java Version Mismatch

```rust
ErrorTemplates::java_version_mismatch("21", "17", Language::ZhCN)
```

**用户看到 | User sees:**
- 摘要: "Java 版本不匹配"
- 原因: "服务器需要 Java 21，但发现 Java 17"
- 操作:
  1. 从 https://adoptium.net 安装 Java 21
  2. 设置 JAVA_HOME 环境变量
  3. 或使用 TUI 配置自定义 Java 路径

### 2. 端口被占用 | Port Already in Use

```rust
ErrorTemplates::port_in_use(25565, Language::ZhCN)
```

**用户看到 | User sees:**
- 摘要: "端口已被占用"
- 原因: "无法启动服务器，因为端口 25565 已被占用"
- 操作:
  1. 停止占用端口 25565 的进程
  2. 在 server.properties 中修改服务器端口
  3. 使用命令查找占用进程

### 3. 磁盘空间不足 | Insufficient Disk Space

```rust
ErrorTemplates::disk_space_low(needed, available, Language::ZhCN)
```

**用户看到 | User sees:**
- 摘要: "磁盘空间不足"
- 原因: "需要 500 MB，但只有 100 MB 可用"
- 可重试: 是
- 操作:
  1. 删除不必要的文件以释放磁盘空间
  2. 删除旧备份或世界存档
  3. 将服务器实例移动到空间更大的磁盘

### 4. 下载失败 | Download Failed

```rust
ErrorTemplates::download_failed(url, reason, Language::ZhCN)
```

**用户看到 | User sees:**
- 摘要: "下载失败"
- 原因: "无法下载服务器文件：连接超时"
- 可重试: 是
- 操作:
  1. 检查网络连接
  2. 验证下载链接是否可访问
  3. 稍后重试
  4. 检查代理或防火墙设置

### 5. 服务器启动失败 | Server Start Failed

```rust
ErrorTemplates::server_start_failed(reason, Language::ZhCN)
```

**用户看到 | User sees:**
- 摘要: "服务器启动失败"
- 原因: "Minecraft 服务器进程意外退出：端口冲突"
- 操作:
  1. 查看服务器日志了解详细错误信息
  2. 验证 Java 版本兼容性
  3. 确保已接受 EULA 协议
  4. 检查端口冲突
  5. 验证世界文件是否损坏

### 6. 备份失败 | Backup Failed

```rust
ErrorTemplates::backup_failed(reason, Language::ZhCN)
```

**用户看到 | User sees:**
- 摘要: "备份失败"
- 原因: "无法创建世界备份：磁盘空间不足"
- 可重试: 是
- 操作:
  1. 确保服务器已停止再进行备份
  2. 检查可用磁盘空间
  3. 验证世界目录权限
  4. 尝试逐个备份世界

### 7. 网络超时 | Network Timeout

```rust
ErrorTemplates::network_timeout(operation, Language::ZhCN)
```

**用户看到 | User sees:**
- 摘要: "网络超时"
- 原因: "操作超时：下载服务器核心"
- 可重试: 是
- 操作:
  1. 检查互联网连接
  2. 网络状况改善后重试
  3. 考虑使用其他镜像或下载源

### 8. 哈希校验失败 | Hash Mismatch

```rust
ErrorTemplates::hash_mismatch(expected, actual, Language::ZhCN)
```

**用户看到 | User sees:**
- 摘要: "文件完整性检查失败"
- 原因: "下载文件的哈希值与预期不符"
- 可重试: 是
- 操作:
  1. 删除损坏的文件并重新下载
  2. 检查下载源是否可信
  3. 验证网络连接是否稳定

### 9. 权限被拒绝 | Permission Denied

```rust
ErrorTemplates::permission_denied(path, operation, Language::ZhCN)
```

**用户看到 | User sees:**
- 摘要: "权限被拒绝"
- 原因: "无法写入：/path/to/file"
- 操作:
  1. 检查文件和目录权限
  2. 确保用户对目标路径有写入权限
  3. 使用适当的权限运行或更改实例位置

## 错误对话框组件 | Error Dialog Component

### 功能特性 | Features

1. **三段式展示**
   - 清晰的摘要行
   - 详细原因说明
   - 技术细节可折叠显示
   - 可操作步骤列表

2. **键盘导航**
   - `d` - 切换技术细节显示
   - `↑/↓` - 在操作建议间导航
   - `c` - 复制诊断报告到剪贴板
   - `r` - 重试操作（如果可重试）
   - `Esc` - 关闭对话框

3. **视觉反馈**
   - 错误图标和标题（根据是否可重试）
   - 高亮当前选中的操作
   - 使用主题颜色系统
   - 响应式布局

### 使用示例 | Usage Example

```rust
use mc_core::{ErrorMessage, ErrorTemplates, Language};
use crate::error_dialog::ErrorDialogState;

// 创建错误消息
let error_msg = ErrorTemplates::disk_space_low(
    1024 * 1024 * 500, // 500 MB needed
    1024 * 1024 * 100, // 100 MB available
    Language::ZhCN,
);

// 创建对话框状态
let dialog_state = ErrorDialogState::new(error_msg);

// 渲染对话框
error_dialog_view::render(frame, &dialog_state);
```

## 诊断报告 | Diagnostic Report

每个错误都可以生成完整的诊断报告供用户复制：

Each error can generate a complete diagnostic report for users to copy:

```
=== 错误诊断报告 | Error Diagnostic Report ===

问题 | Issue:
磁盘空间不足

原因 | Reason:
需要 500 MB，但只有 100 MB 可用

技术细节 | Technical Details:
required_bytes=524288000, available_bytes=104857600

建议操作 | Recommended Actions:
1. 删除不必要的文件以释放磁盘空间
2. 删除旧备份或世界存档
3. 将服务器实例移动到空间更大的磁盘

可重试 | Retryable: 是 | Yes

生成时间 | Generated: 2024-10-21 10:30:45 UTC
```

## AppError 集成 | AppError Integration

`AppError` 现在支持自动转换为用户友好的 `ErrorMessage`：

```rust
let app_error = AppError::new(ErrorCode::DiskSpaceLow, reason)
    .with_evidence("required_bytes=500000000, available_bytes=100000000")
    .with_action("Free space on the instance volume")
    .retryable();

// 转换为用户友好消息
let error_message = app_error.to_error_message(Language::ZhCN);
```

系统会尝试：
1. 识别已知错误模式并使用模板
2. 从 evidence 中提取参数
3. 回退到通用转换

## 实现文件 | Implementation Files

### 核心层 | Core Layer
- `crates/mc-core/src/error_message.rs` - 错误消息结构和模板
- `crates/mc-core/src/error.rs` - AppError 扩展

### TUI 层 | TUI Layer
- `crates/mc-tui/src/error_dialog.rs` - 对话框状态管理
- `crates/mc-tui/src/error_dialog_view.rs` - 对话框渲染

## 测试 | Testing

所有错误模板都有单元测试覆盖：

```bash
cargo test --package mc-core error_message
cargo test --package mc-tui error_dialog
```

## 国际化 | Internationalization

当前支持语言：
- 简体中文 (zh-CN) - 默认
- English (en)

添加新语言：

```rust
pub enum Language {
    En,
    ZhCN,
    // 添加新语言
    JaJP,  // 日语
}

impl ErrorTemplates {
    pub fn java_version_mismatch(required: &str, found: &str, lang: Language) -> ErrorMessage {
        match lang {
            Language::En => { /* ... */ },
            Language::ZhCN => { /* ... */ },
            Language::JaJP => { /* ... */ },
        }
    }
}
```

## 最佳实践 | Best Practices

### DO ✅

1. **使用模板** - 为常见错误使用预定义模板
2. **提供证据** - 在 evidence 字段包含技术细节
3. **可操作步骤** - 给出具体、可执行的建议
4. **标记可重试** - 明确标识暂时性错误
5. **保持简洁** - 摘要应该一目了然

### DON'T ❌

1. **技术术语** - 避免在摘要中使用专业术语
2. **模糊建议** - "检查配置" 不如 "在 server.properties 中设置 eula=true"
3. **冗长说明** - 保持每个部分简洁
4. **混合语言** - 保持一致的语言风格
5. **忽略证据** - 总是提供可用于调试的技术细节

## 未来改进 | Future Improvements

### 短期 | Short Term
- [ ] 添加剪贴板复制功能实现
- [ ] 集成到现有错误处理流程
- [ ] 添加更多错误场景模板
- [ ] 完善错误对话框键盘导航

### 中期 | Medium Term
- [ ] 添加错误统计和分析
- [ ] 实现错误恢复建议的智能排序
- [ ] 支持从错误对话框直接执行某些操作
- [ ] 添加错误历史记录

### 长期 | Long Term
- [ ] 机器学习驱动的错误诊断
- [ ] 社区贡献的错误解决方案库
- [ ] 多语言完整支持
- [ ] 可视化错误分析工具

## 贡献指南 | Contributing

添加新的错误模板：

1. 在 `ErrorTemplates` 中添加新方法
2. 为中英文提供消息
3. 添加单元测试
4. 更新本文档
5. 在相关错误转换中使用模板

示例 PR checklist:
- [ ] 实现了模板方法
- [ ] 添加了单元测试
- [ ] 更新了文档
- [ ] 在 AppError 转换中集成
- [ ] 验证了中英文显示

## 参考 | References

- [Rust Error Handling Best Practices](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [UX Writing for Error Messages](https://uxwritinghub.com/error-message-examples/)
- [Material Design Error States](https://m2.material.io/design/communication/data-formats.html)
