use std::fs;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use mc_core::{ModpackAnalysisReport, ModpackSeverity, ModpackStatus};

use crate::{
    hit::{HitMap, HitTarget},
    modpack_import::ModpackImportState,
    theme, ui,
};

pub(crate) fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &ModpackImportState,
    animation_frame: u8,
    hits: &mut HitMap,
) {
    let compact = area.width < 80 || area.height < 19;
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(if compact { 2 } else { 3 }),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area);
    let header = if compact {
        vec![Line::from(Span::styled(" 整合包导入", theme::accent()))]
    } else {
        vec![
            Line::from(vec![
                Span::styled(" 整合包导入", theme::accent()),
                Span::styled("  ·  静态兼容检查", theme::muted()),
            ]),
            Line::from(Span::styled(
                "选择 ZIP 预览导入当前实例；MRPACK 可检查版本、加载器和服务端文件。",
                theme::muted(),
            )),
        ]
    };
    frame.render_widget(
        Paragraph::new(header).block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(theme::border(false)),
        ),
        areas[0],
    );

    if state.loading {
        render_loading(frame, areas[1], state, animation_frame);
    } else if let Some(report) = &state.report {
        render_report(frame, areas[1], report, compact);
    } else {
        render_picker(frame, areas[1], state, hits);
    }
    render_footer(frame, areas[2], state, animation_frame);
}

fn render_picker(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    state: &ModpackImportState,
    hits: &mut HitMap,
) {
    let items = if state.candidates.is_empty() {
        vec![ListItem::new(Line::from(vec![
            Span::styled("未找到 .mrpack / .zip", theme::warning()),
            Span::styled(
                "  · 将整合包放到 Downloads、桌面或当前目录后按 r",
                theme::muted(),
            ),
        ]))]
    } else {
        state
            .candidates
            .iter()
            .map(|path| {
                let name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("整合包");
                let parent = path.parent().map(|parent| parent.display().to_string());
                let size = fs::metadata(path)
                    .map(|metadata| format_bytes(metadata.len()))
                    .unwrap_or_else(|_| "大小未知".to_owned());
                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(format!("  {name}"), theme::brand()),
                        Span::styled(format!("  · {size}"), theme::muted()),
                    ]),
                    Line::from(Span::styled(
                        format!("    {}", parent.unwrap_or_default()),
                        theme::muted(),
                    )),
                ])
            })
            .collect()
    };
    let mut list_state = ListState::default();
    if !state.candidates.is_empty() {
        list_state.select(Some(state.selected));
    }
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme::border(true))
                .title(format!("选择整合包 · {}", state.candidates.len()))
                .title_style(theme::title(true))
                .style(theme::surface()),
        )
        .highlight_symbol("▸ ")
        .highlight_style(theme::selected_row());
    frame.render_stateful_widget(list, area, &mut list_state);
    let offset = list_state.offset();
    let inner = Block::default().borders(Borders::ALL).inner(area);
    let visible_items = (inner.height / 2) as usize;
    for (row, index) in (offset..state.candidates.len())
        .take(visible_items)
        .enumerate()
    {
        hits.push(
            Rect::new(inner.x, inner.y + (row as u16) * 2, inner.width, 2),
            HitTarget::Modpack(index),
        );
    }
    ui::scrollbar(
        frame,
        area,
        state.candidates.len(),
        ui::window_start(state.selected, state.candidates.len(), visible_items.max(1)),
        visible_items.max(1),
    );
    ui::scrollbar_hits(
        area,
        state.candidates.len(),
        visible_items.max(1),
        hits,
        crate::hit::ScrollView::Modpacks,
    );
}

fn render_loading(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    state: &ModpackImportState,
    animation_frame: u8,
) {
    let path = state
        .selected_path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "所选整合包".to_owned());
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(
                    format!("{} ", theme::spinner(animation_frame)),
                    theme::warning(),
                ),
                Span::styled(
                    "正在分析整合包清单…",
                    theme::warning().add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(Span::styled(path, theme::accent())),
            Line::from(Span::styled(
                "检查路径安全、MC 版本、加载器、Mod 环境标记、哈希和下载域名。",
                theme::muted(),
            )),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme::border(true))
                .title("静态适配检查")
                .title_style(theme::title(true))
                .style(theme::surface()),
        ),
        area,
    );
}

fn render_footer(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    state: &ModpackImportState,
    animation_frame: u8,
) {
    let (message, style) = if let Some(error) = &state.error {
        (error.clone(), theme::danger())
    } else if state.loading {
        (
            format!("{} 正在读取并检查整合包…", theme::spinner(animation_frame)),
            theme::warning(),
        )
    } else if let Some(report) = &state.report {
        (
            format!(
                "检查完成：{} · 安装 {} · 待确认 {} · 排除 {}",
                status_label(report.status),
                report.install_files,
                report.review_files,
                report.excluded_files
            ),
            status_style(report.status),
        )
    } else if state.candidates.is_empty() {
        (
            "没有可分析的文件；按 r 重新扫描。".to_owned(),
            theme::warning(),
        )
    } else {
        (
            "选择 ZIP / MRPACK；Enter 检查或预览，确认后再应用。".to_owned(),
            theme::muted(),
        )
    };
    let actions = if state.report.is_some() {
        Line::from(vec![
            Span::styled("Esc", theme::key_hint()),
            Span::raw(" 返回文件列表 · "),
            Span::styled("r", theme::key_hint()),
            Span::raw(" 重新扫描 · "),
            Span::styled("q", theme::key_hint()),
            Span::raw(" 退出"),
        ])
    } else {
        Line::from(vec![
            Span::styled("↑/↓", theme::key_hint()),
            Span::raw(" 选择 · "),
            Span::styled("Enter", theme::key_hint()),
            Span::raw(" 检查 · "),
            Span::styled("r", theme::key_hint()),
            Span::raw(" 重新扫描 · "),
            Span::styled("?", theme::key_hint()),
            Span::raw(" 帮助 · "),
            Span::styled("Esc", theme::key_hint()),
            Span::raw(" 返回"),
        ])
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(message, style)),
            actions,
            Line::from(Span::styled(
                "本页检查只读；/import <路径> 可预览，停服后加 --apply 执行导入。",
                theme::muted(),
            )),
        ]),
        area,
    );
}

