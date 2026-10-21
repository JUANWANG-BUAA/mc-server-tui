//! Scrollable failure details with a footer that stays visible on small terminals.
use crate::{error_dialog::ErrorDialogState, theme, ui};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub(crate) fn render(frame: &mut Frame<'_>, state: &ErrorDialogState) {
    let area = dialog_area(frame.area());
    frame.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(theme::border_type())
        .border_style(theme::danger())
        .title(" 操作失败 ")
        .style(theme::elevated());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let rows = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(inner);
    let mut lines = vec![Line::from(Span::styled(
        state.message.summary.clone(),
        theme::danger().add_modifier(Modifier::BOLD),
    ))];
    let mut add_text = |heading: &str, text: &str| {
        lines.push(Line::from(Span::styled(heading.to_owned(), theme::muted())));
        for paragraph in text.lines() {
            lines.extend(
                ui::wrap_text(paragraph, inner.width as usize)
                    .into_iter()
                    .map(Line::from),
            );
        }
    };
    add_text("原因", &state.message.reason);
    if !state.message.actions.is_empty() {
        add_text(
            "下一步",
            &state
                .message
                .actions
                .iter()
                .enumerate()
                .map(|(i, action)| format!("{}. {action}", i + 1))
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
    if state.show_details {
        if let Some(evidence) = &state.message.evidence {
            add_text("技术细节", evidence);
        }
    }
    let max_scroll = lines.len().saturating_sub(rows[0].height as usize);
    state.max_scroll.set(max_scroll);
    let start = state.scroll.min(max_scroll);
    let visible: Vec<_> = lines
        .into_iter()
        .skip(start)
        .take(rows[0].height as usize)
        .collect();
    frame.render_widget(Paragraph::new(visible), rows[0]);
    frame.render_widget(
        Paragraph::new(ui::clamp_line(
            Line::from("↑↓ 滚动 · d 详情 · Esc 关闭"),
            rows[1].width as usize,
        ))
        .style(theme::muted()),
        rows[1],
    );
}

fn dialog_area(area: Rect) -> Rect {
    let width = area.width.saturating_sub(4).min(90).max(area.width.min(20));
    let height = area
        .height
        .saturating_sub(2)
        .min(28)
        .max(area.height.min(8));
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dialog_stays_inside_small_and_large_terminals() {
        for (width, height) in [(1, 1), (20, 6), (40, 12), (80, 24), (120, 40)] {
            let full = Rect::new(0, 0, width, height);
            let dialog = dialog_area(full);
            assert_eq!(dialog.intersection(full), dialog);
        }
    }
}
