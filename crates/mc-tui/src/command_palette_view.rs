use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::{
    command_palette::CommandPaletteState,
    hit::{HitMap, HitTarget},
    theme, ui,
};

pub(crate) fn render(frame: &mut Frame<'_>, palette: &CommandPaletteState, hits: &mut HitMap) {
    let full = frame.area();
    frame.render_widget(Clear, full);
    frame.render_widget(Block::default().style(theme::page()), full);
    let area = popup_area(full);
    frame.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(theme::border_type())
        .border_style(theme::border(true))
        .title(format!(
            " 命令面板 · {} · {} / {} 条 ",
            palette.context,
            palette.filtered_len(),
            palette.entries.len()
        ))
        .title_style(theme::title(true))
        .style(theme::surface());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height < 4 {
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
                Span::styled(" / ", theme::brand()),
                Span::styled(palette.query.clone(), theme::accent()),
                Span::styled("▏", theme::accent().add_modifier(Modifier::BOLD)),
                Span::styled(
                    if palette.query.is_empty() {
                        "  输入中文或英文关键词"
                    } else {
                        ""
                    },
                    theme::dim(),
                ),
            ]),
            Line::from(Span::styled(
                "按意图分组；标题、说明、快捷键和英文别名都参与模糊搜索。",
                theme::dim(),
            )),
        ]),
        areas[0],
    );

    let mut items: Vec<ListItem<'_>> = Vec::new();
    let mut item_heights: Vec<u16> = Vec::new();
    let mut entry_rows: Vec<usize> = Vec::new();
    let mut entry_item_indices: Vec<usize> = Vec::new();
    let mut selected = None;
    let mut last_group = None;
    for (index, entry) in palette.filtered_entries().enumerate() {
        if last_group != Some(entry.group) {
            items.push(ListItem::new(Line::from(Span::styled(
                format!(" {}", entry.group),
                theme::dim().add_modifier(Modifier::BOLD),
            ))));
            item_heights.push(1);
            last_group = Some(entry.group);
        }
        entry_rows.push(
            item_heights
                .iter()
                .map(|height| *height as usize)
                .sum::<usize>(),
        );
        entry_item_indices.push(items.len());
        let shortcut = entry.shortcut.unwrap_or("");
        items.push(ListItem::new(vec![
            Line::from(vec![
                Span::styled(format!("  {}  ", entry.title), theme::text()),
                Span::styled(format!(" {shortcut} "), theme::keycap_alt()),
            ]),
            Line::from(Span::styled(
                format!("    {}", entry.detail),
                theme::muted(),
            )),
        ]));
        item_heights.push(2);
        if index == palette.selected {
            selected = Some(items.len() - 1);
        }
    }
    if items.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "  没有匹配命令；退格修改搜索，Esc 关闭。",
            theme::warning(),
        ))));
        item_heights.push(1);
    }
    let item_count = items.len();
    let mut state = ListState::default();
    state.select(selected.or(if items.is_empty() { None } else { Some(0) }));
    let list = List::new(items)
        .block(Block::default())
        .highlight_style(theme::selected_row())
        .highlight_symbol("▌ ")
        .scroll_padding(1);
    frame.render_stateful_widget(list, areas[1], &mut state);

    let offset = state.offset();
    let offset_row: usize = item_heights
        .iter()
        .take(offset)
        .map(|height| *height as usize)
        .sum();
    let visible_height = areas[1].height as usize;
    for (index, item_index) in entry_item_indices.iter().enumerate() {
        if *item_index < offset {
            continue;
        }
        let row = entry_rows[index].saturating_sub(offset_row);
        if row + 1 >= visible_height {
            break;
        }
        hits.push(
            Rect::new(areas[1].x, areas[1].y + row as u16, areas[1].width, 2),
            HitTarget::PaletteEntry(index),
        );
    }
    ui::scrollbar(frame, area, item_count, offset, visible_height);
    ui::scrollbar_hits(
        area,
        item_count,
        visible_height,
        hits,
        crate::hit::ScrollView::Palette,
    );

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("↑/↓", theme::key_hint()),
            Span::raw(" 选择 · "),
            Span::styled("PgUp/PgDn", theme::key_hint()),
            Span::raw(" 翻页 · "),
            Span::styled("Home/End", theme::key_hint()),
            Span::raw(" 首尾 · "),
            Span::styled("F1", theme::key_hint()),
            Span::raw(" 帮助 · "),
            Span::styled("Esc", theme::key_hint()),
            Span::raw(" 关闭"),
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
    let height = area.height.saturating_sub(2).clamp(6, 22);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height + 1);
    Rect::new(x, y, width, height)
}
