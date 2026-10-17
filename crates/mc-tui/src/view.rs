use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::{AppState, Panel};

pub(crate) fn render(frame: &mut Frame<'_>, app: &AppState) {
    if let Some(wizard) = &app.wizard {
        crate::wizard_view::render(frame, wizard);
        return;
    }
    render_dashboard(frame, app);
}

fn render_dashboard(frame: &mut Frame<'_>, app: &AppState) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(frame.area());

    render_header(frame, root[0], app);
    if root[1].width >= 110 {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
            .split(root[1]);
        render_instances(frame, columns[0], app);
        render_tasks(frame, columns[1], app);
    } else {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(root[1]);
        render_instances(frame, rows[0], app);
        render_tasks(frame, rows[1], app);
    }
    render_footer(frame, root[2], app);
}

fn render_header(frame: &mut Frame<'_>, area: Rect, app: &AppState) {
    let text = format!(
        " MC SERVER TUI   实例 {} · 任务 {}",
        app.instances.len(),
        app.tasks.len()
    );
    frame.render_widget(
        Paragraph::new(text).block(Block::default().borders(Borders::BOTTOM)),
        area,
    );
}

fn render_instances(frame: &mut Frame<'_>, area: Rect, app: &AppState) {
    let title = if app.focused_panel == Panel::Instances {
        "实例 · 当前面板"
    } else {
        "实例"
    };
    let items = if app.instances.is_empty() {
        vec![ListItem::new("暂无实例 · 按 r 刷新")]
    } else {
        app.instances
            .iter()
            .map(|instance| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        instance.name.clone(),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(format!(
                        "   {} {} · :{}",
                        instance.provider, instance.game_version, instance.port
                    )),
                ]))
            })
            .collect()
    };
    let mut state = ListState::default();
    if !app.instances.is_empty() {
        state.select(Some(app.selected_instance));
    }
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_symbol("› ")
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_tasks(frame: &mut Frame<'_>, area: Rect, app: &AppState) {
    let title = if app.focused_panel == Panel::Tasks {
        "后台任务 · 当前面板"
    } else {
        "后台任务"
    };
    let items = if app.tasks.is_empty() {
        vec![ListItem::new("暂无后台任务")]
    } else {
        app.tasks
            .iter()
            .map(|task| {
                let progress = task
                    .progress
                    .map(|value| format!(" · {}/{}", value.completed_steps, value.total_steps))
                    .unwrap_or_default();
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!(
                            "{} · {}",
                            status_label(task.status),
                            phase_label(task.phase)
                        ),
                        task_style(task.status),
                    ),
                    Span::raw(format!("{progress} · {}", task.message)),
                ]))
            })
            .collect()
    };
    let mut state = ListState::default();
    if !app.tasks.is_empty() {
        state.select(Some(app.selected_task));
    }
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_symbol("› ")
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &AppState) {
    let lines = vec![
        Line::from(Span::styled(app.status.clone(), status_style(&app.status))),
        Line::from("↑/↓ 选择 · Tab 切换面板 · n 新建 · r 刷新 · q 退出"),
    ];
    frame.render_widget(Paragraph::new(lines), area);
}

fn status_label(status: mc_core::TaskStatus) -> &'static str {
    match status {
        mc_core::TaskStatus::Queued => "排队中",
        mc_core::TaskStatus::Running => "进行中",
        mc_core::TaskStatus::Succeeded => "已完成",
        mc_core::TaskStatus::Failed => "失败",
        mc_core::TaskStatus::Interrupted => "已中断",
    }
}

fn phase_label(phase: mc_core::TaskPhase) -> &'static str {
    match phase {
        mc_core::TaskPhase::Queued => "等待执行",
        mc_core::TaskPhase::ResolvingArtifact => "解析版本",
        mc_core::TaskPhase::DownloadingArtifact => "下载校验",
        mc_core::TaskPhase::InstallingInstance => "安装实例",
        mc_core::TaskPhase::Completed => "完成",
        mc_core::TaskPhase::Failed => "失败",
        mc_core::TaskPhase::Interrupted => "中断",
    }
}

fn task_style(status: mc_core::TaskStatus) -> Style {
    match status {
        mc_core::TaskStatus::Queued | mc_core::TaskStatus::Running => {
            Style::default().fg(Color::Yellow)
        }
        mc_core::TaskStatus::Succeeded => Style::default().fg(Color::Green),
        mc_core::TaskStatus::Failed | mc_core::TaskStatus::Interrupted => {
            Style::default().fg(Color::Red)
        }
    }
}

fn status_style(status: &str) -> Style {
    if status.starts_with("连接失败") {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use mc_core::{InstanceSummary, TaskKind, TaskPhase, TaskProgress, TaskSnapshot, TaskStatus};
    use ratatui::{backend::TestBackend, Terminal};
    use uuid::Uuid;

    use crate::app::{AppState, DashboardData};

    use super::render;

    #[test]
    fn renders_empty_dashboard_at_reference_width() {
        let state = AppState::default();
        let text = render_to_text(80, 24, &state);

        assert!(text.contains("MCSERVERTUI"));
        assert!(text.contains("暂无实例"));
        assert!(text.contains("后台任务"));
        assert!(text.contains("q退出"));
    }

    #[test]
    fn renders_instance_and_task_state() {
        let mut state = AppState::default();
        state.apply_refresh(Ok(DashboardData {
            instances: vec![InstanceSummary {
                id: Uuid::new_v4(),
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
            }],
            tasks: vec![TaskSnapshot {
                id: Uuid::new_v4(),
                kind: TaskKind::CreatePaperInstance,
                instance_id: None,
                status: TaskStatus::Running,
                phase: TaskPhase::DownloadingArtifact,
                message: "正在下载并校验 Paper".to_owned(),
                progress: Some(TaskProgress {
                    completed_steps: 1,
                    total_steps: 3,
                }),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                finished_at: None,
                result: None,
                error: None,
                last_event_sequence: 2,
            }],
        }));
        let text = render_to_text(100, 30, &state);

        assert!(text.contains("生存服"));
        assert!(text.contains("Paper1.21.11"));
        assert!(text.contains("下载校验"));
        assert!(text.contains("1/3"));
    }

    #[test]
    fn compact_terminal_render_does_not_panic() {
        let text = render_to_text(40, 12, &AppState::default());
        assert!(text.contains("MCSERVERTUI"));
    }

    fn render_to_text(width: u16, height: u16, state: &AppState) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|frame| render(frame, state))
            .expect("render dashboard");
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
