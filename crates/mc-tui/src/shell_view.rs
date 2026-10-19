use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::{
    app::AppState,
    hit::{HitMap, HitTarget},
    task_presentation::{kind_short, phase_label, status_label, status_style},
    theme, ui,
};

pub(crate) fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &AppState,
    animation_frame: u8,
    hits: &mut HitMap,
) {
    let compact = area.width < 80 || area.height < 19;
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(3),
            Constraint::Length(if compact { 3 } else { 5 }),
            Constraint::Length(1),
        ])
        .split(area);

    render_header(frame, root[0], app);
    let activity = app
        .tasks
        .iter()
        .find(|task| {
            matches!(
                task.status,
                mc_core::TaskStatus::Running | mc_core::TaskStatus::Queued
            )
        })
        .or_else(|| {
            app.tasks.iter().find(|task| {
                !matches!(
                    task.kind,
                    mc_core::TaskKind::ListWorldBackups
                        | mc_core::TaskKind::VerifyWorldBackup
                        | mc_core::TaskKind::PlanWorldBackupRestore
                ) && task
                    .finished_at
                    .is_some_and(|time| (chrono::Utc::now() - time).num_seconds() < 10)
            })
        });
    if let Some(task) = activity.filter(|_| root[1].height >= 10) {
        let body = Layout::vertical([Constraint::Length(6), Constraint::Min(1)]).split(root[1]);
        render_activity(frame, body[0], task, animation_frame);
        render_transcript(frame, body[1], app, hits);
    } else {
        render_transcript(frame, root[1], app, hits);
    }
    render_composer(frame, root[2], app, compact);
    render_dock(frame, root[3], app, animation_frame, compact);
    render_footer(frame, root[4], app);
    render_suggestions(frame, root[2], app, hits);
}

fn render_activity(frame: &mut Frame<'_>, area: Rect, task: &mc_core::TaskSnapshot, tick: u8) {
    let active = matches!(
        task.status,
        mc_core::TaskStatus::Queued | mc_core::TaskStatus::Running
    );
    let mut bar = ui::animated_progress(
        ui::task_ratio(task),
        area.width.saturating_sub(10) as usize,
        tick,
        active,
    );
    bar.push(Span::styled(
        if active && task.progress.as_ref().is_none_or(|p| p.transfer.is_none()) {
            "  …".to_owned()
        } else {
            format!(" {:>3}%", ui::task_percent(task))
        },
        status_style(task.status),
    ));
    let detail = if task.progress.as_ref().is_some_and(|p| p.transfer.is_some()) {
        ui::task_detail_label(task)
    } else if task.status == mc_core::TaskStatus::Succeeded {
        "操作已完成；结果已写入下方会话。".to_owned()
    } else if let Some(error) = &task.error {
        error.reason.clone()
    } else {
        format!("{} · 操作进行中…", task.message)
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![Span::styled(
                format!(
                    "{} {} · {}",
                    crate::task_presentation::activity_symbol(task.status, tick),
                    crate::task_presentation::kind_label(task.kind),
                    phase_label(task.phase)
                ),
                status_style(task.status),
            )]),
            Line::from(bar),
            Line::from(Span::styled(
                ui::truncate(&detail, area.width.saturating_sub(4) as usize),
                theme::muted(),
            )),
            Line::from(Span::styled(
                if active {
                    "/task 查看来源与事件 · /cancel 取消下载"
                } else {
                    "/tasks 查看任务历史"
                },
                theme::muted(),
            )),
        ])
        .block(
            Block::bordered()
                .title(" 下载与后台任务 ")
                .border_style(status_style(task.status))
                .style(theme::surface()),
        ),
        area,
    );
}

