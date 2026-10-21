# 功能集成实施计划

> 创建时间：2026-10-20
> 目标：将新实现的组件集成到 TUI 主流程

---

## 📋 当前状态总结

### ✅ 已完成的工作
1. **ErrorMessage 系统** - 完整实现，已测试
2. **欢迎界面组件** (welcome_view.rs) - 已实现
3. **错误对话框组件** (error_dialog.rs) - 已实现
4. **上下文提示系统** (contextual_hints.rs) - 已实现
5. **改进的安装脚本** (install_improved.sh) - 已实现

### ⚠️ 待完成的工作
1. 集成欢迎界面到主 TUI
2. 集成错误对话框
3. 集成上下文提示到状态栏
4. 端到端测试
5. 清理未使用代码警告

---

## 🔧 集成步骤详解

### Step 1: 集成欢迎界面 (预计 1-2 小时)

**目标**：在没有实例时显示欢迎界面，而非空白列表

**需要修改的文件**：
1. `crates/mc-tui/src/view.rs` - 添加欢迎界面渲染逻辑
2. `crates/mc-tui/src/app.rs` - 检测空状态
3. `crates/mc-tui/src/lib.rs` - 导出 welcome_view 模块

**实施细节**：
```rust
// 在 view.rs 中
pub(crate) fn render(frame: &mut Frame<'_>, app: &AppState, hits: &mut HitMap) {
    // 检查是否应该显示欢迎界面
    if should_show_welcome(app) {
        welcome_view::render(frame, frame.area(), app, hits);
        return;
    }
    
    // 原有的渲染逻辑...
}

fn should_show_welcome(app: &AppState) -> bool {
    // 条件：
    // 1. 没有实例
    // 2. 没有正在运行的任务
    // 3. 用户没有手动关闭欢迎界面
    app.instances.is_empty() 
        && app.tasks.is_empty()
        && !app.welcome_dismissed
}
```

**测试方法**：
```bash
# 1. 清空实例数据（备份后）
mv ~/.local/share/mc-server-tui/instances ~/mc-instances-backup

# 2. 启动 TUI
./target/release/mc

# 3. 预期结果：看到欢迎界面，有三个创建按钮
# 4. 点击任意按钮，应该触发创建流程
# 5. 创建完成后，欢迎界面消失

# 6. 恢复数据
mv ~/mc-instances-backup ~/.local/share/mc-server-tui/instances
```

---

### Step 2: 集成错误对话框 (预计 1-2 小时)

**目标**：在错误发生时显示模态对话框，而非简单提示

**需要修改的文件**：
1. `crates/mc-tui/src/app.rs` - 添加错误对话框状态
2. `crates/mc-tui/src/view.rs` - 在顶层渲染错误对话框
3. 错误处理代码 - 使用 ErrorMessage 构建错误

**AppState 扩展**：
```rust
pub struct AppState {
    // ... 现有字段
    
    /// 当前显示的错误对话框（如果有）
    pub error_dialog: Option<ErrorDialog>,
}

impl AppState {
    pub fn show_error(&mut self, error: ErrorMessage) {
        self.error_dialog = Some(ErrorDialog::new(error));
    }
    
    pub fn dismiss_error(&mut self) {
        self.error_dialog = None;
    }
}
```

**测试方法**：
```bash
# 1. 触发一个已知错误（例如端口被占用）
# 先启动一个占用 25565 的程序
nc -l 25565 &

# 2. 在 TUI 中创建服务器（默认端口 25565）
./target/release/mc
# /create -> 选择 Paper -> ... -> 启动

# 3. 预期结果：看到错误对话框
# - 显示 "端口被占用"
# - 显示端口号和建议操作
# - 有 "关闭" 按钮

# 4. 测试 "复制诊断报告" 功能
# - 点击按钮
# - 检查剪贴板是否包含诊断信息

# 5. 清理
killall nc
```

---

### Step 3: 集成上下文提示 (预计 1 小时)

**目标**：在底部状态栏显示当前可用操作

**需要修改的文件**：
1. `crates/mc-tui/src/view.rs` - 更新状态栏渲染
2. `crates/mc-tui/src/contextual_hints.rs` - 根据 focus 生成提示

