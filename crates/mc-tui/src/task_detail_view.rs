use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::{
    task_detail::TaskDetailState,
    task_presentation::{kind_label, phase_label, status_label, status_style},
};

pub(crate) fn render(frame: &mut Frame<'_>, detail: &TaskDetailState) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(frame.area());
    let compact = frame.area().width < 80 || frame.area().height < 18;
    frame.render_widget(
        Paragraph::new(format!(" MC SERVER TUI   {}", kind_label(detail.task.kind)))
            .block(Block::default().borders(Borders::BOTTOM)),
        areas[0],
    );

    let summary_height = if compact { 3 } else { 7 };
    let body = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(summary_height), Constraint::Min(1)])
        .split(areas[1]);
    render_summary(frame, body[0], detail, compact);
    render_events(frame, body[1], detail);

    let status = detail
        .error
        .as_deref()
        .map(|error| {
            (
                format!("刷新失败，保留现有事件：{error}"),
                Style::default().fg(Color::Red),
            )
        })
        .unwrap_or_else(|| {
            (
                "↑/↓ 翻阅事件 · r 立即刷新 · Esc 返回".to_owned(),
                Style::default().fg(Color::DarkGray),
            )
        });
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(status.0, status.1)),
            Line::from(format!(
                "当前游标 #{} · {} 条事件",
                detail.last_event_sequence,
                detail.events.len()
            )),
        ]),
        areas[2],
    );
}

fn render_summary(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    detail: &TaskDetailState,
    compact: bool,
) {
    let task = &detail.task;
    let progress = task
        .progress
        .map(|value| format!(" · {}/{}", value.completed_steps, value.total_steps))
        .unwrap_or_default();
    let mut lines = vec![Line::from(vec![
        Span::styled(
            status_label(task.status),
            status_style(task.status).add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(" · {}{progress}", phase_label(task.phase))),
    ])];
    if compact {
        return frame.render_widget(
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("任务状态")),
            area,
        );
    } else {
        lines.push(Line::from(format!("任务 ID {}", task.id)));
        lines.push(Line::from(task.message.clone()));
        if let Some(error) = &task.error {
            let next_action = error
                .next_action
                .as_ref()
                .map(|action| format!(" · 下一步：{action}"))
                .unwrap_or_default();
            lines.push(Line::from(Span::styled(
                format!("错误：{}{next_action}", error.reason),
                Style::default().fg(Color::Red),
            )));
        } else if let Some(result) = &task.result {
            match result {
                mc_core::TaskResult::InstanceCreated(instance) => {
                    lines.push(Line::from(format!("实例：{}", instance.name)));
                }
            }
        } else {
            lines.push(Line::from(format!(
                "创建于 {} UTC",
                task.created_at.format("%Y-%m-%d %H:%M:%S")
            )));
        }
        lines.push(Line::from(format!(
            "最后更新 {} UTC",
            task.updated_at.format("%H:%M:%S")
        )));
    }
    frame.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("任务状态")),
        area,
    );
}

fn render_events(frame: &mut Frame<'_>, area: ratatui::layout::Rect, detail: &TaskDetailState) {
    let visible_rows = area.height.saturating_sub(2) as usize;
    let end = detail.events.len().saturating_sub(detail.scroll_offset);
    let start = end.saturating_sub(visible_rows);
    let lines = if detail.events.is_empty() {
        vec![Line::from("等待首个任务事件…")]
    } else {
        detail.events[start..end]
            .iter()
            .map(|event| {
                let time = event.occurred_at.format("%H:%M:%S");
                let message = event
                    .error
                    .as_ref()
                    .map(|error| format!("{} · {}", event.message, error.reason))
                    .unwrap_or_else(|| event.message.clone());
                Line::from(vec![
                    Span::styled(
                        format!("#{:<3} {time} ", event.sequence),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::styled(
                        format!(
                            "{} · {} · ",
                            status_label(event.status),
                            phase_label(event.phase)
                        ),
                        status_style(event.status),
                    ),
                    Span::raw(message),
                ])
            })
            .collect()
    };
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("事件 · 游标 #{}", detail.last_event_sequence)),
        ),
        area,
    );
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use mc_core::{TaskEvent, TaskKind, TaskPhase, TaskProgress, TaskSnapshot, TaskStatus};
    use ratatui::{backend::TestBackend, Terminal};
    use uuid::Uuid;

    use crate::task_detail::TaskDetailState;

    use super::render;

    #[test]
    fn renders_task_progress_and_event_cursor_at_reference_and_compact_sizes() {
        let mut detail = TaskDetailState::new(task_snapshot());
        detail.events.push(TaskEvent {
            sequence: 1,
            task_id: detail.task.id,
            occurred_at: Utc::now(),
            status: TaskStatus::Running,
            phase: TaskPhase::DownloadingArtifact,
            message: "Paper artifact verified".to_owned(),
            progress: Some(TaskProgress {
                completed_steps: 1,
                total_steps: 3,
            }),
            error: None,
        });
        detail.last_event_sequence = 1;

        for (width, height) in [(80, 24), (40, 12)] {
            let text = render_to_text(&detail, width, height);
            assert!(text.contains("进行中"));
            assert!(text.contains("事件"));
        }
        assert!(render_to_text(&detail, 80, 24).contains("Paperartifactverified"));
    }

    fn task_snapshot() -> TaskSnapshot {
        TaskSnapshot {
            id: Uuid::new_v4(),
            kind: TaskKind::CreatePaperInstance,
            instance_id: None,
            status: TaskStatus::Running,
            phase: TaskPhase::DownloadingArtifact,
            message: "Downloading and verifying Paper".to_owned(),
            progress: Some(TaskProgress {
                completed_steps: 1,
                total_steps: 3,
            }),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            finished_at: None,
            result: None,
            error: None,
            last_event_sequence: 1,
        }
    }

    fn render_to_text(detail: &TaskDetailState, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|frame| render(frame, detail))
            .expect("render");
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .filter(|symbol| !symbol.chars().all(char::is_whitespace))
            .collect()
    }
}