fn render_header(frame: &mut Frame<'_>, area: Rect, app: &AppState) {
    let (online, online_style) = app.connection_badge();
    let line = Line::from(vec![
        Span::styled(" ", theme::muted()),
        Span::styled(online, online_style),
        Span::styled("  ·  实例 ", theme::muted()),
        Span::styled(
            app.instances.len().to_string(),
            theme::accent().add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ·  任务 ", theme::muted()),
        Span::styled(
            app.tasks.len().to_string(),
            theme::accent().add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ·  ", theme::dim()),
        Span::styled(
            ui::truncate(&app.status, area.width.saturating_sub(30) as usize),
            if app.connected == Some(false) {
                theme::danger()
            } else {
                theme::muted()
            },
        ),
    ]);
    frame.render_widget(
        Paragraph::new(line).block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_type(theme::border_type())
                .border_style(theme::border(false)),
        ),
        area,
    );
}

fn render_transcript(frame: &mut Frame<'_>, area: Rect, app: &AppState, hits: &mut HitMap) {
    if area.height == 0 {
        return;
    }
    let text_width = area.width.saturating_sub(2) as usize;
    app.shell_text_width.set(text_width.saturating_sub(3));
    let lines = transcript_lines(app, text_width);
    let visible = area.height as usize;
    app.shell_viewport.set(visible);
    let max_scroll = lines.len().saturating_sub(visible);
    app.shell_max_scroll.set(max_scroll);
    let scroll = app.shell_scroll.min(max_scroll);
    let end = lines.len().saturating_sub(scroll);
    let start = end.saturating_sub(visible);
    let visible_lines: Vec<Line<'static>> = lines[start..end].to_vec();
    frame.render_widget(
        Paragraph::new(visible_lines).style(theme::page()),
        Rect::new(area.x, area.y, area.width.saturating_sub(1), area.height),
    );
    ui::scrollbar(frame, area, lines.len(), start, visible);
    ui::scrollbar_hits(
        area,
        lines.len(),
        visible,
        hits,
        crate::hit::ScrollView::Shell,
    );
}

fn transcript_lines(app: &AppState, width: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for (entry_index, entry) in app.transcript.iter().enumerate() {
        let (marker, marker_style, text_style) = match entry.role {
            crate::transcript::TranscriptRole::User => (
                "❯",
                theme::accent().add_modifier(Modifier::BOLD),
                theme::accent(),
            ),
            crate::transcript::TranscriptRole::Assistant => ("●", theme::muted(), theme::page()),
            crate::transcript::TranscriptRole::Success => ("✓", theme::success(), theme::success()),
            crate::transcript::TranscriptRole::Warning => ("!", theme::warning(), theme::warning()),
            crate::transcript::TranscriptRole::Error => ("×", theme::danger(), theme::danger()),
            crate::transcript::TranscriptRole::Muted => (" ", theme::dim(), theme::dim()),
        };
        for (line_index, text) in entry
            .text
            .lines()
            .flat_map(|line| ui::wrap_text(line, width.saturating_sub(3)))
            .enumerate()
        {
            let prefix = if line_index == 0 {
                Span::styled(format!(" {marker} "), marker_style)
            } else {
                Span::styled("   ", marker_style)
            };
            let text = if text.is_empty() {
                String::new()
            } else {
                ui::truncate(&text, width.saturating_sub(3))
            };
            if text.contains("INFO")
                || text.contains("WARN")
                || text.contains("ERROR")
                || text.contains("Exception")
            {
                let mut spans = vec![prefix];
                spans.extend(ui::log_line(&text, "").spans);
                lines.push(Line::from(spans));
            } else {
                lines.push(Line::from(vec![
                    prefix,
                    Span::styled(ui::clean_terminal_text(&text), text_style),
                ]));
            }
        }
        if entry_index + 1 < app.transcript.len() {
            lines.push(Line::from(""));
        }
    }
    if lines.is_empty() {
        lines.push(Line::from(Span::styled(" 会话已清空。", theme::dim())));
    }
    lines
}

fn render_composer(frame: &mut Frame<'_>, area: Rect, app: &AppState, compact: bool) {
    let input = &app.shell_input;
    let hint = if input.starts_with('/') {
        " Enter 执行 · Tab 补全 "
    } else if input.is_empty() {
        " Enter 发送 "
    } else {
        " Enter 发送到控制台 "
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(theme::border_type())
        .border_style(theme::border(false))
        .title(if compact {
            " 输入 "
        } else {
            " 会话输入 · 直接输入文本即控制台命令 "
        })
        .title_style(theme::title(false))
        .title_bottom(Line::from(Span::styled(hint, theme::dim())).alignment(Alignment::Right))
        .style(theme::surface());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 {
        return;
    }
    let mut spans = vec![Span::styled(
        " ❯ ",
        theme::accent().add_modifier(Modifier::BOLD),
    )];
    if app.shell_input.is_empty() {
        spans.push(Span::styled(
            "▏",
            theme::accent().add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            " 输入 / 命令，或直接输入控制台命令…",
            theme::dim(),
        ));
    } else {
        let available = inner.width.saturating_sub(4) as usize;
        let (before, after) = visible_input(&app.shell_input, app.shell_cursor, available);
        spans.push(Span::styled(before, theme::page()));
        spans.push(Span::styled(
            "▏",
            theme::accent().add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(after, theme::page()));
    }
    frame.render_widget(
        Paragraph::new(Line::from(spans)),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
}

/// Keep the caret visible by dropping leading characters when the line is
/// longer than the composer. This mirrors what a normal shell does.
fn visible_input(input: &str, cursor: usize, available: usize) -> (String, String) {
    let cursor = cursor.min(input.len());
    let before = &input[..cursor];
    let after = &input[cursor..];
    let mut start = 0;
    let mut before_width = ui::display_width(before);
    while before_width + 1 > available && start < before.len() {
        let character = before[start..]
            .chars()
            .next()
            .expect("start is a char boundary");
        start += character.len_utf8();
        before_width = before_width.saturating_sub(ui::display_width(&character.to_string()));
    }
    let visible_before = before[start..].to_owned();
    let remaining = available.saturating_sub(before_width + 1);
    let visible_after = ui::truncate(after, remaining);
    (visible_before, visible_after)
}

fn render_suggestions(frame: &mut Frame<'_>, composer: Rect, app: &AppState, hits: &mut HitMap) {
    let suggestions = app.shell_suggestions();
    if suggestions.is_empty() {
        return;
    }
    let height = suggestions.len() as u16 + 2;
    let width = composer.width.saturating_sub(4).clamp(24, 74);
    let x = composer.x + composer.width.saturating_sub(width) / 2;
    let y = composer.y.saturating_sub(height);
    let area = Rect::new(x, y, width, height);
    frame.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(theme::border_type())
        .border_style(theme::border(false))
        .title(" 命令补全 ")
        .title_style(theme::title(false))
        .title_bottom(
            Line::from(vec![
                Span::styled(" Enter ", theme::key_hint()),
                Span::styled("执行  ", theme::muted()),
                Span::styled(" Tab ", theme::key_hint()),
                Span::styled("填入  ", theme::muted()),
                Span::styled(" ↑↓ ", theme::key_hint()),
                Span::styled("选择 ", theme::muted()),
            ])
            .alignment(Alignment::Right),
        )
        .style(theme::surface());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    for (index, command) in suggestions.iter().enumerate() {
        if index >= inner.height as usize {
            break;
        }
        let selected = index
            == app
                .shell_suggestion_index
                .min(suggestions.len().saturating_sub(1));
        let style = if selected {
            theme::selected_row()
        } else {
            theme::text()
        };
        let invocation = if command.args.is_empty() {
            format!("/{}", command.name)
        } else {
            format!("/{} {}", command.name, command.args)
        };
        let available = inner.width.saturating_sub(invocation.len() as u16 + 6) as usize;
        let row = Line::from(vec![
            Span::styled(format!(" {:<20}", ui::truncate(&invocation, 20)), style),
            Span::styled(ui::truncate(command.summary, available), theme::muted()),
        ]);
        let row_area = Rect::new(inner.x, inner.y + index as u16, inner.width, 1);
        frame.render_widget(Paragraph::new(row), row_area);
        hits.push(row_area, HitTarget::ShellSuggestion(index));
    }
}

fn render_dock(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &AppState,
    animation_frame: u8,
    compact: bool,
) {
    if compact {
        frame.render_widget(
            Paragraph::new(dock_info_lines(app, animation_frame, true, area.width)),
            area,
        );
        return;
    }
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(11), Constraint::Min(1)])
        .split(area);
    render_mascot(frame, columns[0], animation_frame, app.has_animated_work());
    frame.render_widget(
        Paragraph::new(dock_info_lines(
            app,
            animation_frame,
            false,
            columns[1].width,
        )),
        columns[1],
    );
}

fn dock_info_lines(
    app: &AppState,
    animation_frame: u8,
    compact: bool,
    width: u16,
) -> Vec<Line<'static>> {
    let mut lines = vec![instance_line(app)];
    let active: Vec<&mc_core::TaskSnapshot> = app
        .tasks
        .iter()
        .filter(|task| {
            matches!(
                task.status,
                mc_core::TaskStatus::Queued | mc_core::TaskStatus::Running
            )
        })
        .collect();
    if let Some(task) = active.first() {
        lines.push(task_line(task, width, animation_frame));
    } else {
        lines.push(Line::from(vec![
            Span::styled(" ○ ", theme::muted()),
            Span::styled("后台任务空闲 · /tasks 查看历史任务", theme::dim()),
        ]));
    }
    if !compact {
        if let Some(task) = active.get(1) {
            lines.push(task_line(task, width, animation_frame));
        } else if active.len() > 1 {
            lines.push(Line::from(Span::styled(
                format!("  另有 {} 个后台任务 · /tasks", active.len() - 1),
                theme::dim(),
            )));
        } else {
            let last = app
                .tasks
                .iter()
                .filter(|task| {
                    !matches!(
                        task.kind,
                        mc_core::TaskKind::ListWorldBackups
                            | mc_core::TaskKind::VerifyWorldBackup
                            | mc_core::TaskKind::PlanWorldBackupRestore
                    )
                })
                .find(|task| {
                    matches!(
                        task.status,
                        mc_core::TaskStatus::Succeeded
                            | mc_core::TaskStatus::Failed
                            | mc_core::TaskStatus::Cancelled
                    )
                });
            if let Some(task) = last {
                lines.push(Line::from(vec![
                    Span::styled(" 最近任务  ", theme::dim()),
                    Span::styled(kind_short(task.kind), theme::muted()),
                    Span::styled(
                        format!(" · {} · {}", status_label(task.status), task.message),
                        theme::dim(),
                    ),
                ]));
            } else {
                lines.push(Line::from(Span::styled(
                    " 提示：/use 选择实例 · /open 查看日志 · /backups 管理世界备份",
                    theme::dim(),
                )));
            }
        }
    }
    lines.push(resource_line(app, compact));
    if !compact {
        lines.push(quick_actions_line(app));
    }
    let width = width as usize;
    lines
        .into_iter()
        .map(|line| ui::clamp_line(line, width))
        .collect()
}

fn task_line(task: &mc_core::TaskSnapshot, width: u16, animation_frame: u8) -> Line<'static> {
    let ratio = ui::task_ratio(task);
    let percent = (ratio * 100.0).round() as u8;
    let style = status_style(task.status);
    let mut spans = vec![
        Span::styled(
            format!(
                " {} ",
                crate::task_presentation::activity_symbol(task.status, animation_frame)
            ),
            style,
        ),
        Span::styled(
            format!("{} · ", kind_short(task.kind)),
            theme::text().add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("{}  ", phase_label(task.phase)), theme::muted()),
    ];
    let bar_width = if width < 90 { 8 } else { 14 };
    spans.extend(ui::animated_progress(
        ratio,
        bar_width,
        animation_frame,
        task.status == mc_core::TaskStatus::Running,
    ));
    spans.push(Span::styled(
        format!("  {percent:>3}%  "),
        if percent >= 100 {
            theme::success()
        } else {
            theme::accent()
        },
    ));
    let detail = task_dock_detail(task);
    let reserved = 27usize + bar_width;
    spans.push(Span::styled(
        ui::truncate(&detail, width.saturating_sub(reserved as u16) as usize),
        theme::muted(),
    ));
    Line::from(spans)
}

