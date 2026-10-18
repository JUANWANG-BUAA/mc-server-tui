use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::{AppState, Panel};
use crate::{
    task_presentation::{activity_symbol, phase_label, status_label, status_style as task_style},
    theme,
};

pub(crate) fn render(frame: &mut Frame<'_>, app: &AppState, animation_frame: u8) {
    frame.render_widget(Block::default().style(theme::page()), frame.area());
    render_primary(frame, app, animation_frame);
    if let Some(palette) = &app.command_palette {
        crate::command_palette_view::render(frame, palette);
    }
}

fn render_primary(frame: &mut Frame<'_>, app: &AppState, animation_frame: u8) {
    if app.modpack_import_visible {
        if let Some(import) = &app.modpack_import {
            crate::modpack_import_view::render(frame, import, animation_frame);
        }
        return;
    }
    if let Some(wizard) = &app.wizard {
        crate::wizard_view::render(frame, wizard, animation_frame);
        return;
    }
    if app.task_detail_visible {
        if let Some(detail) = &app.task_detail {
            crate::task_detail_view::render(frame, detail, animation_frame);
            return;
        }
    }
    if app.backup_manager_visible {
        if let Some(manager) = &app.backup_manager {
            crate::backup_manager_view::render(frame, manager, animation_frame);
        }
        return;
    }
    if app.instance_detail_visible {
        if let Some(detail) = &app.instance_detail {
            crate::instance_detail_view::render(frame, detail, animation_frame);
        } else if let Some(instance_id) = app.instance_detail_id {
            crate::instance_detail_view::render_loading(frame, instance_id);
        }
        return;
    }
    render_dashboard(frame, app, animation_frame);
}

fn render_dashboard(frame: &mut Frame<'_>, app: &AppState, animation_frame: u8) {
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
        render_tasks(frame, columns[1], app, animation_frame);
    } else {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(root[1]);
        render_instances(frame, rows[0], app);
        render_tasks(frame, rows[1], app, animation_frame);
    }
    render_footer(frame, root[2], app);
}

fn render_header(frame: &mut Frame<'_>, area: Rect, app: &AppState) {
    let line = Line::from(vec![
        Span::styled(" ◆ MC SERVER TUI ", theme::brand()),
        Span::styled(format!("实例 {}", app.instances.len()), theme::accent()),
        Span::raw("  ·  "),
        Span::styled(format!("后台任务 {}", app.tasks.len()), theme::muted()),
    ]);
    frame.render_widget(
        Paragraph::new(line).block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(theme::border(false)),
        ),
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
        vec![
            ListItem::new(Line::from(Span::styled(
                "还没有服务器",
                theme::brand().add_modifier(ratatui::style::Modifier::BOLD),
            ))),
            ListItem::new(Line::from(Span::styled(
                "1  按 n 新建 Paper：选择稳定版本，自动检测兼容 Java",
                theme::accent(),
            ))),
            ListItem::new(Line::from(Span::styled(
                "2  按 / 搜“导入整合包”，先检查 MC、加载器和 Mod",
                theme::warning(),
            ))),
            ListItem::new(Line::from(Span::styled(
                "3  下载、备份和恢复都在后台任务中可查看、可恢复",
                theme::muted(),
            ))),
        ]
    } else {
        app.instances
            .iter()
            .map(|instance| {
                ListItem::new(Line::from(vec![
                    Span::styled(instance.name.clone(), theme::brand()),
                    Span::styled(
                        format!(
                            "   {} {} · :{}",
                            instance.provider, instance.game_version, instance.port
                        ),
                        theme::muted(),
                    ),
                ]))
            })
            .collect()
    };
    let mut state = ListState::default();
    if !app.instances.is_empty() && app.focused_panel == Panel::Instances {
        state.select(Some(app.selected_instance));
    }
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme::border(app.focused_panel == Panel::Instances))
                .title(title)
                .title_style(theme::title(app.focused_panel == Panel::Instances))
                .style(theme::surface()),
        )
        .highlight_symbol("▸ ")
        .highlight_style(theme::selected_row());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_tasks(frame: &mut Frame<'_>, area: Rect, app: &AppState, animation_frame: u8) {
    let title = if app.focused_panel == Panel::Tasks {
        "后台任务 · 当前面板"
    } else {
        "后台任务"
    };
    let items = if app.tasks.is_empty() {
        vec![ListItem::new(Span::styled("暂无后台任务", theme::muted()))]
    } else {
        app.tasks
            .iter()
            .map(|task| {
                let progress = task
                    .progress
                    .clone()
                    .map(|value| format!(" · {}/{}", value.completed_steps, value.total_steps))
                    .unwrap_or_default();
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{} ", activity_symbol(task.status, animation_frame)),
                        task_style(task.status),
                    ),
                    Span::styled(
                        format!(
                            "{} · {}",
                            status_label(task.status),
                            phase_label(task.phase)
                        ),
                        task_style(task.status),
                    ),
                    Span::styled(format!("{progress} · {}", task.message), theme::muted()),
                ]))
            })
            .collect()
    };
    let mut state = ListState::default();
    if !app.tasks.is_empty() && app.focused_panel == Panel::Tasks {
        state.select(Some(app.selected_task));
    }
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme::border(app.focused_panel == Panel::Tasks))
                .title(title)
                .title_style(theme::title(app.focused_panel == Panel::Tasks))
                .style(theme::surface()),
        )
        .highlight_symbol("▸ ")
        .highlight_style(theme::selected_row());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &AppState) {
    let lines = vec![
        Line::from(Span::styled(app.status.clone(), status_style(&app.status))),
        Line::from(vec![
            Span::styled("↑/↓", theme::key_hint()),
            Span::raw(" 选择 · "),
            Span::styled("Enter", theme::key_hint()),
            Span::raw(" 打开 · "),
            Span::styled("n", theme::key_hint()),
            Span::raw(" 新建 · "),
            Span::styled("/", theme::key_hint()),
            Span::raw(" 命令 · "),
            Span::styled("Ctrl-K", theme::key_hint()),
            Span::raw(" 全局命令 · "),
            Span::styled("q", theme::key_hint()),
            Span::raw(" 退出"),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines), area);
}

fn status_style(status: &str) -> Style {
    if status.contains("失败") {
        theme::danger()
    } else if status.contains("正在") || status.contains("刷新") || status.contains("提交") {
        theme::warning()
    } else {
        theme::success()
    }
}

#[cfg(test)]
mod tests;
