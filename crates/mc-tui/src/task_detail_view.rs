use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::{
    task_detail::TaskDetailState,
    task_presentation::{activity_symbol, kind_label, phase_label, status_label, status_style},
    theme,
};

pub(crate) fn render(frame: &mut Frame<'_>, detail: &TaskDetailState, animation_frame: u8) {
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
        Paragraph::new(Line::from(vec![
            Span::styled(" ◆ MC SERVER TUI ", theme::brand()),
            Span::styled(kind_label(detail.task.kind), theme::accent()),
        ]))
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(theme::border(false)),
        ),
        areas[0],
    );

    let has_backup_page = matches!(
        &detail.task.result,
        Some(mc_core::TaskResult::WorldBackupsListed { .. })
    );
    let summary_height = if compact {
        3
    } else if has_backup_page {
        8
    } else {
        7
    };
    let body = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(summary_height), Constraint::Min(1)])
        .split(areas[1]);
    render_summary(frame, body[0], detail, compact, animation_frame);
    render_events(frame, body[1], detail);

    let status = detail
        .error
        .as_deref()
        .map(|error| (format!("刷新失败，保留现有事件：{error}"), theme::danger()))
        .map(|(message, style)| Line::from(Span::styled(message, style)))
        .unwrap_or_else(|| {
            Line::from(vec![
                Span::styled("↑/↓", theme::key_hint()),
                Span::raw(" 翻阅事件 · "),
                Span::styled("r", theme::key_hint()),
                Span::raw(" 立即刷新 · "),
                Span::styled("Esc", theme::key_hint()),
                Span::raw(" 返回"),
            ])
        });
    frame.render_widget(
        Paragraph::new(vec![
            status,
            Line::from(vec![
                Span::styled(
                    format!("{} ", activity_symbol(detail.task.status, animation_frame)),
                    status_style(detail.task.status),
                ),
                Span::styled("当前游标 ", theme::muted()),
                Span::styled(format!("#{}", detail.last_event_sequence), theme::accent()),
                Span::raw(" · "),
                Span::styled(format!("{} 条事件", detail.events.len()), theme::muted()),
            ]),
        ]),
        areas[2],
    );
}

fn render_summary(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    detail: &TaskDetailState,
    compact: bool,
    animation_frame: u8,
) {
    let task = &detail.task;
    let progress = task
        .progress
        .map(|value| format!(" · {}/{}", value.completed_steps, value.total_steps))
        .unwrap_or_default();
    let mut lines = vec![Line::from(vec![
        Span::styled(
            format!("{} ", activity_symbol(task.status, animation_frame)),
            status_style(task.status),
        ),
        Span::styled(
            status_label(task.status),
            status_style(task.status).add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(" · {}{progress}", phase_label(task.phase))),
    ])];
    if compact {
        return frame.render_widget(
            Paragraph::new(lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(theme::border(false))
                    .title("任务状态")
                    .title_style(theme::title(false))
                    .style(theme::surface()),
            ),
            area,
        );
    } else {
        lines.push(Line::from(format!("任务 ID {}", task.id)));
        if let Some(error) = &task.error {
            lines.push(Line::from(task.message.clone()));
            let next_action = error
                .next_action
                .as_ref()
                .map(|action| format!(" · 下一步：{action}"))
                .unwrap_or_default();
            lines.push(Line::from(Span::styled(
                format!("错误：{}{next_action}", error.reason),
                theme::danger(),
            )));
        } else if let Some(mc_core::TaskResult::WorldBackupsListed { instance_id, page }) =
            &task.result
        {
            lines.push(Line::from(format!(
                "实例：{instance_id} · 本页 {} 个备份",
                page.backups.len()
            )));
            for backup in &page.backups {
                let short_id = backup.id.to_string();
                lines.push(Line::from(format!(
                    "{} · {} · {} 个文件 · {}",
                    &short_id[..8],
                    backup.created_at.format("%Y-%m-%d %H:%M UTC"),
                    backup.file_count,
                    format_bytes(backup.total_bytes)
                )));
            }
            if let Some(cursor) = &page.next_cursor {
                lines.push(Line::from(format!("下一页游标：{cursor}")));
            } else {
                lines.push(Line::from("没有更多备份"));
            }
        } else if let Some(result) = &task.result {
            lines.push(Line::from(task.message.clone()));
            match result {
                mc_core::TaskResult::InstanceCreated(instance) => {
                    lines.push(Line::from(format!("实例：{}", instance.name)));
                }
                mc_core::TaskResult::WorldBackupCreated {
                    instance_id,
                    backup_id,
                } => {
                    lines.push(Line::from(format!("实例：{instance_id}")));
                    lines.push(Line::from(format!("备份 ID：{backup_id}")));
                }
                mc_core::TaskResult::WorldBackupsListed { .. } => {}
                mc_core::TaskResult::WorldBackupVerified {
                    instance_id,
                    backup,
                } => {
                    lines.push(Line::from(format!("实例：{instance_id}")));
                    lines.push(Line::from(format!("已验证备份：{}", backup.id)));
                    lines.push(Line::from(format!(
                        "内容：{} 个文件 · {}",
                        backup.file_count,
                        format_bytes(backup.total_bytes)
                    )));
                }
                mc_core::TaskResult::WorldBackupRestorePlanned { instance_id, plan } => {
                    lines.push(Line::from(format!("实例：{instance_id}")));
                    lines.push(Line::from(format!("备份：{}", plan.backup_id)));
                    lines.push(Line::from(format!(
                        "备份时间：{} UTC",
                        plan.created_at.format("%Y-%m-%d %H:%M")
                    )));
                    lines.push(Line::from(format!(
                        "将替换：{}",
                        format_worlds(&plan.replaced_worlds)
                    )));
                    lines.push(Line::from(format!(
                        "将新增：{}",
                        format_worlds(&plan.added_worlds)
                    )));
                    lines.push(Line::from(format!(
                        "将移除：{}",
                        format_worlds(&plan.removed_worlds)
                    )));
                    lines.push(Line::from(format!(
                        "暂存空间：{} / 可用 {}",
                        format_bytes(plan.required_space_bytes),
                        format_bytes(plan.available_space_bytes)
                    )));
                    lines.push(Line::from(format!("确认令牌：{}", plan.confirmation_token)));
                }
                mc_core::TaskResult::WorldBackupRestored {
                    instance_id,
                    backup_id,
                } => {
                    lines.push(Line::from(format!("实例：{instance_id}")));
                    lines.push(Line::from(format!("已恢复备份：{backup_id}")));
                }
            }
        } else {
            lines.push(Line::from(task.message.clone()));
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
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme::border(false))
                .title("任务状态")
                .title_style(theme::title(false))
                .style(theme::surface()),
        ),
        area,
    );
}

fn format_bytes(bytes: u64) -> String {
    const MIB: u64 = 1024 * 1024;
    if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else {
        format!("{bytes} bytes")
    }
}

fn format_worlds(worlds: &[String]) -> String {
    if worlds.is_empty() {
        "无".to_owned()
    } else {
        worlds.join("、")
    }
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
                    Span::styled(format!("#{:<3} {time} ", event.sequence), theme::accent()),
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
                .border_style(theme::border(false))
                .title(format!("事件 · 游标 #{}", detail.last_event_sequence))
                .title_style(theme::title(false))
                .style(theme::surface()),
        ),
        area,
    );
}

#[cfg(test)]
mod tests;