fn render_report(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    report: &ModpackAnalysisReport,
    compact: bool,
) {
    let loader = report
        .loader
        .as_ref()
        .map(|loader| format!("{} {}", loader.dependency, loader.version))
        .unwrap_or_else(|| "未声明".to_owned());
    let mc = report
        .minecraft_version
        .clone()
        .unwrap_or_else(|| "未知".to_owned());
    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                format!("{} ", status_symbol(report.status)),
                status_style(report.status),
            ),
            Span::styled(
                status_label(report.status),
                status_style(report.status).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("  {}", report.name), theme::accent()),
        ]),
        Line::from(vec![
            Span::styled("版本  ", label_style()),
            Span::styled(report.version_id.clone(), theme::muted()),
            Span::styled(format!(" · MC {mc}"), theme::muted()),
            Span::styled(format!(" · {loader}"), theme::muted()),
        ]),
    ];
    if !compact {
        lines.push(Line::from(vec![
            Span::styled("分流  ", label_style()),
            Span::styled(
                format!(
                    "安装 {} · 待确认 {} · 排除 {} · Mod {}",
                    report.install_files,
                    report.review_files,
                    report.excluded_files,
                    report.mod_files
                ),
                theme::accent(),
            ),
            Span::styled(
                format!(
                    " · {}{}",
                    format_bytes(report.total_install_bytes),
                    if report.server_override_files > 0 {
                        format!(" · server-overrides {} 个", report.server_override_files)
                    } else {
                        String::new()
                    }
                ),
                theme::muted(),
            ),
        ]));
    }
    if report.issues.is_empty() {
        lines.push(Line::from(Span::styled(
            "✓ 未发现阻断项或兼容性警告。",
            theme::success(),
        )));
    } else {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("检查结果", label_style())));
        let limit = if compact { 2 } else { 6 };
        for issue in report.issues.iter().take(limit) {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{} ", severity_symbol(issue.severity)),
                    severity_style(issue.severity),
                ),
                Span::styled(issue.title.clone(), severity_style(issue.severity)),
            ]));
            lines.push(Line::from(Span::styled(
                format!("   {}", issue.detail),
                theme::muted(),
            )));
            if !compact {
                if let Some(action) = &issue.action {
                    lines.push(Line::from(Span::styled(
                        format!("   → {action}"),
                        theme::warning(),
                    )));
                }
            }
        }
        if report.issues.len() > limit {
            lines.push(Line::from(Span::styled(
                format!(
                    "还有 {} 条问题；请在 CLI 报告或后续导入预览中查看。",
                    report.issues.len() - limit
                ),
                theme::muted(),
            )));
        }
    }
    if report.files_truncated {
        lines.push(Line::from(Span::styled(
            "文件清单已截断；摘要计数仍覆盖完整索引。",
            theme::warning(),
        )));
    }
    if !compact {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "本页只读，不会下载 Mod；/import <路径> --apply 可停服导入。",
            theme::muted(),
        )));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(theme::border(true))
                    .title("整合包适配报告")
                    .title_style(theme::title(true))
                    .style(theme::surface()),
            )
            .style(theme::surface()),
        area,
    );
}

fn status_label(status: ModpackStatus) -> &'static str {
    match status {
        ModpackStatus::Ready => "可以导入",
        ModpackStatus::Review => "需要确认",
        ModpackStatus::Blocked => "禁止导入",
    }
}

fn status_symbol(status: ModpackStatus) -> &'static str {
    match status {
        ModpackStatus::Ready => "✓",
        ModpackStatus::Review => "!",
        ModpackStatus::Blocked => "×",
    }
}

fn status_style(status: ModpackStatus) -> ratatui::style::Style {
    match status {
        ModpackStatus::Ready => theme::success(),
        ModpackStatus::Review => theme::warning(),
        ModpackStatus::Blocked => theme::danger(),
    }
}

fn severity_symbol(severity: ModpackSeverity) -> &'static str {
    match severity {
        ModpackSeverity::Blocker => "×",
        ModpackSeverity::Warning => "!",
        ModpackSeverity::Info => "·",
    }
}

fn severity_style(severity: ModpackSeverity) -> ratatui::style::Style {
    match severity {
        ModpackSeverity::Blocker => theme::danger(),
        ModpackSeverity::Warning => theme::warning(),
        ModpackSeverity::Info => theme::muted(),
    }
}

fn label_style() -> ratatui::style::Style {
    theme::muted().add_modifier(Modifier::BOLD)
}

fn format_bytes(bytes: u64) -> String {
    const MIB: u64 = 1024 * 1024;
    const GIB: u64 = MIB * 1024;
    if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests;
