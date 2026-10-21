//! Error dialog view for rendering user-friendly error messages.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::{error_dialog::ErrorDialogState, theme, ui};

pub(crate) fn render(frame: &mut Frame<'_>, state: &ErrorDialogState) {
    let full = frame.area();

    // Dim the background
    frame.render_widget(Clear, full);
    frame.render_widget(Block::default().style(theme::page()), full);

    let area = dialog_area(full);
    frame.render_widget(Clear, area);

    let title = if state.message.retryable {
        " ⚠ 错误（可重试） | Error (Retryable) "
    } else {
        " ✗ 错误 | Error "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(theme::border_type())
        .border_style(theme::border(true))
        .title(title)
        .title_style(theme::danger().add_modifier(Modifier::BOLD))
        .style(theme::elevated());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 6 {
        return;
    }

    // Calculate layout
    let mut constraints = vec![
        Constraint::Length(3), // Summary
        Constraint::Length(3), // Reason
    ];

    if state.show_details && state.message.evidence.is_some() {
        constraints.push(Constraint::Length(4)); // Evidence (when shown)
    }

    if !state.message.actions.is_empty() {
        let actions_height = (state.message.actions.len() as u16).min(6) + 2;
        constraints.push(Constraint::Length(actions_height)); // Actions
    }

    constraints.push(Constraint::Length(3)); // Footer

    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    let mut area_idx = 0;

    // 1. Summary (what happened)
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled("发生了什么 | What Happened", theme::muted())),
            Line::from(Span::styled(
                state.message.summary.clone(),
                theme::danger().add_modifier(Modifier::BOLD),
            )),
        ]),
        areas[area_idx],
    );
    area_idx += 1;

    // 2. Reason (why it happened)
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled("原因 | Reason", theme::muted())),
            Line::from(state.message.reason.clone()),
        ])
        .wrap(Wrap { trim: true }),
        areas[area_idx],
    );
    area_idx += 1;

    // 3. Evidence (technical details, collapsible)
    if state.show_details && state.message.evidence.is_some() {
        if let Some(evidence) = &state.message.evidence {
            frame.render_widget(
                Paragraph::new(vec![
                    Line::from(Span::styled("技术细节 | Technical Details", theme::muted())),
                    Line::from(Span::styled(evidence.clone(), theme::dim())),
                ])
                .wrap(Wrap { trim: true }),
                areas[area_idx],
            );
        }
        area_idx += 1;
    }

    // 4. Actions (what to do next)
    if !state.message.actions.is_empty() {
        let mut lines = vec![Line::from(Span::styled(
            "建议操作 | Recommended Actions",
            theme::muted(),
        ))];

        for (i, action) in state.message.actions.iter().enumerate() {
            let is_selected = i == state.selected_action;
            let prefix = if is_selected { "▶ " } else { "  " };
            let style = if is_selected {
                theme::accent().add_modifier(Modifier::BOLD)
            } else {
                theme::text()
            };
            lines.push(Line::from(Span::styled(
                format!("{}{}. {}", prefix, i + 1, action),
                style,
            )));
        }

        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), areas[area_idx]);
        area_idx += 1;
    }

    // 5. Footer (keyboard hints)
    let details_hint = if state.message.evidence.is_some() {
        if state.show_details {
            vec![
                Span::styled("d", theme::key_hint()),
                Span::raw(" 隐藏技术细节 | Hide Details · "),
            ]
        } else {
            vec![
                Span::styled("d", theme::key_hint()),
                Span::raw(" 显示技术细节 | Show Details · "),
            ]
        }
    } else {
        vec![]
    };

    let mut footer_spans = details_hint;
    footer_spans.extend([
        Span::styled("c", theme::key_hint()),
        Span::raw(" 复制诊断报告 | Copy Report · "),
        Span::styled("r", theme::key_hint()),
        Span::raw(" 重试 | Retry · "),
        Span::styled("Esc", theme::key_hint()),
        Span::raw(" 关闭 | Close"),
    ]);

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(footer_spans),
        ]),
        areas[area_idx],
    );
}

fn dialog_area(area: Rect) -> Rect {
    let width = area.width.saturating_sub(8).clamp(40, 90);
    let height = area.height.saturating_sub(4).clamp(12, 30);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialog_area_calculation() {
        let full = Rect::new(0, 0, 100, 40);
        let dialog = dialog_area(full);
        assert!(dialog.width >= 40);
        assert!(dialog.width <= 90);
        assert!(dialog.height >= 12);
        assert!(dialog.height <= 30);
    }

    #[test]
    fn small_terminal_handling() {
        let full = Rect::new(0, 0, 50, 20);
        let dialog = dialog_area(full);
        assert!(dialog.width >= 40);
        assert!(dialog.height >= 12);
    }
}
