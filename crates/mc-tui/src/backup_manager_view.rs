use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::{
    backup_manager::{format_bytes, BackupManagerState},
    hit::{HitMap, HitTarget},
    theme, ui,
};

pub(crate) fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &BackupManagerState,
    animation_frame: u8,
    hits: &mut HitMap,
) {
    let compact = area.width < 80 || area.height < 17;
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(if compact { 2 } else { 3 }),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area);

    render_header(frame, areas[0], state, compact);
    if state.is_restoring() {
        render_restoring(frame, areas[1], state, animation_frame, compact);
    } else if let Some(plan) = &state.plan {
        render_plan(frame, areas[1], state, plan, compact);
    } else if let Some(backup_id) = state.restored_backup_id {
        render_restored(frame, areas[1], backup_id, compact);
    } else if state.backups.is_empty() && state.operation().is_some() {
        render_loading(frame, areas[1], state, animation_frame);
    } else {
        render_browser(frame, areas[1], state, animation_frame, hits);
    }
    render_footer(frame, areas[2], state, animation_frame);
}

fn render_header(frame: &mut Frame<'_>, area: Rect, state: &BackupManagerState, compact: bool) {
    let line = Line::from(vec![
        Span::styled(" 世界与备份", theme::accent()),
        Span::raw("  ·  "),
        Span::styled(state.instance_name.clone(), theme::muted()),
    ]);
    let lines = if compact {
        vec![line]
    } else {
        vec![
            line,
            Line::from(vec![
                Span::styled(" 停机一致性", theme::success()),
                Span::raw("  ·  "),
                Span::styled("分页目录", theme::accent()),
                Span::raw("  ·  "),
                Span::styled("恢复前强制复验", theme::warning()),
            ]),
        ]
    };
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(theme::border(false)),
        ),
        area,
    );
}

fn render_browser(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &BackupManagerState,
    animation_frame: u8,
    hits: &mut HitMap,
) {
    let compact = area.width < 80 || area.height < 17;
    if compact {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(2)])
            .split(area);
        render_backup_list(frame, rows[0], state, hits);
        render_selected_backup(frame, rows[1], state, animation_frame, true);
    } else {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .split(area);
        let title = format!(
            "已载入 {} 个通过完整性校验的备份{}",
            state.backups.len(),
            if state.has_more() {
                " · 按 m 加载更多"
            } else {
                ""
            }
        );
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("备份目录  ", theme::muted()),
                Span::styled(title, theme::accent()),
            ]))
            .block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(theme::border(false)),
            ),
            rows[0],
        );
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(48), Constraint::Percentage(52)])
            .split(rows[1]);
        render_backup_list(frame, columns[0], state, hits);
        render_selected_backup(frame, columns[1], state, animation_frame, false);
    }
}

fn render_backup_list(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &BackupManagerState,
    hits: &mut HitMap,
) {
    let items = if state.backups.is_empty() {
        vec![ListItem::new(Span::styled(
            "没有可恢复的备份",
            theme::muted(),
        ))]
    } else {
        state
            .backups
            .iter()
            .map(|backup| {
                let verified = state.verified_backup_id == Some(backup.id);
                ListItem::new(Line::from(vec![
                    Span::styled(
                        if verified { " ✓ " } else { " · " },
                        if verified {
                            theme::success()
                        } else {
                            theme::muted()
                        },
                    ),
                    Span::styled(
                        backup.created_at.format("%m-%d %H:%M").to_string(),
                        theme::accent(),
                    ),
                    Span::styled(
                        format!(
                            "  {}件 · {}",
                            backup.file_count,
                            format_bytes(backup.total_bytes)
                        ),
                        theme::muted(),
                    ),
                ]))
            })
            .collect()
    };
    let mut list_state = ListState::default();
    if !state.backups.is_empty() {
        list_state.select(Some(state.selected));
    }
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme::border(true))
                .title(format!("备份快照 · {}", state.backups.len()))
                .title_style(theme::title(true))
                .style(theme::surface()),
        )
        .highlight_symbol("▸ ")
        .highlight_style(theme::selected_row());
    frame.render_stateful_widget(list, area, &mut list_state);
    let offset = list_state.offset();
    let inner = Block::default().borders(Borders::ALL).inner(area);
    for (row, index) in (offset..state.backups.len())
        .take(inner.height as usize)
        .enumerate()
    {
        hits.push(
            Rect::new(inner.x, inner.y + row as u16, inner.width, 1),
            HitTarget::Backup(index),
        );
    }
    ui::scrollbar(
        frame,
        area,
        state.backups.len(),
        ui::window_start(state.selected, state.backups.len(), inner.height as usize),
        inner.height as usize,
    );
    ui::scrollbar_hits(
        area,
        state.backups.len(),
        inner.height as usize,
        hits,
        crate::hit::ScrollView::Backups,
    );
}

