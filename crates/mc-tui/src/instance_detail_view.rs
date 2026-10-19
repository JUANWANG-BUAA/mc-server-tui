use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::{hit::HitMap, instance_detail::InstanceDetailState, theme, ui};

pub(crate) fn render_loading(frame: &mut Frame<'_>, instance_id: uuid::Uuid, _hits: &mut HitMap) {
    let area = frame.area();
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled("◆ MC SERVER TUI   实例详情", theme::brand())),
            Line::from(format!("正在读取实例 {instance_id}…")),
            Line::from("Esc 返回"),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme::border(true))
                .style(theme::surface()),
        ),
        area,
    );
}

pub(crate) fn render(
    frame: &mut Frame<'_>,
    detail: &InstanceDetailState,
    animation_frame: u8,
    _hits: &mut HitMap,
) {
    let compact = frame.area().width < 80 || frame.area().height < 18;
    let (header_height, summary_height, footer_height) =
        if compact { (2, 4, 3) } else { (3, 7, 4) };
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_height),
            Constraint::Length(summary_height),
            Constraint::Min(1),
            Constraint::Length(footer_height),
        ])
        .split(frame.area());

    let (name, status) = detail
        .detail
        .as_ref()
        .map(|data| (data.instance.name.as_str(), state_label(data.server.state)))
        .unwrap_or(("实例详情", "读取中"));
    let server_state = detail
        .detail
        .as_ref()
        .map(|data| data.server.state)
        .unwrap_or(mc_core::ServerState::Unknown);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" ◆ MC SERVER TUI ", theme::brand()),
            Span::styled("实例 · ", theme::dim()),
            Span::styled(name, theme::text().add_modifier(Modifier::BOLD)),
            Span::raw("  ·  "),
            Span::styled(
                format!("{} ", theme::server_marker(server_state, animation_frame)),
                state_style(status),
            ),
            Span::styled(status, state_style(status)),
        ]))
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(theme::border(false)),
        ),
        areas[0],
    );

    let summary = if let Some(data) = &detail.detail {
        let server = &data.server;
        let mut lines = if compact {
            vec![
                Line::from(vec![
                    Span::styled(
                        state_label(server.state),
                        state_style(state_label(server.state)),
                    ),
                    Span::styled(format!(" · {}", server.address), theme::accent()),
                ]),
                Line::from(Span::styled(
                    server.message.clone(),
                    state_style(state_label(server.state)),
                )),
            ]
        } else {
            vec![Line::from(vec![
                Span::styled("状态  ", label_style()),
                Span::styled(
                    state_label(server.state),
                    state_style(state_label(server.state)),
                ),
                Span::styled("   地址 ", label_style()),
                Span::styled(server.address.clone(), theme::accent()),
            ])]
        };
        if !compact {
            lines.push(Line::from(vec![
                Span::styled("版本  ", label_style()),
                Span::styled(data.instance.game_version.clone(), theme::accent()),
                Span::styled(
                    format!(" · build {} · Java ", data.instance.build_id),
                    theme::muted(),
                ),
                Span::styled(
                    data.instance.java_path.display().to_string(),
                    theme::accent(),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("资源  ", label_style()),
                Span::styled(
                    format!(
                        "{}–{} MiB",
                        data.instance.min_memory_mb, data.instance.max_memory_mb
                    ),
                    theme::accent(),
                ),
                Span::styled(" · 进程 ", label_style()),
                Span::styled(
                    server
                        .pid
                        .map(|pid| pid.to_string())
                        .unwrap_or_else(|| "—".to_owned()),
                    theme::accent(),
                ),
                Span::styled(" · 启动 ", label_style()),
                Span::styled(
                    server
                        .started_at
                        .map(|time| time.format("%H:%M:%S UTC").to_string())
                        .unwrap_or_else(|| "—".to_owned()),
                    theme::muted(),
                ),
            ]));
            if let Some(stats) = detail.stats {
                lines.push(Line::from(vec![
                    Span::styled("占用  ", label_style()),
                    Span::styled(format!("CPU {:.1}%", stats.cpu_percent), theme::accent()),
                    Span::styled("  ·  RSS ", label_style()),
                    Span::styled(ui::format_bytes(stats.rss_bytes), theme::accent()),
                ]));
            }
            lines.push(Line::from(Span::styled(
                server.message.clone(),
                state_style(state_label(server.state)),
            )));
            if let Some(code) = server.last_exit_code {
                lines.push(Line::from(format!("最后退出码  {code}")));
            }
        }
        lines
    } else {
        vec![Line::from("正在加载实例状态…")]
    };
    frame.render_widget(
        Paragraph::new(summary).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme::border(false))
                .title("服务状态")
                .title_style(theme::title(false))
                .style(theme::surface()),
        ),
        areas[1],
    );

    let visible_rows = areas[2].height.saturating_sub(2) as usize;
    let log_lines: Vec<&str> = detail.logs.lines().collect();
    let max_scroll = log_lines.len().saturating_sub(visible_rows.max(1));
    let scroll = detail.log_scroll.min(max_scroll);
    let end = log_lines.len().saturating_sub(scroll);
    let start = end.saturating_sub(visible_rows.max(1));
    let lines = if log_lines.is_empty() {
        vec![Line::from("等待服务端输出…")]
    } else {
        log_lines[start..end]
            .iter()
            .map(|line| Line::from(*line))
            .collect()
    };
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme::border(false))
                .title(if scroll == 0 {
                    format!("控制台日志 · byte {} · 跟随最新", detail.log_offset)
                } else {
                    format!(
                        "控制台日志 · byte {} · 已上滚 {} 行",
                        detail.log_offset, scroll
                    )
                })
                .title_style(theme::title(false))
                .style(theme::surface()),
        ),
        areas[2],
    );
    ui::scrollbar(
        frame,
        areas[2],
        log_lines.len(),
        log_lines.len().saturating_sub(end),
        visible_rows.max(1),
    );

    let status_line = if detail.editing_console {
        Line::from(Span::styled(
            format!("命令> {}▏", detail.console_input),
            theme::accent(),
        ))
    } else if let Some(error) = &detail.error {
        Line::from(Span::styled(error, theme::danger()))
    } else if let Some(notice) = &detail.notice {
        let style = if notice.contains("正在") {
            theme::warning()
        } else {
            theme::success()
        };
        Line::from(vec![
            Span::styled(format!("{} ", theme::spinner(animation_frame)), style),
            Span::styled(notice, style),
        ])
    } else {
        Line::from(Span::styled("服务端控制台与生命周期操作", theme::muted()))
    };
    frame.render_widget(
        Paragraph::new(vec![
            status_line,
            action_help_line(compact, detail.editing_console, server_state),
            navigation_help_line(detail.editing_console),
        ]),
        areas[3],
    );
}

