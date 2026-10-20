use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
    Frame,
};

use crate::{
    app::AppState,
    hit::{HitMap, HitTarget},
    theme, ui,
};

pub(crate) fn render(frame: &mut Frame<'_>, app: &AppState, hits: &mut HitMap) {
    let full = frame.area();
    frame.render_widget(Clear, full);
    frame.render_widget(Block::default().style(theme::page()), full);
    let outer = full.inner(Margin {
        vertical: 1,
        horizontal: 1,
    });
    frame.render_widget(Clear, outer);
    let block = Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .border_type(theme::border_type())
        .border_style(theme::border(true))
        .title(" 键盘快捷键与鼠标操作 ")
        .title_style(theme::title(true).add_modifier(Modifier::BOLD))
        .style(theme::surface());
    let inner = block.inner(outer);
    frame.render_widget(block, outer);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);
    let content = rows[0];
    let footer = rows[1];

    let lines = help_lines();
    let line_count = lines.len();
    let content_height = content.height as usize;
    let max_scroll = line_count.saturating_sub(content_height);
    app.help_max_scroll.set(max_scroll);
    let scroll = app.help_scroll.min(max_scroll);

    frame.render_widget(
        Paragraph::new(lines)
            .style(theme::surface())
            .scroll((scroll as u16, 0)),
        content,
    );
    ui::scrollbar(frame, outer, line_count, scroll, content_height);
    ui::scrollbar_hits(
        outer,
        line_count,
        content_height,
        hits,
        crate::hit::ScrollView::Help,
    );

    let status = if max_scroll == 0 {
        Line::from(vec![
            Span::styled("全键盘可达", theme::success()),
            Span::styled(" · 鼠标单击选择 / 双击打开", theme::muted()),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                format!(
                    "第 {}-{}/{} 行",
                    scroll + 1,
                    (scroll + content_height).min(line_count),
                    line_count
                ),
                theme::accent(),
            ),
            Span::styled(" · ", theme::dim()),
            Span::styled("↑↓ 滚动 · PgUp/PgDn 翻页", theme::muted()),
        ])
    };
    let footer_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(16)])
        .split(footer);
    frame.render_widget(Paragraph::new(status), footer_area[0]);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" Esc ", theme::keycap()),
            Span::styled(" 关闭 ", theme::muted()),
        ]))
        .alignment(Alignment::Right),
        footer_area[1],
    );
    hits.push(footer, HitTarget::HelpClose);
}

