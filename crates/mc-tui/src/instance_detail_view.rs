use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::instance_detail::InstanceDetailState;

pub(crate) fn render_loading(frame: &mut Frame<'_>, instance_id: uuid::Uuid) {
    let area = frame.area();
    frame.render_widget(
        Paragraph::new(vec![
            Line::from("MC SERVER TUI   实例详情"),
            Line::from(format!("正在读取实例 {instance_id}…")),
            Line::from("Esc 返回"),
        ])
        .block(Block::default().borders(Borders::ALL)),
        area,
    );
}

pub(crate) fn render(frame: &mut Frame<'_>, detail: &InstanceDetailState) {
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
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                " MC SERVER TUI   ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(name),
            Span::raw(" · "),
            Span::styled(status, state_style(status)),
        ]))
        .block(Block::default().borders(Borders::BOTTOM)),
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
                    Span::raw(format!(" · {}", server.address)),
                ]),
                Line::from(server.message.clone()),
            ]
        } else {
            vec![Line::from(vec![
                Span::styled("状态  ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    state_label(server.state),
                    state_style(state_label(server.state)),
                ),
                Span::raw(format!("   地址 {}", server.address)),
            ])]
        };
        if !compact {
            lines.push(Line::from(format!(
                "版本  {} · build {} · Java {}",
                data.instance.game_version,
                data.instance.build_id,
                data.instance.java_path.display()
            )));
            lines.push(Line::from(format!(
                "资源  {}–{} MiB · 进程 {} · 启动 {}",
                data.instance.min_memory_mb,
                data.instance.max_memory_mb,
                server
                    .pid
                    .map(|pid| pid.to_string())
                    .unwrap_or_else(|| "—".to_owned()),
                server
                    .started_at
                    .map(|time| time.format("%H:%M:%S UTC").to_string())
                    .unwrap_or_else(|| "—".to_owned())
            )));
            lines.push(Line::from(server.message.clone()));
            if let Some(code) = server.last_exit_code {
                lines.push(Line::from(format!("最后退出码  {code}")));
            }
        }
        lines
    } else {
        vec![Line::from("正在加载实例状态…")]
    };
    frame.render_widget(
        Paragraph::new(summary).block(Block::default().borders(Borders::ALL).title("服务状态")),
        areas[1],
    );

    let visible_rows = areas[2].height.saturating_sub(2) as usize;
    let log_lines: Vec<&str> = detail.logs.lines().collect();
    let start = log_lines.len().saturating_sub(visible_rows.max(1));
    let lines = if log_lines.is_empty() {
        vec![Line::from("等待服务端输出…")]
    } else {
        log_lines[start..]
            .iter()
            .map(|line| Line::from(*line))
            .collect()
    };
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("控制台日志 · byte {}", detail.log_offset)),
        ),
        areas[2],
    );

    let status_line = if detail.editing_console {
        Line::from(format!("命令> {}▏", detail.console_input))
    } else if let Some(error) = &detail.error {
        Line::from(Span::styled(error, Style::default().fg(Color::Red)))
    } else if let Some(notice) = &detail.notice {
        Line::from(Span::styled(notice, Style::default().fg(Color::Yellow)))
    } else {
        Line::from(if compact {
            "s 启动 · x 停止 · b 备份 · l 列表"
        } else {
            "s 启动 · x 停止 · b 备份 · l 备份列表 · : 控制台命令 · r 刷新"
        })
    };
    let help = if detail.editing_console {
        "Enter 发送 · Esc 取消"
    } else {
        "↑/↓ 翻日志 · Enter/留在此页 · Esc 返回"
    };
    frame.render_widget(
        Paragraph::new(vec![status_line, Line::from(help)]),
        areas[3],
    );
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
        "运行中" => Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
        "启动中" | "停止中" => Style::default().fg(Color::Yellow),
        "启动失败" | "状态未知" => Style::default().fg(Color::Red),
        _ => Style::default().fg(Color::DarkGray),
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use mc_core::{InstanceDetail, InstanceSummary, ServerSnapshot, ServerState};
    use ratatui::{backend::TestBackend, Terminal};
    use uuid::Uuid;

    use crate::instance_detail::InstanceDetailState;

    use super::render;

    #[test]
    fn renders_instance_state_and_controls_at_reference_and_compact_sizes() {
        let instance_id = Uuid::new_v4();
        let mut state = InstanceDetailState::loading(instance_id);
        state.detail = Some(InstanceDetail {
            instance: InstanceSummary {
                id: instance_id,
                name: "生存服".to_owned(),
                provider: "Paper".to_owned(),
                game_version: "1.21.11".to_owned(),
                build_id: 17,
                java_path: "/opt/java/bin/java".into(),
                min_memory_mb: 1024,
                max_memory_mb: 4096,
                port: 25565,
                directory: "/tmp/server".into(),
                created_at: Utc::now(),
            },
            server: ServerSnapshot {
                instance_id,
                state: ServerState::Running,
                pid: Some(1234),
                port: 25565,
                address: "127.0.0.1:25565".to_owned(),
                started_at: Some(Utc::now()),
                last_exit_code: None,
                message: "服务端已就绪".to_owned(),
            },
        });
        state.logs = "Done (0.4s)!\n玩家加入\n".to_owned();

        let standard = render_to_text(&state, 80, 24);
        assert!(standard.contains("生存服"));
        assert!(standard.contains("运行中"));
        assert!(standard.contains("控制台日志"));
        assert!(standard.contains("启动"));
        assert!(standard.contains("备份"));

        let compact = render_to_text(&state, 40, 12);
        assert!(compact.contains("运行中"));
        assert!(compact.contains("已就绪"));
        assert!(compact.contains("备份"));
    }

    fn render_to_text(state: &InstanceDetailState, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|frame| render(frame, state))
            .expect("render instance detail");
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
