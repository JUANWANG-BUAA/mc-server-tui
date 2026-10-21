# 错误处理改进清单 | Error Handling Improvements Checklist

## 用户可见改进 | User-Visible Improvements

### ✅ 已完成 | Completed

#### 1. 三段式错误格式 | Three-Part Error Format
- **发生了什么**：清晰的用户友好摘要
- **为什么发生**：详细原因和技术细节（可折叠）
- **下一步做什么**：可操作的修复步骤

**示例：**
```
✗ 错误 | Error

发生了什么 | What Happened
磁盘空间不足

原因 | Reason  
需要 500 MB，但只有 100 MB 可用

技术细节 | Technical Details (可展开)
required_bytes=524288000, available_bytes=104857600

建议操作 | Recommended Actions
1. 删除不必要的文件以释放磁盘空间
2. 删除旧备份或世界存档
3. 将服务器实例移动到空间更大的磁盘
```

#### 2. 关键错误场景模板 | Key Error Scenario Templates

已实现以下场景的友好错误信息：

- ✅ **Java 版本不匹配** - 提供下载链接和配置指导
- ✅ **端口被占用** - 提供查找占用进程的命令
- ✅ **磁盘空间不足** - 显示具体需求和可用空间
- ✅ **下载失败** - 区分网络问题和源问题
- ✅ **服务器启动失败** - 提供常见原因检查清单
- ✅ **备份失败** - 提供诊断步骤
- ✅ **网络超时** - 建议重试和替代方案
- ✅ **哈希校验失败** - 说明文件损坏并建议重新下载
- ✅ **权限被拒绝** - 提供权限检查和修复方法

#### 3. 双语支持 | Bilingual Support
- ✅ 简体中文（默认）
- ✅ English
- 🔄 自动根据系统语言选择（待实现）

#### 4. 错误对话框组件 | Error Dialog Component
- ✅ 居中弹出式对话框
- ✅ 使用主题颜色系统
- ✅ 响应式布局适配不同终端大小
- ✅ 清晰的视觉层次

#### 5. 诊断报告生成 | Diagnostic Report Generation
- ✅ 完整的错误信息汇总
- ✅ 双语格式
- ✅ 包含时间戳
- ✅ 结构化便于复制粘贴

### 🔄 进行中 | In Progress

#### 6. 键盘交互 | Keyboard Interaction
- 🔄 `d` - 切换技术细节显示
- 🔄 `↑/↓` - 在操作建议间导航
- 🔄 `c` - 复制诊断报告到剪贴板
- 🔄 `r` - 重试操作（如果可重试）
- 🔄 `Esc` - 关闭对话框

#### 7. 集成到现有流程 | Integration into Existing Flows
- 🔄 更新任务详情页面的错误显示
- 🔄 更新实例管理页面的错误处理
- 🔄 更新备份管理页面的错误显示
- 🔄 统一所有错误展示使用新组件

### 📋 待实现 | To Do

#### 8. 剪贴板功能 | Clipboard Functionality
- ⏳ 实现诊断报告复制到系统剪贴板
- ⏳ 复制成功反馈提示
- ⏳ 跨平台剪贴板支持（macOS/Linux/Windows）

#### 9. 智能错误识别 | Smart Error Recognition
- ⏳ 从日志自动提取错误类型
- ⏳ 识别更多错误模式
- ⏳ 提供上下文相关的解决方案

#### 10. 错误恢复操作 | Error Recovery Actions
- ⏳ 从错误对话框直接执行某些修复操作
- ⏳ 一键重试失败的操作
- ⏳ 智能推荐最佳修复方案

#### 11. 错误历史 | Error History
- ⏳ 记录最近的错误
- ⏳ 查看错误发生趋势
- ⏳ 导出错误日志

#### 12. 更多错误模板 | More Error Templates
- ⏳ EULA 未接受
- ⏳ 内存不足（OOM）
- ⏳ 配置文件格式错误
- ⏳ 世界文件损坏
- ⏳ 模组冲突
- ⏳ 插件加载失败
- ⏳ 数据库连接失败

## 技术改进 | Technical Improvements

### ✅ 已完成 | Completed

1. **结构化错误消息** - `ErrorMessage` 类型
2. **错误模板系统** - `ErrorTemplates` 工具类
3. **AppError 集成** - 自动转换到用户友好格式
4. **完整测试覆盖** - 单元测试和集成测试
5. **错误对话框状态管理** - `ErrorDialogState`
6. **错误对话框视图渲染** - `error_dialog_view`