fn label_style() -> Style {
    theme::muted().add_modifier(Modifier::BOLD)
}

fn action_help_line(
    compact: bool,
    editing_console: bool,
    state: mc_core::ServerState,
) -> Line<'static> {
    if editing_console {
        return Line::from(Span::styled("控制台命令输入", theme::muted()));
    }
    let can_start = matches!(
        state,
        mc_core::ServerState::Installed
            | mc_core::ServerState::Stopped
            | mc_core::ServerState::Failed
            | mc_core::ServerState::Unknown
    );
    if compact {
        Line::from(vec![
            if can_start {
                Span::styled("s", theme::key_hint())
            } else {
                Span::styled("x", theme::key_hint())
            },
            Span::raw(if can_start {
                " 启动 · "
            } else {
                " 停止 · "
            }),
            Span::styled(":", theme::key_hint()),
            Span::raw(" 控制台 · "),
            Span::styled("b", theme::key_hint()),
            Span::raw(" 备份 · "),
            Span::styled("l", theme::key_hint()),
            Span::raw(" 备份页 · "),
            Span::styled("/", theme::key_hint()),
            Span::raw(" 命令"),
        ])
    } else {
        Line::from(vec![
            if can_start {
                Span::styled("s", theme::key_hint())
            } else {
                Span::styled("x", theme::key_hint())
            },
            Span::raw(if can_start {
                " 启动 · "
            } else {
                " 优雅停止 · "
            }),
            Span::styled(":", theme::key_hint()),
            Span::raw(" 控制台 · "),
            Span::styled("b", theme::key_hint()),
            Span::raw(" 备份 · "),
            Span::styled("l", theme::key_hint()),
            Span::raw(" 世界与备份 · "),
            Span::styled("r", theme::key_hint()),
            Span::raw(" 刷新 · "),
            Span::styled("d", theme::key_hint()),
            Span::raw(" 主页 · "),
            Span::styled("?", theme::key_hint()),
            Span::raw(" 帮助"),
        ])
    }
}

fn navigation_help_line(editing_console: bool) -> Line<'static> {
    if editing_console {
        Line::from(vec![
            Span::styled("Enter", theme::key_hint()),
            Span::raw(" 发送 · "),
            Span::styled("Esc", theme::key_hint()),
            Span::raw(" 取消"),
        ])
    } else {
        Line::from(vec![
            Span::styled("↑/↓", theme::key_hint()),
            Span::raw(" 滚动日志 · "),
            Span::styled("PgUp/PgDn", theme::key_hint()),
            Span::raw(" 翻页 · "),
            Span::styled("Home/End", theme::key_hint()),
            Span::raw(" 顶部/最新 · "),
            Span::styled("Esc", theme::key_hint()),
            Span::raw(" 返回"),
        ])
    }
}

fn state_label(state: mc_core::ServerState) -> &'static str {
    match state {
        mc_core::ServerState::Installed => "已安装",
        mc_core::ServerState::Starting => "启动中",
        mc_core::ServerState::Running => "运行中",
        mc_core::ServerState::Stopping => "停止中",
        mc_core::ServerState::Stopped => "已停止",
        mc_core::ServerState::Failed => "启动失败",
        mc_core::ServerState::Unknown => "状态未知",
    }
}

fn state_style(state: &str) -> Style {
    match state {
        "运行中" => theme::success().add_modifier(Modifier::BOLD),
        "启动中" | "停止中" => theme::warning(),
        "启动失败" | "状态未知" => theme::danger(),
        _ => theme::muted(),
    }
}

#[cfg(test)]
mod tests;
