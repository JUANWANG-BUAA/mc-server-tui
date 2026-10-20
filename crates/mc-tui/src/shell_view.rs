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
    shell,
    task_presentation::{kind_short, phase_label, status_style},
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
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
            Constraint::Length(1),
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
    if let Some(task) = activity.filter(|_| root[1].height >= 3) {
        let body = Layout::vertical([Constraint::Length(2), Constraint::Min(1)]).split(root[1]);
        render_activity(frame, body[0], app, task, animation_frame);
        render_transcript(frame, body[1], app, hits);
    } else {
        render_transcript(frame, root[1], app, hits);
    }
    render_composer(frame, root[2], app, compact);
    let connection = match app.connected {
        Some(true) => "管理服务已连接",
        Some(false) => "管理服务已断开 · 显示上次状态；/refresh 重连",
        None => "管理服务连接中…",
    };
    let status = if app.status == "已连接 · 自动刷新中"
        || app.status == "正在连接后台服务…"
        || app.connected == Some(false)
    {
        connection.to_owned()
    } else {
        format!("{} · {connection}", app.status)
    };
    frame.render_widget(
        Paragraph::new(status).style(if app.connected == Some(false) {
            theme::warning()
        } else {
            theme::muted()
        }),
        root[3],
    );
    render_footer(frame, root[4], app);
    render_suggestions(frame, root[2], app, hits);
}

fn render_activity(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &AppState,
    task: &mc_core::TaskSnapshot,
    tick: u8,
) {
    let target = task
        .instance_id
        .and_then(|id| app.instances.iter().find(|instance| instance.id == id))
        .map(|instance| instance.name.as_str())
        .unwrap_or("新服务器 / 后台任务");
    let number = app
        .tasks
        .iter()
        .position(|candidate| candidate.id == task.id)
        .unwrap_or(0)
        + 1;
    let title = format!(
        "{} · /task {number} 查看详情{}",
        target,
        if crate::task_presentation::is_cancelable_download(task) {
            " · /cancel"
        } else {
            ""
        }
    );
    frame.render_widget(
        Paragraph::new(vec![
            task_line(task, area.width, tick),
            Line::from(Span::styled(
                ui::truncate(&title, area.width as usize),
                theme::muted(),
            )),
        ]),
        area,
    );
}

fn selected_server(app: &AppState) -> Option<&mc_core::ServerSnapshot> {
    let instance = app.instances.get(app.selected_instance)?;
    app.instance_detail
        .as_ref()
        .and_then(|detail| detail.detail.as_ref())
        .map(|detail| &detail.server)
        .filter(|server| server.instance_id == instance.id)
        .or_else(|| app.servers.get(&instance.id))
}

fn render_header(frame: &mut Frame<'_>, area: Rect, app: &AppState) {
    let lines = vec![
        instance_line(app),
        quick_actions_line(app),
        resource_line(app, area.width < 80),
    ];
    frame.render_widget(
        Paragraph::new(
            lines
                .into_iter()
                .map(|line| ui::clamp_line(line, area.width as usize))
                .collect::<Vec<_>>(),
        ),
        area,
    );
}