fn task_dock_detail(task: &mc_core::TaskSnapshot) -> String {
    if let Some(transfer) = task
        .progress
        .as_ref()
        .and_then(|progress| progress.transfer.as_ref())
    {
        let rate = transfer
            .bytes_per_second
            .map(ui::format_rate)
            .unwrap_or_else(|| "测速中".to_owned());
        format!(
            "{} / {} · {}",
            ui::format_bytes(transfer.downloaded_bytes),
            ui::format_bytes(transfer.total_bytes),
            rate
        )
    } else {
        task.message.clone()
    }
}

fn instance_line(app: &AppState) -> Line<'static> {
    let Some(instance) = app.instances.get(app.selected_instance) else {
        return Line::from(vec![
            Span::styled(" ▸ ", theme::accent()),
            Span::styled(
                if app
                    .tasks
                    .iter()
                    .any(crate::task_presentation::is_cancelable_download)
                {
                    "正在准备新服务器"
                } else {
                    "还没有服务器"
                },
                theme::text().add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                if app
                    .tasks
                    .iter()
                    .any(crate::task_presentation::is_cancelable_download)
                {
                    "  ·  /tasks 查看进度 · /cancel 取消"
                } else {
                    "  ·  输入 /create 新建 Paper 服务器"
                },
                theme::dim(),
            ),
        ]);
    };
    let server = app
        .instance_detail
        .as_ref()
        .and_then(|detail| detail.detail.as_ref())
        .map(|detail| &detail.server);
    let state = server
        .map(|server| {
            (
                ui::server_state_label(server.state),
                ui::server_state_style(server.state),
            )
        })
        .unwrap_or(("未加载", theme::dim()));
    Line::from(vec![
        Span::styled(" ▸ ", theme::accent()),
        Span::styled(
            instance.name.clone(),
            theme::text().add_modifier(Modifier::BOLD),
        ),
        Span::styled(" · ", theme::dim()),
        Span::styled(state.0, state.1),
        Span::styled(
            format!(
                "  {} {}  ·  :{}",
                instance.provider.to_ascii_lowercase(),
                instance.game_version,
                instance.port
            ),
            theme::muted(),
        ),
    ])
}

