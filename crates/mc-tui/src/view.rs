use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::{AppState, Panel};
use crate::task_presentation::{
    activity_symbol, kind_short, phase_label, status_label, status_style as task_style,
};
use crate::{
    hit::{HitMap, HitTarget},
    theme, ui,
};

pub(crate) fn render(frame: &mut Frame<'_>, app: &AppState, animation_frame: u8) {
    frame.render_widget(Block::default().style(theme::page()), frame.area());
    let mut hits = HitMap::default();
    render_primary(frame, app, animation_frame, &mut hits);
    if app.help_visible {
        crate::help_view::render(frame, app, &mut hits);
    } else if let Some(palette) = &app.command_palette {
        crate::command_palette_view::render(frame, palette, &mut hits);
    }
    *app.hits.borrow_mut() = hits;
}

fn render_primary(frame: &mut Frame<'_>, app: &AppState, animation_frame: u8, hits: &mut HitMap) {
    if app.modpack_import_visible {
        if let Some(import) = &app.modpack_import {
            crate::modpack_import_view::render(frame, import, animation_frame, hits);
        }
        return;
    }
    if let Some(wizard) = &app.wizard {
        crate::wizard_view::render(frame, wizard, animation_frame, hits);
        return;
    }
    if app.task_detail_visible {
        if let Some(detail) = &app.task_detail {
            crate::task_detail_view::render(frame, detail, animation_frame, hits);
            return;
        }
    }
    if app.backup_manager_visible {
        if let Some(manager) = &app.backup_manager {
            crate::backup_manager_view::render(frame, manager, animation_frame, hits);
        }
        return;
    }
    if app.instance_detail_visible {
        if let Some(detail) = &app.instance_detail {
            crate::instance_detail_view::render(frame, detail, animation_frame, hits);
        } else if let Some(instance_id) = app.instance_detail_id {
            crate::instance_detail_view::render_loading(frame, instance_id, hits);
        }
        return;
    }
    if app.shell_visible {
        crate::shell_view::render(frame, app, animation_frame, hits);
        return;
    }
    render_dashboard(frame, app, animation_frame, hits);
}

fn render_dashboard(frame: &mut Frame<'_>, app: &AppState, animation_frame: u8, hits: &mut HitMap) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(frame.area());

    render_header(frame, root[0], app);
    if root[1].width >= 110 {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(52),
                Constraint::Length(1),
                Constraint::Percentage(47),
            ])
            .split(root[1]);
        render_instances(frame, columns[0], app, hits);
        render_tasks(frame, columns[2], app, animation_frame, hits);
    } else {
        let body_height = root[1].height;
        let max_instances = (body_height * 45 / 100).max(6);
        let instance_height = (app.instances.len() as u16 + 4).clamp(6, max_instances);
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(instance_height),
                Constraint::Length(1),
                Constraint::Min(1),
            ])
            .split(root[1]);
        render_instances(frame, rows[0], app, hits);
        render_tasks(frame, rows[2], app, animation_frame, hits);
    }
    render_footer(frame, root[2], app);
}

fn render_header(frame: &mut Frame<'_>, area: Rect, app: &AppState) {
    let running = app
        .tasks
        .iter()
        .filter(|task| {
            matches!(
                task.status,
                mc_core::TaskStatus::Queued | mc_core::TaskStatus::Running
            )
        })
        .count();
    let completed = app
        .tasks
        .iter()
        .filter(|task| task.status == mc_core::TaskStatus::Succeeded)
        .count();
    let line1 = Line::from(vec![
        Span::styled(" ◆ MC SERVER TUI ", theme::brand()),
        Span::styled(
            format!("v{}", env!("CARGO_PKG_VERSION")),
            theme::dim().add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ·  ", theme::dim()),
        Span::styled("本地守护进程", theme::muted()),
        Span::styled("  ·  ", theme::dim()),
        Span::styled(
            if app.status.contains("失败") {
                "连接异常"
            } else {
                "在线"
            },
            if app.status.contains("失败") {
                theme::danger()
            } else {
                theme::success()
            },
        ),
    ]);
    let line2 = Line::from(vec![
        Span::styled(" 实例 ", theme::muted()),
        Span::styled(
            app.instances.len().to_string(),
            theme::accent().add_modifier(Modifier::BOLD),
        ),
        Span::styled("   ·   任务 ", theme::muted()),
        Span::styled(
            app.tasks.len().to_string(),
            theme::accent().add_modifier(Modifier::BOLD),
        ),
        Span::styled("   ·   进行中 ", theme::muted()),
        Span::styled(
            running.to_string(),
            if running > 0 {
                theme::warning().add_modifier(Modifier::BOLD)
            } else {
                theme::muted()
            },
        ),
        Span::styled("   ·   已完成 ", theme::muted()),
        Span::styled(completed.to_string(), theme::success()),
        Span::styled("      自动刷新 2s", theme::dim()),
    ]);
    frame.render_widget(
        Paragraph::new(vec![line1, line2]).block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_type(theme::border_type())
                .border_style(theme::border(false)),
        ),
        area,
    );
}

