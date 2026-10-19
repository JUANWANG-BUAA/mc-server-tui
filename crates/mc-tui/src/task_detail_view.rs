use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::{
    hit::HitMap,
    task_detail::TaskDetailState,
    task_presentation::{activity_symbol, kind_label, phase_label, status_label, status_style},
    theme, ui,
};

pub(crate) fn render(
    frame: &mut Frame<'_>,
    detail: &TaskDetailState,
    animation_frame: u8,
    _hits: &mut HitMap,
) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(frame.area());
    let compact = frame.area().width < 80 || frame.area().height < 18;
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" ◆ MC SERVER TUI ", theme::brand()),
            Span::styled("任务 · ", theme::dim()),
            Span::styled(
                kind_label(detail.task.kind),
                theme::text().add_modifier(Modifier::BOLD),
            ),
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
    let has_transfer = detail
        .task
        .progress
        .as_ref()
        .is_some_and(|progress| progress.transfer.is_some());
    let summary_height = if compact {
        if has_transfer {
            4
        } else {
            3
        }
    } else if has_transfer {
        10
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
                Span::raw(" 翻阅 · "),
                Span::styled("PgUp/PgDn", theme::key_hint()),
                Span::raw(" 翻页 · "),
                Span::styled("r", theme::key_hint()),
                Span::raw(" 刷新 · "),
                Span::styled("d", theme::key_hint()),
                Span::raw(" 主页 · "),
                Span::styled("?", theme::key_hint()),
                Span::raw(" 帮助 · "),
                Span::styled("Esc", theme::key_hint()),
                Span::raw(" 返回"),
            ])
        });
    let task_hint = if let Some(notice) = &detail.notice {
        Line::from(Span::styled(notice.clone(), theme::warning()))
    } else if detail.can_cancel() {
        if detail.cancel_armed {
            Line::from(vec![
                Span::styled("再按 x", theme::danger().add_modifier(Modifier::BOLD)),
                Span::raw(" 确认取消下载 · "),
                Span::styled("Esc", theme::key_hint()),
                Span::raw(" 返回"),
            ])
        } else {
            Line::from(vec![
                Span::styled("x", theme::key_hint()),
                Span::raw(" 取消下载并保留可续传临时文件 · "),
                Span::styled("Esc", theme::key_hint()),
                Span::raw(" 返回"),
            ])
        }
    } else {
        Line::from(Span::styled(
            "/ 命令 · d 主页 · ? 帮助 · q 退出",
            theme::muted(),
        ))
    };
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
            task_hint,
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
        .clone()
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
    let transfer = task
        .progress
        .as_ref()
        .and_then(|progress| progress.transfer.as_ref());
    if let Some(transfer) = transfer {
        lines.push(transfer_progress_line(transfer, compact));
        if !compact {
            lines.push(Line::from(vec![
                Span::styled("来源  ", theme::muted()),
                Span::styled(transfer.source.clone(), theme::accent()),
            ]));
        }
    }
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
    let position = detail
        .events
        .len()
        .saturating_sub(1)
        .saturating_sub(detail.scroll_offset);
    ui::scrollbar(
        frame,
        area,
        detail.events.len(),
        position,
        visible_rows.max(1),
    );
}

#[cfg(test)]
mod tests;

fn transfer_progress_line(progress: &mc_core::TransferProgress, compact: bool) -> Line<'static> {
    let ratio = if progress.total_bytes == 0 {
        0.0
    } else {
        (progress.downloaded_bytes as f64 / progress.total_bytes as f64).clamp(0.0, 1.0)
    };
    let width = if compact { 12 } else { 18 };
    let filled = ((ratio * width as f64).round() as usize).min(width);
    let bar = format!("{}{}", "█".repeat(filled), "░".repeat(width - filled),);
    let percent = (ratio * 100.0).round() as u8;
    if compact {
        Line::from(vec![
            Span::styled("下载 ", theme::muted()),
            Span::styled(bar, theme::progress()),
            Span::styled(format!(" {percent}%"), theme::accent()),
        ])
    } else {
        let speed = progress
            .bytes_per_second
            .map(format_transfer_rate)
            .unwrap_or_else(|| "速度计算中".to_owned());
        let eta = progress
            .bytes_per_second
            .filter(|rate| *rate > 0)
            .map(|rate| {
                format!(
                    " · 剩余 {}",
                    ui::format_eta(
                        progress
                            .total_bytes
                            .saturating_sub(progress.downloaded_bytes),
                        rate
                    )
                )
            })
            .unwrap_or_default();
        let mut spans = vec![Span::styled("下载  ", theme::muted())];
        spans.extend(ui::progress_bar(ratio, width, theme::progress()));
        spans.push(Span::styled(format!(" {percent:>3}%"), theme::accent()));
        spans.push(Span::styled(
            format!(
                " · {} / {} · {speed}{eta}",
                format_bytes(progress.downloaded_bytes),
                format_bytes(progress.total_bytes)
            ),
            theme::muted(),
        ));
        Line::from(spans)
    }
}

fn format_transfer_rate(bytes: u64) -> String {
    format!("{}/s", format_bytes(bytes))
}