fn quick_actions_line(app: &AppState) -> Line<'static> {
    if app.instances.is_empty() {
        return Line::from(vec![
            Span::styled(" 操作  ", theme::dim()),
            Span::styled("/create", theme::key_hint()),
            Span::styled(" 新建  · ", theme::dim()),
            Span::styled("/import", theme::key_hint()),
            Span::styled(" 导入整合包  · ", theme::dim()),
            Span::styled("/help", theme::key_hint()),
            Span::styled(" 命令", theme::dim()),
        ]);
    }
    let running = app
        .instance_detail
        .as_ref()
        .and_then(|detail| detail.detail.as_ref())
        .is_some_and(|detail| {
            matches!(
                detail.server.state,
                mc_core::ServerState::Running
                    | mc_core::ServerState::Starting
                    | mc_core::ServerState::Stopping
            )
        });
    Line::from(vec![
        Span::styled(" 操作  ", theme::dim()),
        Span::styled(if running { "/stop" } else { "/start" }, theme::key_hint()),
        Span::styled("  ·  ", theme::dim()),
        Span::styled("/open", theme::key_hint()),
        Span::styled("  ·  ", theme::dim()),
        Span::styled("/logs", theme::key_hint()),
        Span::styled("  ·  ", theme::dim()),
        Span::styled("/backups", theme::key_hint()),
        Span::styled("  ·  ", theme::dim()),
        Span::styled("/tasks", theme::key_hint()),
        if app
            .tasks
            .iter()
            .any(crate::task_presentation::is_cancelable_download)
        {
            Span::styled("  ·  ", theme::dim())
        } else {
            Span::styled("", theme::dim())
        },
        if app
            .tasks
            .iter()
            .any(crate::task_presentation::is_cancelable_download)
        {
            Span::styled("/cancel", theme::key_hint())
        } else {
            Span::styled("", theme::dim())
        },
    ])
}