fn render_selected_backup(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &BackupManagerState,
    animation_frame: u8,
    compact: bool,
) {
    let Some(backup) = state.selected_backup() else {
        frame.render_widget(
            Paragraph::new("暂无备份摘要")
                .block(panel("所选备份", false))
                .style(theme::surface()),
            area,
        );
        return;
    };
    let verified = state.verified_backup_id == Some(backup.id);
    let verification = if verified {
        Line::from(Span::styled("✓ 已单独复验完整性", theme::success()))
    } else if state.operation().is_some() {
        Line::from(vec![
            Span::styled(
                format!("{} ", theme::spinner(animation_frame)),
                theme::warning(),
            ),
            Span::styled("正在等待后台任务结果", theme::warning()),
        ])
    } else {
        Line::from(Span::styled("按 v 重新哈希并验证所选备份", theme::muted()))
    };
    let mut lines = vec![
        Line::from(Span::styled(backup.id.to_string(), theme::accent())),
        Line::from(vec![Span::styled(
            backup
                .created_at
                .format("%Y-%m-%d %H:%M:%S UTC")
                .to_string(),
            theme::muted(),
        )]),
        Line::from(vec![
            Span::styled("一致性  ", label_style()),
            Span::styled("服务已停止 · 复制后已复验", theme::success()),
        ]),
        Line::from(vec![
            Span::styled("内容    ", label_style()),
            Span::styled(
                format!(
                    "{} 个文件 · {}",
                    backup.file_count,
                    format_bytes(backup.total_bytes)
                ),
                theme::accent(),
            ),
        ]),
        verification,
    ];
    if !compact {
        lines.push(Line::from(vec![
            Span::styled("恢复    ", label_style()),
            Span::styled("Enter 预览影响", theme::warning()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("确认    ", label_style()),
            Span::styled("c 输入 RESTORE", theme::warning()),
        ]));
        lines.push(Line::from(Span::styled(
            "停机恢复；替换前保留当前世界回滚目录。",
            theme::muted(),
        )));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(panel("所选备份", !compact))
            .style(theme::surface()),
        area,
    );
}

fn render_loading(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &BackupManagerState,
    animation_frame: u8,
) {
    let task = state
        .tracked_task_id()
        .map(|id| format!("任务 {}", &id.to_string()[..8]))
        .unwrap_or_else(|| "正在提交后台任务".to_owned());
    let lines = vec![
        Line::from(vec![
            Span::styled(
                format!("{} ", theme::spinner(animation_frame)),
                theme::warning(),
            ),
            Span::styled(
                state
                    .operation()
                    .map(operation_label)
                    .unwrap_or("正在读取备份"),
                theme::warning().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(Span::styled(
            "目录页和文件哈希均由 daemon 在实例锁内校验。",
            theme::muted(),
        )),
        Line::from(Span::styled(task, theme::muted())),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .block(panel("读取备份目录", true))
            .style(theme::surface()),
        area,
    );
}

fn render_plan(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &BackupManagerState,
    plan: &mc_core::BackupRestorePlan,
    compact: bool,
) {
    // Impact and the confirmation prompt come first; technical evidence is
    // last so a short terminal clips details, never the action line.
    let mut lines = vec![
        Line::from(Span::styled(
            "✓ 目标备份已再次校验；预览尚未修改任何世界文件。",
            theme::success().add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("恢复到  ", label_style()),
            Span::styled(
                plan.created_at
                    .with_timezone(&chrono::Local)
                    .format("%Y-%m-%d %H:%M")
                    .to_string(),
                theme::accent(),
            ),
            Span::styled(
                format!(" 的备份 · {}", &plan.backup_id.to_string()[..8]),
                theme::muted(),
            ),
        ]),
        impact_line("替换", &plan.replaced_worlds, theme::warning()),
        impact_line("新增", &plan.added_worlds, theme::accent()),
        impact_line("移除", &plan.removed_worlds, theme::danger()),
    ];
    if !compact {
        lines.push(Line::from(vec![
            Span::styled("空间  ", label_style()),
            Span::styled(
                format!(
                    "需要 {} · 可用 {}",
                    format_bytes(plan.required_space_bytes),
                    format_bytes(plan.available_space_bytes)
                ),
                if plan.required_space_bytes <= plan.available_space_bytes {
                    theme::success()
                } else {
                    theme::danger()
                },
            ),
        ]));
    }
    lines.push(Line::from(vec![
        Span::styled("下一步  ", label_style()),
        if state.editing_confirmation {
            Span::styled("在底部输入 RESTORE 后按 Enter", theme::warning())
        } else {
            Span::styled("按 c 输入 RESTORE", theme::warning())
        },
        Span::styled("；Esc 取消。", theme::muted()),
    ]));
    if !compact {
        lines.push(Line::from(Span::styled(
            "恢复完成后服务器保持停止，由你复查并显式启动；失败自动回滚。",
            theme::warning(),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("技术细节", label_style())));
        lines.push(Line::from(Span::styled(
            "1. 停止并锁定实例，复验目标备份与当前世界现场树。",
            theme::muted(),
        )));
        lines.push(Line::from(Span::styled(
            "2. 同文件系统暂存目标世界，再按 journal 顺序替换目录。",
            theme::muted(),
        )));
        lines.push(Line::from(Span::styled(
            "3. 扫描新树并写入 committed 记录；当前世界保留为回滚目录。",
            theme::muted(),
        )));
        lines.push(Line::from(vec![
            Span::styled("令牌  ", label_style()),
            Span::styled(short_token(&plan.confirmation_token), theme::muted()),
        ]));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(panel("恢复影响预览 · 停机操作", true))
            .style(theme::surface()),
        area,
    );
}

fn render_restoring(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &BackupManagerState,
    animation_frame: u8,
    compact: bool,
) {
    let plan = state.plan.as_ref();
    let worlds = plan
        .map(|plan| format_worlds(&plan.target_worlds))
        .unwrap_or_else(|| "目标世界".to_owned());
    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                format!("{} ", theme::spinner(animation_frame)),
                theme::warning(),
            ),
            Span::styled(
                "正在执行恢复事务",
                theme::warning().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("目标  ", label_style()),
            Span::styled(worlds, theme::accent()),
        ]),
        Line::from(Span::styled(
            "当前世界、暂存副本和回滚目录按 journal 顺序替换。",
            theme::muted(),
        )),
        Line::from(Span::styled(
            "请勿手动修改实例目录；持久任务会在后台继续执行。",
            theme::danger(),
        )),
    ];
    if !compact {
        lines.push(Line::from(Span::styled(
            "完成后服务器仍保持停止，返回实例详情即可复查并启动。",
            theme::muted(),
        )));
        if let Some(task_id) = state.tracked_task_id() {
            lines.push(Line::from(Span::styled(
                format!("任务  {task_id}"),
                theme::muted(),
            )));
        }
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(panel("恢复事务 · 执行中", true))
            .style(theme::surface()),
        area,
    );
}

fn render_restored(frame: &mut Frame<'_>, area: Rect, backup_id: uuid::Uuid, compact: bool) {
    let mut lines = vec![
        Line::from(Span::styled(
            "✓ 世界恢复完成并通过目录树校验",
            theme::success().add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("来源  ", label_style()),
            Span::styled(backup_id.to_string(), theme::accent()),
        ]),
        Line::from(Span::styled(
            "服务器保持停止；Esc 返回实例详情后可复查日志并启动。",
            theme::warning(),
        )),
    ];
    if !compact {
        lines.push(Line::from(Span::styled(
            "如启动异常，请保留当前日志并检查 daemon 的恢复事务证据，不要直接覆盖世界目录。",
            theme::muted(),
        )));
        lines.push(Line::from(Span::styled(
            "r 重新读取备份目录 · Esc 返回实例详情",
            theme::muted(),
        )));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(panel("恢复结果", true))
            .style(theme::surface()),
        area,
    );
}

fn render_footer(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &BackupManagerState,
    animation_frame: u8,
) {
    let busy = state.operation().is_some();
    let (symbol, message, style) = if let Some(error) = &state.error {
        ("×".to_owned(), error.clone(), theme::danger())
    } else if let Some(operation) = state.operation() {
        (
            theme::spinner(animation_frame).to_owned(),
            operation_label(operation).to_owned(),
            theme::warning(),
        )
    } else if let Some(backup_id) = state.restored_backup_id {
        (
            "✓".to_owned(),
            format!("已恢复 {}", &backup_id.to_string()[..8]),
            theme::success(),
        )
    } else {
        (
            "✓".to_owned(),
            state
                .notice
                .clone()
                .unwrap_or_else(|| "备份列表已通过完整性校验".to_owned()),
            theme::success(),
        )
    };
    let status = Line::from(vec![
        Span::styled(format!("{symbol} "), style),
        Span::styled(message, style),
    ]);
    let (primary, secondary) = if state.editing_confirmation {
        // The live input stays in the fixed footer so it is visible at every
        // terminal size, like the session composer.
        (
            Line::from(vec![
                Span::styled("确认> ", theme::warning().add_modifier(Modifier::BOLD)),
                Span::styled(format!("{}▏", state.confirmation_input), theme::accent()),
                Span::styled("  需完整输入 RESTORE", theme::muted()),
            ]),
            help_line(
                "Enter",
                "执行恢复",
                "Esc",
                "取消输入",
                "RESTORE",
                "完整短语",
            ),
        )
    } else if state.is_reviewing_plan() {
        (
            help_line("c", "输入恢复确认", "Esc", "取消预览", "q", "退出"),
            help_line("RESTORE", "完整短语", "c", "重新输入", "Esc", "不执行恢复"),
        )
    } else if state.is_restoring() {
        (
            Line::from(Span::styled(
                "恢复事务运行中；本页自动读取持久任务结果。",
                theme::warning(),
            )),
            help_line(
                "q",
                "退出 TUI",
                "Esc",
                "事务完成前不可返回",
                "Ctrl-C",
                "强制退出",
            ),
        )
    } else if busy {
        (
            Line::from(Span::styled(
                "后台任务运行中；本页自动刷新，不会重复提交。",
                theme::warning(),
            )),
            help_line("Esc", "任务完成前保持本页", "q", "退出", "r", "不重复提交"),
        )
    } else if state.restored_backup_id.is_some() {
        (
            help_line("Esc", "返回实例详情", "q", "退出", "r", "刷新目录"),
            help_line("v", "重新校验", "Enter", "再次预览", "Ctrl-C", "退出"),
        )
    } else if state.error.is_some() {
        (
            help_line("r", "重试刷新", "v", "校验所选", "Esc", "返回"),
            help_line("Enter", "恢复预览", "q", "退出", "", ""),
        )
    } else if area.width < 80 {
        (
            help_line("v", "校验", "Enter", "恢复", "Esc", "返回"),
            if state.has_more() {
                help_line("m", "更多", "r", "刷新", "q", "退出")
            } else {
                help_line("b", "备份", "r", "刷新", "q", "退出")
            },
        )
    } else {
        (
            help_line("v", "校验所选", "Enter", "恢复预览", "d", "主页"),
            if state.has_more() {
                help_line("m", "更多", "b", "新建备份", "Esc", "返回")
            } else {
                help_line("j/k", "选择", "b", "新建备份", "Esc", "返回")
            },
        )
    };
    frame.render_widget(Paragraph::new(vec![status, primary, secondary]), area);
}
fn panel(title: &'static str, focused: bool) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(theme::border_type())
        .border_style(theme::border(focused))
        .title(title)
        .title_style(theme::title(focused))
}

fn label_style() -> Style {
    theme::muted().add_modifier(Modifier::BOLD)
}

fn help_line(
    k1: &'static str,
    v1: &'static str,
    k2: &'static str,
    v2: &'static str,
    k3: &'static str,
    v3: &'static str,
) -> Line<'static> {
    Line::from(vec![
        Span::styled(k1, theme::key_hint()),
        Span::raw(format!(" {v1} · ")),
        Span::styled(k2, theme::key_hint()),
        Span::raw(format!(" {v2} · ")),
        Span::styled(k3, theme::key_hint()),
        Span::raw(format!(" {v3}")),
    ])
}
fn impact_line(label: &'static str, worlds: &[String], style: Style) -> Line<'static> {
    let value = if worlds.is_empty() {
        "无".to_owned()
    } else {
        format_worlds(worlds)
    };
    Line::from(vec![
        Span::styled(format!("{label}  "), label_style()),
        Span::styled(value, style),
    ])
}

fn format_worlds(worlds: &[String]) -> String {
    if worlds.is_empty() {
        "无".to_owned()
    } else {
        worlds.join("、")
    }
}
fn short_token(token: &str) -> String {
    if token.len() > 18 {
        format!("{}…{}", &token[..10], &token[token.len() - 6..])
    } else {
        token.to_owned()
    }
}

fn operation_label(operation: crate::backup_manager::BackupOperation) -> &'static str {
    match operation {
        crate::backup_manager::BackupOperation::LoadingPage => "正在读取并校验备份页…",
        crate::backup_manager::BackupOperation::LoadingMore => "正在加载更多备份…",
        crate::backup_manager::BackupOperation::Creating => "正在创建停机世界备份…",
        crate::backup_manager::BackupOperation::Verifying => "正在重新哈希所选备份…",
        crate::backup_manager::BackupOperation::Planning => "正在生成恢复影响预览…",
        crate::backup_manager::BackupOperation::Restoring => "正在执行世界恢复事务…",
    }
}

#[cfg(test)]
mod tests;