fn help_lines() -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    section(&mut lines, "全局");
    item(&mut lines, "Ctrl-K", "打开命令面板（任意页面）");
    item(&mut lines, "?  /  F1", "显示或关闭本帮助");
    item(&mut lines, "q  /  Ctrl-C", "退出 TUI；后台任务会继续运行");
    item(&mut lines, "Esc", "返回上一层或关闭浮层");
    item(&mut lines, "/", "在非文本页面打开命令面板");
    item(&mut lines, "F2", "循环切换 7 套主题；/theme 列出全部");

    item(&mut lines, "F3", "运行设置 / 实例摘要 / 备份详情展开与收起");

    section(&mut lines, "会话终端");
    item(&mut lines, "Enter", "发送输入；以 / 开头则执行命令");
    item(
        &mut lines,
        "Tab / Enter",
        "Tab 填入补全；前缀 Enter 先补全，再次 Enter 执行",
    );
    item(
        &mut lines,
        "← →  /  Home End",
        "移动输入光标；Ctrl-A / Ctrl-E 行首行尾",
    );
    item(&mut lines, "↑ ↓", "翻历史命令，或在补全列表中移动");
    item(
        &mut lines,
        "PgUp / PgDn",
        "翻阅会话记录；Ctrl-Home/End 首尾",
    );
    item(&mut lines, "Ctrl-L", "清空当前会话记录");
    item(&mut lines, "F2", "切换主题；/theme 可列出或指定");
    item(
        &mut lines,
        "直接输入文本",
        "发往输入框标注的服务器；切服后草稿禁止发送",
    );
    item(&mut lines, "/logs", "在会话中查看最近 12 行控制台日志");
    item(&mut lines, "/cancel [n]", "取消下载任务并保留可续传文件");

    section(&mut lines, "仪表盘");
    item(&mut lines, "↑ ↓  /  j k", "在实例或任务列表中移动");
    item(&mut lines, "Tab / Shift-Tab", "切换实例与任务面板");
    item(&mut lines, "1  /  2", "直接聚焦实例 / 任务面板");
    item(&mut lines, "Enter", "打开所选实例或任务详情");
    item(&mut lines, "n  /  i", "新建 Paper 服务器 / 导入整合包");
    item(&mut lines, "PgUp / PgDn", "列表翻页");
    item(&mut lines, "Home / End", "跳到首项 / 末项");
    item(&mut lines, "r", "立即刷新仪表盘");
    item(&mut lines, "t", "切换到会话终端");

    section(&mut lines, "新建向导");
    item(&mut lines, "Tab / Shift-Tab", "切换字段");
    item(&mut lines, "↑ ↓", "在版本、Java、内存列表中移动");
    item(&mut lines, "PgUp / PgDn", "选择列表翻页");
    item(&mut lines, "Home / End", "第一个 / 最后一个选项");
    item(&mut lines, "← →", "切换选项（兼容旧习惯）");
    item(&mut lines, "Enter", "下一步 / 提交创建任务");
    item(&mut lines, "Space", "接受 Minecraft EULA（仅确认页）");
    item(&mut lines, "Esc", "上一步 / 取消向导");

    section(&mut lines, "实例详情");
    item(&mut lines, "s  /  x", "启动 / 优雅停止");
    item(&mut lines, ":  /  b  /  l", "控制台 / 备份 / 世界与备份");
    item(&mut lines, "↑ ↓  /  PgUp PgDn", "滚动控制台日志");
    item(&mut lines, "Home / End", "日志顶部 / 最新输出");
    item(&mut lines, "r", "刷新状态与增量日志");
    item(&mut lines, "d  /  Esc", "回到会话主页 / 实例列表");

    section(&mut lines, "任务详情");
    item(&mut lines, "↑ ↓  /  PgUp PgDn", "滚动任务事件");
    item(&mut lines, "Home / End", "最早 / 最新事件");
    item(&mut lines, "x", "取消下载（按两次确认）");
    item(&mut lines, "r  /  d  /  Esc", "刷新 / 仪表盘 / 任务列表");

    section(&mut lines, "世界与备份");
    item(&mut lines, "↑ ↓  /  PgUp PgDn", "选择备份快照");
    item(&mut lines, "v", "重新哈希并校验所选备份");
    item(&mut lines, "Enter", "预览恢复影响");
    item(&mut lines, "c", "输入 RESTORE 确认恢复");
    item(&mut lines, "m  /  b  /  r", "更多 / 新建备份 / 刷新");
    item(&mut lines, "d  /  Esc", "回到会话主页 / 实例详情");

    section(&mut lines, "命令面板");
    item(&mut lines, "输入关键词", "按标题、说明和中英文别名搜索");
    item(&mut lines, "↑ ↓  /  PgUp PgDn", "选择命令");
    item(&mut lines, "Enter  /  Esc", "执行 / 关闭");
    item(&mut lines, "F1", "在文本输入界面也可打开帮助");

    section(&mut lines, "鼠标");
    item(&mut lines, "滚轮", "滚动任务列表、事件流或控制台日志");
    item(&mut lines, "单击", "选中行、聚焦面板、选中版本/Java/内存");
    item(&mut lines, "双击", "打开实例或任务、执行命令面板条目");
    item(&mut lines, "点击 EULA 行", "直接切换 EULA 勾选状态");

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "提示：本帮助不会暂停后台任务；按 Esc 关闭后继续操作。",
        theme::dim(),
    )));
    lines
}

fn section(lines: &mut Vec<Line<'static>>, title: &'static str) {
    if !lines.is_empty() {
        lines.push(Line::from(""));
    }
    lines.push(Line::from(vec![
        Span::styled("▍", theme::accent()),
        Span::styled(title, theme::accent().add_modifier(Modifier::BOLD)),
    ]));
}

fn item(lines: &mut Vec<Line<'static>>, key: &'static str, description: &'static str) {
    lines.push(Line::from(vec![
        Span::styled(format!("  {key:<20}"), theme::key_hint()),
        Span::styled(description, theme::muted()),
    ]));
}