fn resource_line(app: &AppState, compact: bool) -> Line<'static> {
    let Some(instance) = app.instances.get(app.selected_instance) else {
        return Line::from(Span::styled(
            " 资源  —  ·  选中实例后显示进程占用",
            theme::dim(),
        ));
    };
    let server = app
        .instance_detail
        .as_ref()
        .and_then(|detail| detail.detail.as_ref())
        .map(|detail| &detail.server);
    let stats = app
        .instance_detail
        .as_ref()
        .and_then(|detail| detail.stats.as_ref());
    let mut spans = vec![Span::styled(" 资源  ", theme::dim())];
    match (server.map(|server| server.state), stats) {
        (Some(mc_core::ServerState::Running), Some(stats)) => {
            spans.push(Span::styled(
                format!("CPU {:.1}%", stats.cpu_percent),
                theme::accent(),
            ));
            spans.push(Span::styled(
                format!("  ·  RSS {}", ui::format_bytes(stats.rss_bytes)),
                theme::muted(),
            ));
        }
        (Some(mc_core::ServerState::Running), None) => {
            spans.push(Span::styled("CPU —  ·  RSS —", theme::dim()));
        }
        (Some(state), _) if state != mc_core::ServerState::Unknown => {
            spans.push(Span::styled(ui::server_state_label(state), theme::muted()));
        }
        _ => {
            spans.push(Span::styled("状态未加载", theme::dim()));
        }
    }
    if !compact {
        spans.push(Span::styled(
            format!(
                "  ·  分配内存 {}-{} MiB",
                instance.min_memory_mb, instance.max_memory_mb
            ),
            theme::dim(),
        ));
    }
    Line::from(spans)
}

