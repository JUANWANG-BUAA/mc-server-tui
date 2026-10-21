// 首次使用欢迎界面
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::{
    app::AppState,
    hit::{HitMap, HitTarget},
    theme,
};

/// 渲染欢迎界面（仅在没有实例且没有任务时显示）
pub(crate) fn render(frame: &mut Frame<'_>, area: Rect, _app: &AppState, hits: &mut HitMap) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(theme::border_type())
        .border_style(theme::border(true))
        .title(" 欢迎使用 MC Server TUI ")
        .title_style(theme::title(true).add_modifier(Modifier::BOLD))
        .style(theme::surface());
    
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // 计算布局
    let compact = area.width < 70 || area.height < 20;
    
    if compact {
        render_compact_welcome(frame, inner, hits);
    } else {
        render_full_welcome(frame, inner, hits);
    }
}

fn render_full_welcome(frame: &mut Frame<'_>, area: Rect, hits: &mut HitMap) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),  // Steve 像素图案
            Constraint::Length(2),  // 间隔
            Constraint::Length(8),  // 三个主要入口
            Constraint::Length(2),  // 间隔
            Constraint::Length(4),  // 快捷键提示
            Constraint::Min(1),     // 底部填充
        ])
        .split(area);

    // Steve 像素图案（居中）
    render_steve_pixel_art(frame, sections[0]);

    // 三个主要入口
    render_action_cards(frame, sections[2], hits);

    // 快捷键提示
    render_shortcuts(frame, sections[4]);
}

fn render_compact_welcome(frame: &mut Frame<'_>, area: Rect, hits: &mut HitMap) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // 标题
            Constraint::Length(6),  // 三个主要入口
            Constraint::Length(3),  // 快捷键提示
            Constraint::Min(1),     // 底部填充
        ])
        .split(area);

    // 简化标题
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("◆ ", theme::accent()),
            Span::styled("开始管理 Minecraft 服务器", theme::text().add_modifier(Modifier::BOLD)),
        ]))
        .alignment(Alignment::Center),
        sections[0],
    );

    // 三个主要入口（紧凑版）
    render_action_cards(frame, sections[1], hits);

    // 快捷键提示（简化版）
    render_shortcuts_compact(frame, sections[2]);
}

fn render_steve_pixel_art(frame: &mut Frame<'_>, area: Rect) {
    // Steve 8x8 像素头像（使用方块字符）
    let steve_lines = vec![
        Line::from(vec![
            Span::styled("        ████████        ", theme::dim()),
        ]),
        Line::from(vec![
            Span::styled("      ██", theme::dim()),
            Span::styled("████", theme::text()),
            Span::styled("████", theme::dim()),
            Span::styled("██      ", theme::dim()),
        ]),
        Line::from(vec![
            Span::styled("      ██", theme::dim()),
            Span::styled("██", theme::text()),
            Span::styled("  ", theme::surface()),
            Span::styled("██", theme::text()),
            Span::styled("  ", theme::surface()),
            Span::styled("██", theme::dim()),
            Span::styled("██      ", theme::dim()),
        ]),
        Line::from(vec![
            Span::styled("      ██", theme::dim()),
            Span::styled("████████", theme::text()),
            Span::styled("██      ", theme::dim()),
        ]),
        Line::from(vec![
            Span::styled("      ██", theme::dim()),
            Span::styled("██  ████", theme::text()),
            Span::styled("██      ", theme::dim()),
        ]),
        Line::from(vec![
            Span::styled("      ██", theme::dim()),
            Span::styled("████", theme::text()),
            Span::styled("  ", theme::surface()),
            Span::styled("██", theme::text()),
            Span::styled("██      ", theme::dim()),
        ]),
        Line::from(vec![
            Span::styled("        ████████        ", theme::dim()),
        ]),
        Line::from(""),
    ];

    frame.render_widget(
        Paragraph::new(steve_lines).alignment(Alignment::Center),
        area,
    );
}

fn render_action_cards(frame: &mut Frame<'_>, area: Rect, hits: &mut HitMap) {
    let cards = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Min(0),
        ])
        .split(area);

    // 卡片 1: 创建 Paper
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" n ", theme::keycap()),
            Span::styled("  创建 Paper 服务器  ", theme::text().add_modifier(Modifier::BOLD)),
            Span::styled("→  版本和 Java 都会自动准备", theme::muted()),
        ])),
        cards[0],
    );
    hits.push(cards[0], HitTarget::WelcomeCreatePaper);

    // 卡片 2: 创建 Fabric
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" f ", theme::keycap()),
            Span::styled("  创建 Fabric 服务器 ", theme::text().add_modifier(Modifier::BOLD)),
            Span::styled("→  自动下载稳定 Loader", theme::muted()),
        ])),
        cards[1],
    );
    hits.push(cards[1], HitTarget::WelcomeCreateFabric);

    // 卡片 3: 导入整合包
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" i ", theme::keycap()),
            Span::styled("  导入现有整合包      ", theme::text().add_modifier(Modifier::BOLD)),
            Span::styled("→  先检查 .mrpack 整合包", theme::muted()),
        ])),
        cards[2],
    );
    hits.push(cards[2], HitTarget::WelcomeImportModpack);
}

fn render_shortcuts(frame: &mut Frame<'_>, area: Rect) {
    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("更多操作：  ", theme::muted()),
            Span::styled(" / ", theme::keycap()),
            Span::styled(" 打开命令面板  ", theme::dim()),
            Span::styled(" ? ", theme::keycap()),
            Span::styled(" 查看帮助  ", theme::dim()),
            Span::styled(" t ", theme::keycap()),
            Span::styled(" 会话终端  ", theme::dim()),
        ]),
        Line::from(vec![
            Span::styled("            ", theme::muted()),
            Span::styled(" F2 ", theme::keycap()),
            Span::styled(" 切换主题  ", theme::dim()),
            Span::styled(" q ", theme::keycap()),
            Span::styled(" 退出 TUI", theme::dim()),
        ]),
    ];

    frame.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center),
        area,
    );
}

fn render_shortcuts_compact(frame: &mut Frame<'_>, area: Rect) {
    let lines = vec![
        Line::from(vec![
            Span::styled(" / ", theme::keycap()),
            Span::styled(" 命令  ", theme::dim()),
            Span::styled(" ? ", theme::keycap()),
            Span::styled(" 帮助  ", theme::dim()),
            Span::styled(" t ", theme::keycap()),
            Span::styled(" 终端  ", theme::dim()),
            Span::styled(" q ", theme::keycap()),
            Span::styled(" 退出", theme::dim()),
        ])
        .alignment(Alignment::Center),
    ];

    frame.render_widget(Paragraph::new(lines), area);
}