fn render_instances(frame: &mut Frame<'_>, area: Rect, app: &AppState, hits: &mut HitMap) {
    let focused = app.focused_panel == Panel::Instances;
    let title = format!(
        " 实例 · {}{} ",
        app.instances.len(),
        if focused { "  ·  当前面板" } else { "" }
    );
    let block = theme::panel(title, focused).title_bottom(
        Line::from(vec![
            Span::styled(" n ", theme::key_hint()),
            Span::styled("新建  ", theme::muted()),
            Span::styled(" i ", theme::key_hint()),
            Span::styled("导入整合包 ", theme::muted()),
        ])
        .alignment(Alignment::Right),
    );
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.instances.is_empty() {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(vec![
                    Span::styled("▦  ", theme::accent()),
                    Span::styled("还没有服务器", theme::brand().add_modifier(Modifier::BOLD)),
                ]),
                Line::from(Span::styled(
                    "按 n 新建 Paper，版本和 Java 都会自动准备",
                    theme::muted(),
                )),
                Line::from(Span::styled("按 i 先检查 .mrpack 整合包", theme::muted())),
                Line::from(Span::styled("按 / 打开完整命令面板", theme::dim())),
            ]),
            inner,
        );
        return;
    }

    let items: Vec<ListItem<'_>> = app
        .instances
        .iter()
        .map(|instance| {
            let name = ui::truncate(&instance.name, 18);
            ListItem::new(Line::from(vec![
                Span::styled(name, theme::text().add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!(
                        "   {} {}",
                        instance.provider.to_ascii_lowercase(),
                        instance.game_version
                    ),
                    theme::muted(),
                ),
                Span::styled(format!("  ·  :{}", instance.port), theme::muted()),
                Span::styled(
                    format!(
                        "  ·  {}-{} MiB",
                        instance.min_memory_mb, instance.max_memory_mb
                    ),
                    theme::dim(),
                ),
            ]))
        })
        .collect();
    let mut state = ListState::default();
    if focused {
        state.select(Some(app.selected_instance));
    }
    let list = List::new(items)
        .block(Block::default())
        .highlight_symbol("▌ ")
        .highlight_style(theme::selected_row())
        .scroll_padding(1);
    frame.render_stateful_widget(list, inner, &mut state);

    let offset = state.offset();
    let visible = inner.height as usize;
    for (row, index) in (offset..app.instances.len()).take(visible).enumerate() {
        hits.push(
            Rect::new(inner.x, inner.y + row as u16, inner.width, 1),
            HitTarget::Instance(index),
        );
    }
    ui::scrollbar(
        frame,
        area,
        app.instances.len(),
        app.selected_instance,
        visible,
    );
}