fn render_transcript(frame: &mut Frame<'_>, area: Rect, app: &AppState, hits: &mut HitMap) {
    if area.height == 0 {
        return;
    }
    let area = if app.instances.is_empty()
        && app.tasks.is_empty()
        && area.width >= 80
        && area.height >= 10
    {
        let columns = Layout::horizontal([Constraint::Length(11), Constraint::Min(1)]).split(area);
        render_mascot(frame, columns[0], 2, false);
        columns[1]
    } else {
        area
    };
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

fn render_composer(frame: &mut Frame<'_>, area: Rect, app: &AppState, _compact: bool) {
    let input = &app.shell_input;
    let hint = if input.starts_with('/') {
        if needs_completion_confirmation(app) {
            " Enter 补全后再次确认 · Tab 填入 "
        } else {
            " Enter 执行 · Tab 填入 "
        }
    } else if input.is_empty() {
        " Enter 发送 "
    } else {
        " Enter 发送到控制台 "
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(theme::border_type())
        .border_style(theme::border(false))
        .title(composer_title(app, area.width))
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

fn needs_completion_confirmation(app: &AppState) -> bool {
    let input = app.shell_input.trim();
    let Some(name) = input.strip_prefix('/') else {
        return false;
    };
    !name.contains(char::is_whitespace)
        && shell::find(name).is_none()
        && !app.shell_suggestions().is_empty()
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
    let enter_hint = if needs_completion_confirmation(app) {
        "先补全；再次 Enter 执行  "
    } else {
        "执行  "
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(theme::border_type())
        .border_style(theme::border(false))
        .title(" 命令补全 ")
        .title_style(theme::title(false))
        .title_bottom(
            Line::from(vec![
                Span::styled(" Enter ", theme::key_hint()),
                Span::styled(enter_hint, theme::muted()),
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
    if task
        .progress
        .as_ref()
        .is_some_and(|progress| progress.transfer.is_some())
    {
        spans.extend(ui::progress_bar(ratio, bar_width, theme::progress()));
        spans.push(Span::styled(format!("  {percent:>3}%  "), theme::accent()));
    } else {
        spans.push(Span::styled(
            format!("{} · ", crate::task_presentation::status_label(task.status)),
            theme::muted(),
        ));
    }
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
    let server = selected_server(app);
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
            ui::truncate(&instance.name, 24),
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
    let actions = if app.instances.is_empty() {
        if app.has_animated_work() {
            "/tasks 查看建服进度 · /help 帮助"
        } else {
            "/create 创建服务器 · /help 帮助"
        }
    } else {
        match selected_server(app).map(|server| server.state) {
            Some(mc_core::ServerState::Running) => "/connect 加入方式 · /open 日志 · /backups 备份",
            Some(mc_core::ServerState::Starting | mc_core::ServerState::Stopping) => {
                "/open 查看进展 · /logs 最近日志"
            }
            Some(mc_core::ServerState::Installed | mc_core::ServerState::Stopped) => {
                "/start 启动 · /config 设置 · /backups 备份"
            }
            Some(mc_core::ServerState::Failed) => "/open 检查失败原因 · /logs 最近日志",
            _ => "/refresh 刷新状态 · /open 查看详情",
        }
    };
    Line::from(Span::styled(format!(" 下一步  {actions}"), theme::text()))
}

fn composer_title(app: &AppState, width: u16) -> String {
    let title = if app.shell_input.trim_start().starts_with('/')
        && app
            .shell_input
            .split_whitespace()
            .next()
            .and_then(|name| name.strip_prefix('/'))
            .and_then(shell::find)
            .is_none_or(|command| command.name != "console")
    {
        " 管理命令 · / 查看操作 ".to_owned()
    } else {
        let target = app.shell_draft_target.or_else(|| {
            app.instances
                .get(app.selected_instance)
                .map(|instance| instance.id)
        });
        match target.and_then(|id| app.instances.iter().find(|instance| instance.id == id)) {
            Some(instance) => {
                let changed = app.shell_draft_target.is_some_and(|id| {
                    app.instances
                        .get(app.selected_instance)
                        .is_none_or(|current| current.id != id)
                });
                format!(
                    " 控制台 → {}{} ",
                    ui::truncate(&instance.name, width.saturating_sub(30) as usize),
                    if changed {
                        " · 目标已切换，禁止发送"
                    } else {
                        ""
                    }
                )
            }
            None => " 管理命令 · 输入 /create 建服 ".to_owned(),
        }
    };
    ui::truncate(&title, width.saturating_sub(2) as usize)
}

fn resource_line(app: &AppState, compact: bool) -> Line<'static> {
    let Some(instance) = app.instances.get(app.selected_instance) else {
        return Line::from(Span::styled(
            " 资源  —  ·  选中实例后显示进程占用",
            theme::dim(),
        ));
    };
    let server = selected_server(app);
    let stats = app
        .instance_detail
        .as_ref()
        .filter(|detail| detail.instance_id == instance.id)
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
            ("Ctrl-K", "操作"),
            ("F1", "帮助"),
        ])
    };
    frame.render_widget(Paragraph::new(line).style(theme::dim()), area);
}

#[cfg(test)]
mod tests;
