use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::{command_palette::CommandPaletteState, theme};

pub(crate) fn render(frame: &mut Frame<'_>, palette: &CommandPaletteState) {
    let full = frame.area();
    frame.render_widget(Clear, full);
    frame.render_widget(Block::default().style(theme::page()), full);
    let area = popup_area(full);
    frame.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::border(true))
        .title(format!("命令面板 · {}", palette.context))
        .title_style(theme::title(true))
        .style(theme::surface());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height < 3 {
        return;
    }
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(" /", theme::brand()),
                Span::styled(palette.query.clone(), theme::accent()),
                Span::styled("▏", theme::muted()),
            ]),
            Line::from(Span::styled(
                "输入中文或英文关键词筛选；动作会直接切换到对应流程。",
                theme::muted(),
            )),
        ]),
        areas[0],
    );
    let mut items = Vec::new();
    let mut selected = None;
    let mut last_group = None;
    for (index, entry) in palette.filtered_entries().enumerate() {
        if last_group != Some(entry.group) {
            items.push(ListItem::new(Line::from(Span::styled(
                entry.group,
                theme::title(false).add_modifier(Modifier::BOLD),
            ))));
            last_group = Some(entry.group);
        }
        let shortcut = entry.shortcut.unwrap_or("");
        let title_style = if index == palette.selected {
            theme::selected_row()
        } else {
            theme::brand()
        };
        let detail_style = if index == palette.selected {
            theme::accent().add_modifier(Modifier::BOLD)
        } else {
            theme::muted()
        };
        items.push(ListItem::new(vec![
            Line::from(vec![
                Span::styled(format!("  {}", entry.title), title_style),
                Span::styled(format!("  {shortcut}"), theme::key_hint()),
            ]),
            Line::from(Span::styled(format!("    {}", entry.detail), detail_style)),
        ]));
        if index == palette.selected {
            selected = Some(items.len() - 1);
        }
    }
    if items.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "没有匹配命令；退格修改搜索，Esc 关闭。",
            theme::warning(),
        ))));
    }
    let mut state = ListState::default();
    state.select(selected.or(if items.is_empty() { None } else { Some(0) }));
    let list = List::new(items)
        .highlight_style(theme::selected_row())
        .highlight_symbol("▸ ");
    frame.render_stateful_widget(list, areas[1], &mut state);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("↑/↓", theme::key_hint()),
            Span::raw(" 选择 · "),
            Span::styled("Enter", theme::key_hint()),
            Span::raw(" 执行 · "),
            Span::styled("Esc", theme::key_hint()),
            Span::raw(" 关闭 · 输入 "),
            Span::styled("/", theme::key_hint()),
            Span::raw(" 打开命令"),
        ])),
        areas[2],
    );
}

fn popup_area(area: Rect) -> Rect {
    let available = area.width.saturating_sub(4);
    let width = if available < 24 {
        area.width
    } else {
        available.min(78)
    };
    let max_height = area.height.saturating_sub(2).max(6);
    let height = max_height.min(20);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height + 1);
    Rect::new(x, y, width, height)
}