### 🔄 进行中 | In Progress

1. **AppError 自动识别** - 从 evidence 提取参数
2. **错误分类改进** - 更细粒度的错误代码

### 📋 待实现 | To Do

1. **错误统计和分析**
2. **错误聚合和去重**
3. **错误上下文增强**
4. **性能监控集成**

## 使用指南 | Usage Guide

### 开发者：创建新的错误消息

```rust
use mc_core::{ErrorMessage, ErrorTemplates, Language};

// 方式 1: 使用模板（推荐）
let error = ErrorTemplates::disk_space_low(
    needed_bytes,
    available_bytes,
    Language::ZhCN,
);

// 方式 2: 手动构建
let error = ErrorMessage::new(
    "操作失败",
    "具体原因说明"
)
.with_evidence("technical_detail=value")
.with_actions([
    "第一步：做这个",
    "第二步：做那个",
])
.retryable();
```

### 开发者：在 TUI 中显示错误

```rust
use crate::error_dialog::ErrorDialogState;

// 创建错误对话框
let dialog = ErrorDialogState::new(error_message);

// 在渲染循环中
if let Some(dialog) = &app.error_dialog {
    error_dialog_view::render(frame, dialog);
}
```

### 用户：查看错误详情

1. 错误发生时自动显示对话框
2. 阅读"发生了什么"了解问题
3. 按 `d` 键查看技术细节（可选）
4. 查看"建议操作"列表
5. 按 `c` 键复制完整诊断报告（用于报告问题）
6. 按 `r` 键重试（如果显示可重试）
7. 按 `Esc` 关闭对话框

### 用户：复制诊断报告

按 `c` 键复制，然后可以：
- 粘贴到 GitHub Issue
- 发送给技术支持
- 保存到日志文件
- 分享给社区寻求帮助

## 测试 | Testing

### 运行测试

```bash
# 测试错误消息模块
cargo test --package mc-core error_message

# 测试错误对话框
cargo test --package mc-tui error_dialog

# 运行所有测试
cargo test
```

### 测试覆盖率

- ✅ 错误消息构建器
- ✅ 诊断报告生成
- ✅ 所有错误模板（中英文）
- ✅ 错误对话框状态管理
- ✅ 对话框布局计算

## 文档 | Documentation

- ✅ [完整技术文档](./ERROR_HANDLING_IMPROVEMENTS.md)
- ✅ API 文档（通过 `cargo doc` 生成）
- ✅ 代码注释和示例
- ⏳ 用户指南（待添加到用户文档）

## 性能影响 | Performance Impact

- **内存**: 每个错误消息 < 1KB
- **渲染**: 对话框渲染与现有 UI 组件性能相当
- **启动时间**: 无影响
- **运行时开销**: 可忽略不计

## 兼容性 | Compatibility

- ✅ macOS
- ✅ Linux  
- ✅ Windows
- ✅ 所有支持的终端模拟器
- ✅ 向后兼容现有 API

## 反馈和贡献 | Feedback and Contributions

### 报告问题
如果错误消息不够清晰或建议不够具体，请：
1. 复制诊断报告（按 `c` 键）
2. 在 GitHub 创建 Issue
3. 描述期望的改进

### 贡献新模板
欢迎为更多错误场景添加模板：
1. 在 `ErrorTemplates` 中添加方法
2. 提供中英文消息
3. 添加单元测试
4. 提交 Pull Request

## 版本历史 | Version History

### v0.2.1 (Current)
- ✅ 初始错误处理改进实现
- ✅ 9 个关键错误场景模板
- ✅ 错误对话框组件
- ✅ 诊断报告生成
- ✅ 双语支持

### 未来版本 | Future Versions

#### v0.2.2 (Planned)
- 剪贴板功能实现
- 完整 TUI 集成
- 更多错误模板

#### v0.3.0 (Planned)
- 智能错误识别
- 错误恢复操作
- 错误历史记录

## 致谢 | Acknowledgments

感谢所有提供反馈和建议的用户，帮助我们改进错误处理体验。

Thank you to all users who provided feedback and suggestions to help improve the error handling experience.

---

**最后更新 | Last Updated**: 2024-10-21  
**文档版本 | Document Version**: 1.0