**实施细节**：
```rust
// 在渲染状态栏时
fn render_status_bar(frame: &mut Frame, area: Rect, app: &AppState) {
    let hint = ContextualHint::from_app_state(app);
    let hint_line = hint.render_as_status_line();
    
    // 将提示渲染到状态栏右侧
    frame.render_widget(hint_line, status_bar_hint_area);
}
```

**测试方法**：
```bash
# 1. 启动 TUI
./target/release/mc

# 2. 在不同界面切换，观察底部状态栏
# - 实例列表：应显示 "Enter 查看详情  / 命令  ? 帮助"
# - 实例详情：应显示 "s 启停  l 日志  b 备份"
# - 控制台：应显示 "直接输入命令  Esc 返回"

# 3. 验证提示准确性
```

---

### Step 4: 端到端测试 (预计 1 小时)

**测试场景清单**：

1. **首次使用流程**
   - [ ] 清空实例数据
   - [ ] 启动 TUI
   - [ ] 看到欢迎界面
   - [ ] 点击 "创建 Paper 服务器"
   - [ ] 完成创建流程
   - [ ] 欢迎界面消失，显示实例列表

2. **错误处理流程**
   - [ ] 触发 Java 版本错误（使用错误的 Java）
   - [ ] 看到错误对话框，显示三段式信息
   - [ ] 点击 "复制诊断报告"
   - [ ] 验证剪贴板内容
   - [ ] 关闭对话框

3. **上下文提示**
   - [ ] 在各个界面验证提示正确显示
   - [ ] 提示与实际可用操作匹配

4. **回归测试**
   - [ ] 现有功能不受影响
   - [ ] 创建、启动、停止、备份等核心功能正常
   - [ ] 多实例切换正常

---

### Step 5: 清理和优化 (预计 1 小时)

**待修复的警告**：
```rust
// 1. 移除未使用的导入
// crates/mc-tui/src/error_dialog_view.rs
- use crate::ui;  // 如果未使用就移除

// 2. 标记暂时未使用的代码
#[allow(dead_code)]
enum HintType {
    // ...
}

// 3. 添加文档注释
/// 根据应用状态生成上下文提示
pub fn from_app_state(app: &AppState) -> Self {
    // ...
}
```

---

## 📊 预期成果

完成集成后，用户将体验到：

1. **更友好的首次使用**
   - 不再看到空白界面
   - 清晰的三个入口选项
   - 像素风格的 Steve 增加亲和力

2. **更清晰的错误信息**
   - 不再只有简单的错误文本
   - 三段式结构：发生了什么、为什么、怎么办
   - 可复制的诊断报告

3. **更好的可发现性**
   - 底部状态栏显示当前可用操作
   - 新手不需要记忆所有快捷键
   - 上下文相关的提示

---

## 🎯 成功标准

### 功能完整性
- [ ] 欢迎界面在空状态时正常显示
- [ ] 三个入口按钮功能正常
- [ ] 错误对话框在错误时正常显示
- [ ] 诊断报告可以复制
- [ ] 上下文提示在不同界面正确显示

### 用户体验
- [ ] 界面响应流畅，无卡顿
- [ ] 布局在不同终端尺寸下正常
- [ ] 文案清晰易懂
- [ ] 操作直观，符合预期

### 代码质量
- [ ] 无编译错误
- [ ] 无编译警告（或已标记为允许）
- [ ] 单元测试通过
- [ ] 代码有适当的注释

---

## 📅 执行时间表

### 今天晚些时候 (3-4 小时)
- Step 1: 集成欢迎界面 (1-2h)
- Step 2: 集成错误对话框 (1-2h)

### 明天 (2-3 小时)
- Step 3: 集成上下文提示 (1h)
- Step 4: 端到端测试 (1h)
- Step 5: 清理和优化 (1h)

### 提交和文档 (30 分钟)
- 提交集成代码
- 更新 CHANGELOG
- 更新 PROJECT_STATUS_DASHBOARD
- 标记 Quick Wins 完成

---

## 💡 注意事项

1. **备份数据**：测试前备份 `~/.local/share/mc-server-tui/`
2. **增量提交**：每个 Step 完成后立即提交
3. **实际测试**：不要只依赖编译通过，一定要实际运行
4. **用户视角**：测试时模拟新用户，不要跳过步骤
5. **文档同步**：更新用户指南中的截图和说明

---

## 🚀 开始集成

准备好后，按照上述步骤逐一执行。遇到问题随时记录，不要强行推进。

**下一步**：开始 Step 1 - 集成欢迎界面。