fn render_mascot(frame: &mut Frame<'_>, area: Rect, animation_frame: u8, active: bool) {
    // Steve's 8×8 face, two pixel rows per terminal cell; half blocks preserve
    // the square silhouette, brown hair/beard, blue eyes and broad nose.
    const PIXELS: [&str; 8] = [
        "hhhhhhhh", "hhhhhhhh", "hssssssh", "ssssssss", "swessews", "ssnnnnss", "ssdbbdss",
        "sssbbsss",
    ];
    let color = |pixel| match pixel {
        'h' => Color::Rgb(54, 37, 25),
        's' | ' ' => Color::Rgb(181, 136, 108),
        'w' => Color::Rgb(237, 234, 225),
        'e' => Color::Rgb(75, 65, 145),
        'n' => Color::Rgb(156, 105, 78),
        'd' => Color::Rgb(106, 69, 46),
        _ => Color::Rgb(67, 43, 29),
    };
    let mut lines = vec![Line::from(Span::styled(
        if active {
            ["·", "•", "●", "•"][usize::from(animation_frame) % 4]
        } else {
            "STEVE"
        },
        theme::muted(),
    ))];
    for pair in 0..4 {
        let top: Vec<_> = PIXELS[pair * 2].chars().take(8).collect();
        let bottom: Vec<_> = PIXELS[pair * 2 + 1].chars().take(8).collect();
        let spans = (0..8)
            .map(|x| {
                let upper = if pair == 2 && (x == 2 || x == 5) && animation_frame % 40 < 2 {
                    's'
                } else {
                    top[x]
                };
                Span::styled("▀", Style::default().fg(color(upper)).bg(color(bottom[x])))
            })
            .collect::<Vec<_>>();
        lines.push(Line::from(spans));
    }
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), area);
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &AppState) {
    let line = if app.shell_scroll > 0 {
        ui::plain_hint_line(&[
            ("PgDn/Ctrl-End", "回到最新"),
            ("PgUp", "继续上滚"),
            ("Enter", "发送"),
            ("F1", "帮助"),
        ])
    } else {
        ui::plain_hint_line(&[
            ("Enter", "发送"),
            ("Tab", "补全"),
            ("←/→", "光标"),
            ("↑/↓", "历史"),
            ("F1", "帮助"),
        ])
    };
    frame.render_widget(Paragraph::new(line).style(theme::dim()), area);
}

#[cfg(test)]
mod tests;
