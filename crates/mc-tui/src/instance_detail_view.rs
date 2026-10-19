use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::{hit::HitMap, instance_detail::InstanceDetailState, theme, ui};

pub(crate) fn render_loading(
    frame: &mut Frame<'_>,
    area: Rect,
    instance_id: uuid::Uuid,
    _hits: &mut HitMap,
) {
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled("实例详情", theme::accent())),
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
    area: Rect,
    detail: &InstanceDetailState,
    animation_frame: u8,
    hits: &mut HitMap,
) {
    let compact = area.width < 80 || area.height < 17;
    let (header_height, summary_height, footer_height) =
        if compact { (2, 4, 3) } else { (2, 9, 4) };
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_height),
            Constraint::Length(summary_height),
            Constraint::Min(1),
            Constraint::Length(footer_height),
        ])
        .split(area);

    let (name, status, status_style) = detail
        .detail
        .as_ref()
        .map(|data| {
            (
                data.instance.name.as_str(),
                ui::server_state_label(data.server.state),
                ui::server_state_style(data.server.state),
            )
        })
        .unwrap_or(("实例详情", "读取中", theme::muted()));
    let server_state = detail
        .detail
        .as_ref()
        .map(|data| data.server.state)
        .unwrap_or(mc_core::ServerState::Unknown);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" 实例 · ", theme::dim()),
            Span::styled(name, theme::text().add_modifier(Modifier::BOLD)),
            Span::raw("  ·  "),
            Span::styled(
                format!("{} ", theme::server_marker(server_state, animation_frame)),
                status_style,
            ),
            Span::styled(status, status_style),
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
                        ui::server_state_label(server.state),
                        ui::server_state_style(server.state),
                    ),
                    Span::styled(format!(" · {}", server.address), theme::accent()),
                ]),
                Line::from(Span::styled(
                    ui::server_message(&server.message),
                    ui::server_state_style(server.state),
                )),
            ]
        } else {
            vec![Line::from(vec![
                Span::styled("状态  ", label_style()),
                Span::styled(
                    ui::server_state_label(server.state),
                    ui::server_state_style(server.state),
                ),
                Span::styled("   地址 ", label_style()),
                Span::styled(server.address.clone(), theme::accent()),
            ])]
        };
        if !compact {
            lines.push(Line::from(vec![
                Span::styled("版本  ", label_style()),
                Span::styled(
                    format!("{} {}", data.instance.provider, data.instance.game_version),
                    theme::accent(),
                ),
                Span::styled(
                    if data.instance.provider == "fabric" {
                        " · 稳定 Loader · Java ".to_owned()
                    } else {
                        format!(" · build {} · Java ", data.instance.build_id)
                    },
                    theme::muted(),
                ),
                Span::styled(ui::java_label(&data.instance.java_path), theme::accent()),
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
                        .map(|time| {
                            time.with_timezone(&chrono::Local)
                                .format("%H:%M:%S")
                                .to_string()
                        })
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
            if detail.stats.is_some() {
                let width = (area.width as usize / 3).clamp(8, 24);
                lines.push(Line::from(vec![
                    Span::styled("CPU ", theme::muted()),
                    Span::styled(
                        ui::history_plot(&detail.cpu_history, width, 100),
                        theme::accent(),
                    ),
                    Span::styled("  内存 ", theme::muted()),
                    Span::styled(
                        ui::history_plot(
                            &detail.rss_history,
                            width,
                            u64::from(data.instance.max_memory_mb),
                        ),
                        theme::success(),
                    ),
                    Span::styled(" · 最近 60 次采样", theme::muted()),
                ]));
            }
            if server.state == mc_core::ServerState::Starting {
                let mut spans = vec![Span::styled("启动 ", theme::warning())];
                spans.extend(ui::animated_progress(0.0, 16, animation_frame, true));
                spans.push(Span::styled(
                    " · 首次可能下载游戏文件；等待实际就绪日志",
                    theme::muted(),
                ));
                lines.push(Line::from(spans));
            }
            if let Some(alert) = detail.alerts().first() {
                lines.push(Line::from(Span::styled(
                    format!("! {}", alert),
                    theme::warning(),
                )));
            }
            lines.push(Line::from(Span::styled(
                "/connect 加入方式 · /monitor 在线人数 · /players 权限 · f 搜索日志",
                theme::muted(),
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
    let needle = detail.log_filter.to_lowercase();
    let log_lines: Vec<&str> = detail
        .logs
        .lines()
        .filter(|line| {
            needle.is_empty()
                || ui::clean_terminal_text(line)
                    .to_lowercase()
                    .contains(&needle)
        })
        .collect();
    let max_scroll = log_lines.len().saturating_sub(visible_rows.max(1));
    detail.log_max_scroll.set(max_scroll);
    let scroll = detail.log_scroll.min(max_scroll);
    let end = log_lines.len().saturating_sub(scroll);
    let start = end.saturating_sub(visible_rows.max(1));
    let lines = if log_lines.is_empty() {
        vec![Line::from("等待服务端输出…")]
    } else {
        log_lines[start..end]
            .iter()
            .map(|line| {
                ui::log_line(
                    &ui::clean_terminal_text(line)
                        .chars()
                        .skip(detail.log_horizontal)
                        .collect::<String>(),
                    &detail.log_filter,
                )
            })
            .collect()
    };
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme::border(false))
                .title(if scroll == 0 {
                    format!(
                        "控制台日志 · {} 行 · {} · 横移 {}",
                        log_lines.len(),
                        if detail.log_filter.is_empty() {
                            "全部"
                        } else {
                            &detail.log_filter
                        },
                        detail.log_horizontal
                    )
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
    ui::scrollbar(frame, areas[2], log_lines.len(), start, visible_rows.max(1));
    ui::scrollbar_hits(
        areas[2],
        log_lines.len(),
        visible_rows.max(1),
        hits,
        crate::hit::ScrollView::Logs,
    );

    let status_line = if detail.editing_filter {
        Line::from(Span::styled(
            format!("搜索> {}▏ · Enter 保留过滤 · Esc 清除", detail.log_filter),
            theme::accent(),
        ))
    } else if detail.editing_console {
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
            Span::raw(" 滚动 · f 搜索 · ←/→ 横移 · "),
            Span::styled("PgUp/PgDn", theme::key_hint()),
            Span::raw(" 翻页 · "),
            Span::styled("Home/End", theme::key_hint()),
            Span::raw(" 顶部/最新 · "),
            Span::styled("Esc", theme::key_hint()),
            Span::raw(" 返回"),
        ])
    }
}

#[cfg(test)]
mod tests;