fn render_tasks(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &AppState,
    animation_frame: u8,
    hits: &mut HitMap,
) {
    let focused = app.focused_panel == Panel::Tasks;
    let title = format!(
        " 后台任务 · {}{} ",
        app.tasks.len(),
        if focused { "  ·  当前面板" } else { "" }
    );
    let block = theme::panel(title, focused).title_bottom(
        Line::from(vec![
            Span::styled(" Enter ", theme::key_hint()),
            Span::styled("详情  ", theme::muted()),
            Span::styled(" ↑↓ ", theme::key_hint()),
            Span::styled("选择 ", theme::muted()),
        ])
        .alignment(Alignment::Right),
    );
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.tasks.is_empty() {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(vec![
                    Span::styled("◌  ", theme::muted()),
                    Span::styled("暂无后台任务", theme::muted().add_modifier(Modifier::BOLD)),
                ]),
                Line::from(Span::styled(
                    "建服、备份和恢复都会在这里显示字节进度",
                    theme::dim(),
                )),
                Line::from(Span::styled(
                    "按 n 新建服务器，按 i 导入整合包",
                    theme::dim(),
                )),
            ]),
            inner,
        );
        return;
    }

    let bar_width = inner.width.saturating_sub(30).clamp(8, 24) as usize;
    let items: Vec<ListItem<'_>> = app
        .tasks
        .iter()
        .map(|task| {
            let percent = ui::task_percent(task);
            let ratio = ui::task_ratio(task);
            let style = task_style(task.status);
            let line1 = Line::from(vec![
                Span::styled(
                    format!("{} ", activity_symbol(task.status, animation_frame)),
                    style,
                ),
                Span::styled(
                    format!("{} · ", kind_short(task.kind)),
                    theme::text().add_modifier(Modifier::BOLD),
                ),
                Span::styled(phase_label(task.phase), theme::muted()),
                Span::styled(format!("   {} ", ui::task_step_label(task)), theme::dim()),
                Span::styled(status_label(task.status), style),
            ]);
            let mut spans = ui::progress_bar(ratio, bar_width, theme::progress());
            spans.push(Span::styled(
                format!("  {percent:>3}%  "),
                if percent >= 100 {
                    theme::success()
                } else {
                    theme::accent()
                },
            ));
            let detail = ui::task_detail_label(task);
            let available = inner.width.saturating_sub(bar_width as u16 + 10) as usize;
            spans.push(Span::styled(
                ui::truncate(&detail, available),
                theme::muted(),
            ));
            ListItem::new(vec![line1, Line::from(spans)])
        })
        .collect();
    let mut state = ListState::default();
    if focused {
        state.select(Some(app.selected_task));
    }
    let list = List::new(items)
        .block(Block::default())
        .highlight_symbol("▌ ")
        .highlight_style(theme::selected_row())
        .scroll_padding(1);
    frame.render_stateful_widget(list, inner, &mut state);

    let offset = state.offset();
    let item_height = 2u16;
    let visible_items = (inner.height / item_height) as usize;
    for (row, index) in (offset..app.tasks.len()).take(visible_items).enumerate() {
        let y = inner.y + (row as u16) * item_height;
        hits.push(
            Rect::new(inner.x, y, inner.width, item_height.min(inner.bottom() - y)),
            HitTarget::Task(index),
        );
    }
    ui::scrollbar(
        frame,
        area,
        app.tasks.len(),
        app.selected_task,
        visible_items.max(1),
    );
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &AppState) {
    let (symbol, style) = if app.status.contains("失败") {
        ("×", theme::danger())
    } else if app.status.contains("正在") || app.status.contains("刷新") {
        ("◐", theme::warning())
    } else {
        ("●", theme::success())
    };
    let status = Line::from(vec![
        Span::styled(format!(" {symbol} "), style),
        Span::styled(app.status.clone(), style),
    ]);
    frame.render_widget(
        Paragraph::new(vec![
            status,
            ui::plain_hint_line(&[
                ("↑/↓", "选择"),
                ("Enter", "打开"),
                ("Tab", "面板"),
                ("n", "新建"),
                ("/", "命令"),
                ("?", "帮助"),
            ]),
            ui::plain_hint_line(&[
                ("i", "导入"),
                ("t", "会话终端"),
                ("PgUp/PgDn", "翻页"),
                ("r", "刷新"),
                ("q", "退出"),
            ])
            .style(theme::dim()),
        ]),
        area,
    );
}

#[cfg(test)]
mod tests;
