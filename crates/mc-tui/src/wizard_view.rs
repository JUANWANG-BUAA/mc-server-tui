use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::theme;
use crate::{
    hit::{HitMap, HitTarget},
    ui,
    wizard::{CreateWizard, WizardStep},
};

const MEMORY_LABELS: [&str; 4] = ["0.5-1G", "1-2G", "2-4G", "4-8G"];

pub(crate) fn render(
    frame: &mut Frame<'_>,
    wizard: &CreateWizard,
    animation_frame: u8,
    hits: &mut HitMap,
) {
    let compact = frame.area().width < 80 || frame.area().height < 20;
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(if compact { 2 } else { 3 }),
            Constraint::Min(1),
            Constraint::Length(if compact { 2 } else { 3 }),
        ])
        .split(frame.area());

    render_header(frame, areas[0], wizard, compact);
    match wizard.step {
        WizardStep::Server => render_server_step(frame, areas[1], wizard, compact, hits),
        WizardStep::Runtime => render_runtime_step(frame, areas[1], wizard, compact, hits),
        WizardStep::Review => render_review_step(frame, areas[1], wizard, compact, hits),
    }
    render_footer(frame, areas[2], wizard, animation_frame);
}

fn render_header(frame: &mut Frame<'_>, area: Rect, wizard: &CreateWizard, compact: bool) {
    let step_number = match wizard.step {
        WizardStep::Server => 1,
        WizardStep::Runtime => 2,
        WizardStep::Review => 3,
    };
    let title = Line::from(vec![
        Span::styled(" ◆ MC SERVER TUI ", theme::brand()),
        Span::styled("新建 Paper 服务器", theme::accent()),
        Span::styled(
            format!("   第 {step_number}/3 步"),
            theme::dim().add_modifier(Modifier::BOLD),
        ),
    ]);
    let lines = if compact {
        vec![title]
    } else {
        vec![title, progress_line(wizard.step)]
    };
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_type(theme::border_type())
                .border_style(theme::border(false)),
        ),
        area,
    );
}

fn step_title(step: WizardStep) -> &'static str {
    match step {
        WizardStep::Server => " 1 · 实例与游戏版本 ",
        WizardStep::Runtime => " 2 · Java、内存与端口 ",
        WizardStep::Review => " 3 · 安装计划与 EULA ",
    }
}

fn panel(wizard: &CreateWizard) -> Block<'static> {
    let hint = match wizard.step {
        WizardStep::Review => " Space 切换 EULA ",
        _ => " Tab 切换字段 ",
    };
    theme::panel(step_title(wizard.step), true)
        .title_bottom(Line::from(Span::styled(hint, theme::dim())).alignment(Alignment::Right))
}

fn render_server_step(
    frame: &mut Frame<'_>,
    area: Rect,
    wizard: &CreateWizard,
    compact: bool,
    hits: &mut HitMap,
) {
    let block = panel(wizard);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if wizard.options_loading {
        render_loading(
            frame,
            inner,
            "正在读取 Paper 稳定版本和本机 Java 运行时…",
            0,
        );
        return;
    }
    if let Some(error) = &wizard.options_error {
        render_loading(frame, inner, error, 1);
        return;
    }

    let name_focused = wizard.focused_field == 0;
    let name = if wizard.name.is_empty() {
        "输入一个容易识别的名称".to_owned()
    } else {
        wizard.name.clone()
    };
    let name_style = if name_focused {
        theme::selected_row()
    } else if wizard.name.is_empty() {
        theme::dim()
    } else {
        theme::accent()
    };
    let name_row = Rect::new(inner.x, inner.y, inner.width, 1);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                if name_focused { " › " } else { "   " },
                focus_style(name_focused),
            ),
            Span::styled("实例名称  ", label_style()),
            Span::styled(name, name_style),
        ])),
        name_row,
    );
    if name_focused {
        let x = name_row
            .x
            .saturating_add(13 + wizard.name.chars().count() as u16)
            .min(name_row.right().saturating_sub(1));
        frame.set_cursor_position((x, name_row.y));
    }
    hits.push(name_row, HitTarget::WizardField(0));

    if compact || inner.height < 7 {
        render_version_list(frame, inner, wizard, hits, inner.y + 2);
        return;
    }

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "名称只用于识别实例，不会写入 Minecraft 世界。",
            theme::muted(),
        ))),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );

    let header = Line::from(vec![
        Span::styled(
            if wizard.focused_field == 1 {
                " › "
            } else {
                "   "
            },
            focus_style(wizard.focused_field == 1),
        ),
        Span::styled("Minecraft 版本  ", label_style()),
        Span::styled(
            ui::window_label(
                wizard.selected_version,
                wizard.versions.len(),
                version_viewport(inner.height, compact),
            ),
            theme::accent(),
        ),
        Span::styled("   ↑↓ 选择 · PgUp/PgDn 翻页", theme::dim()),
    ]);
    frame.render_widget(
        Paragraph::new(header),
        Rect::new(inner.x, inner.y + 3, inner.width, 1),
    );
    render_version_list(frame, inner, wizard, hits, inner.y + 4);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "RC、pre-release 已隐藏；创建时锁定最新稳定构建和 SHA-256。",
            theme::dim(),
        ))),
        Rect::new(inner.x, inner.bottom().saturating_sub(1), inner.width, 1),
    );
}

fn version_viewport(inner_height: u16, compact: bool) -> usize {
    let used = if compact { 2 } else { 5 };
    inner_height.saturating_sub(used).max(1) as usize
}

fn render_version_list(
    frame: &mut Frame<'_>,
    panel_inner: Rect,
    wizard: &CreateWizard,
    hits: &mut HitMap,
    list_y: u16,
) {
    let compact = panel_inner.height < 10;
    let bottom_reserved = if compact { 0 } else { 1 };
    let list_height = panel_inner
        .bottom()
        .saturating_sub(list_y)
        .saturating_sub(bottom_reserved);
    let list_area = Rect::new(panel_inner.x, list_y, panel_inner.width, list_height);
    if list_area.height == 0 {
        return;
    }
    let items: Vec<ListItem<'_>> = if wizard.versions.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "版本列表尚未就绪。",
            theme::muted(),
        )))]
    } else {
        wizard
            .versions
            .iter()
            .enumerate()
            .map(|(index, version)| {
                let mut spans = vec![
                    Span::styled(format!("{:<12}", version.id), theme::text()),
                    Span::styled(format!("  {}", version.group), theme::muted()),
                ];
                if index == 0 {
                    spans.push(Span::styled("   最新稳定", theme::success()));
                }
                ListItem::new(Line::from(spans))
            })
            .collect()
    };
    let mut state = ListState::default();
    if !wizard.versions.is_empty() {
        state.select(Some(wizard.selected_version));
    }
    let list = List::new(items)
        .block(Block::default())
        .highlight_symbol("▸ ")
        .highlight_style(theme::selected_row())
        .scroll_padding(1);
    frame.render_stateful_widget(list, list_area, &mut state);

    let offset = state.offset();
    let visible = list_area.height as usize;
    for (row, index) in (offset..wizard.versions.len()).take(visible).enumerate() {
        hits.push(
            Rect::new(list_area.x, list_area.y + row as u16, list_area.width, 1),
            HitTarget::Version(index),
        );
    }
    let panel = Rect::new(
        panel_inner.x.saturating_sub(1),
        panel_inner.y.saturating_sub(1),
        panel_inner.width + 2,
        panel_inner.height + 2,
    );
    ui::scrollbar(
        frame,
        panel,
        wizard.versions.len(),
        wizard.selected_version,
        visible.max(1),
    );
}

fn render_runtime_step(
    frame: &mut Frame<'_>,
    area: Rect,
    wizard: &CreateWizard,
    compact: bool,
    hits: &mut HitMap,
) {
    let block = panel(wizard);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if wizard.java_runtimes.is_empty() {
        render_loading(
            frame,
            inner,
            "未找到可用 Java。请先安装 Java 21 或更高版本，再重新打开向导。",
            1,
        );
        return;
    }

    let java_header = Line::from(vec![
        Span::styled(
            if wizard.focused_field == 0 {
                " › "
            } else {
                "   "
            },
            focus_style(wizard.focused_field == 0),
        ),
        Span::styled("Java 运行时  ", label_style()),
        Span::styled(
            ui::window_label(
                wizard.selected_java,
                wizard.java_runtimes.len(),
                java_viewport(inner.height, compact),
            ),
            theme::accent(),
        ),
        Span::styled("   自动检测 · ↑↓ 选择", theme::dim()),
    ]);
    frame.render_widget(
        Paragraph::new(java_header),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );

    let java_list_height = if compact {
        2
    } else {
        4.min(inner.height.saturating_sub(6))
    };
    let java_area = Rect::new(inner.x, inner.y + 1, inner.width, java_list_height);
    let java_items: Vec<ListItem<'_>> = wizard
        .java_runtimes
        .iter()
        .enumerate()
        .map(|(index, runtime)| {
            let mut spans = vec![
                Span::styled(wizard.java_choice_label(index), theme::text()),
                if wizard.java_is_recommended(index) {
                    Span::styled("   推荐", theme::success().add_modifier(Modifier::BOLD))
                } else {
                    Span::styled("", theme::muted())
                },
            ];
            if !compact {
                spans.push(Span::styled(
                    format!("   {}", ui::truncate(&runtime.executable, 34)),
                    theme::dim(),
                ));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();
    let mut java_state = ListState::default();
    java_state.select(Some(wizard.selected_java));
    let java_list = List::new(java_items)
        .block(Block::default())
        .highlight_symbol("▸ ")
        .highlight_style(theme::selected_row())
        .scroll_padding(1);
    frame.render_stateful_widget(java_list, java_area, &mut java_state);

    let java_offset = java_state.offset();
    for (row, index) in (java_offset..wizard.java_runtimes.len())
        .take(java_area.height as usize)
        .enumerate()
    {
        hits.push(
            Rect::new(java_area.x, java_area.y + row as u16, java_area.width, 1),
            HitTarget::Java(index),
        );
    }
    let panel = Rect::new(
        inner.x.saturating_sub(1),
        inner.y.saturating_sub(1),
        inner.width + 2,
        inner.height + 2,
    );
    ui::scrollbar(
        frame,
        panel,
        wizard.java_runtimes.len(),
        wizard.selected_java,
        java_area.height as usize,
    );

    let memory_y = java_area.bottom() + 1;
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                if wizard.focused_field == 1 {
                    " › "
                } else {
                    "   "
                },
                focus_style(wizard.focused_field == 1),
            ),
            Span::styled("内存预设  ", label_style()),
            Span::styled("←/→ 或 ↑/↓ 切换", theme::dim()),
        ])),
        Rect::new(inner.x, memory_y, inner.width, 1),
    );
    render_memory_chips(frame, inner, memory_y + 1, wizard, hits);

    let port_y = memory_y + 2;
    let port_focused = wizard.focused_field == 2;
    let port_style = if port_focused {
        theme::selected_row()
    } else {
        theme::accent()
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                if port_focused { " › " } else { "   " },
                focus_style(port_focused),
            ),
            Span::styled("服务端口  ", label_style()),
            Span::styled(format!("{:>5}", wizard.port), port_style),
            Span::styled("   已自动选择可用端口", theme::dim()),
        ])),
        Rect::new(inner.x, port_y, inner.width, 1),
    );
    hits.push(
        Rect::new(inner.x, port_y, inner.width, 1),
        HitTarget::WizardField(2),
    );

    if !compact && inner.height > 10 {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "Java 路径无需手填；系统会绑定可执行的运行时，启动前再次预检。",
                theme::dim(),
            ))),
            Rect::new(inner.x, inner.bottom().saturating_sub(1), inner.width, 1),
        );
    }
}

fn java_viewport(inner_height: u16, compact: bool) -> usize {
    if compact {
        2
    } else {
        4.min(inner_height.saturating_sub(6)).max(1) as usize
    }
}

fn render_memory_chips(
    frame: &mut Frame<'_>,
    inner: Rect,
    y: u16,
    wizard: &CreateWizard,
    hits: &mut HitMap,
) {
    if y >= inner.bottom() {
        return;
    }
    let row = Rect::new(inner.x, y, inner.width, 1);
    let label_width = 14u16;
    let chip_width = 11u16;
    let mut constraints = vec![Constraint::Length(label_width)];
    constraints.extend(std::iter::repeat_n(
        Constraint::Length(chip_width),
        MEMORY_LABELS.len(),
    ));
    constraints.push(Constraint::Min(1));
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(row);
    for (index, label) in MEMORY_LABELS.iter().enumerate() {
        let selected = index == wizard.memory_profile;
        let chip_area = columns[index + 1];
        let style = if selected {
            theme::selected_chip()
        } else if wizard.focused_field == 1 {
            theme::elevated()
        } else {
            theme::surface()
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(format!(" {label} "), style)))
                .alignment(Alignment::Center),
            chip_area,
        );
        hits.push(
            Rect::new(chip_area.x, chip_area.y, chip_area.width, 1),
            HitTarget::Memory(index),
        );
    }
}

fn render_review_step(
    frame: &mut Frame<'_>,
    area: Rect,
    wizard: &CreateWizard,
    compact: bool,
    hits: &mut HitMap,
) {
    let block = panel(wizard);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height < 4 {
        return;
    }

    let version = wizard
        .selected_version()
        .map(|version| version.id.clone())
        .unwrap_or_else(|| "未选择".to_owned());
    let java = wizard.java_choice_label(wizard.selected_java);
    let (min_memory, max_memory) = wizard.memory_range();
    let mut lines = vec![
        Line::from(Span::styled(
            "✓ 安装计划已就绪；提交后才会开始下载。",
            theme::success().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        plan_line("实例    ", wizard.name.clone(), ""),
        plan_line("服务端  ", format!("Paper {version}"), " · 最新稳定构建"),
        plan_line("Java    ", java, ""),
        plan_line(
            "资源    ",
            format!(
                "{}-{} GiB · 端口 {}",
                min_memory / 1024,
                max_memory / 1024,
                wizard.port
            ),
            "",
        ),
        plan_line(
            "下载    ",
            "Paper 官方 CDN".to_owned(),
            " · 完成后校验 SHA-256",
        ),
    ];
    if !compact {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "下载字节进度可在任务详情查看；按 x 两次可安全取消。",
            theme::dim(),
        )));
        lines.push(Line::from(Span::styled(
            "已完成的部分会保留，重新创建同一版本时可继续使用。",
            theme::dim(),
        )));
    }
    lines.push(Line::from(""));
    let eula_index = lines.len();
    let eula_focused = wizard.focused_field == 0;
    let marker = if wizard.accept_eula { "✓" } else { "○" };
    lines.push(Line::from(vec![
        Span::styled(
            if eula_focused { " ▸ " } else { "   " },
            focus_style(eula_focused),
        ),
        Span::styled(
            format!("[{marker}] "),
            if wizard.accept_eula {
                theme::success()
            } else {
                theme::warning()
            },
        ),
        Span::styled(
            if compact {
                "接受 Minecraft EULA（Space）".to_owned()
            } else {
                "我已阅读并接受 Minecraft EULA（Space 切换）".to_owned()
            },
            if wizard.accept_eula {
                theme::success()
            } else {
                theme::warning().add_modifier(Modifier::BOLD)
            },
        ),
    ]));
    frame.render_widget(Paragraph::new(lines), inner);
    if (eula_index as u16) < inner.height {
        let y = inner.y + eula_index as u16;
        hits.push(Rect::new(inner.x, y, inner.width, 1), HitTarget::ToggleEula);
    }
}

fn plan_line(label: &'static str, value: String, suffix: &'static str) -> Line<'static> {
    Line::from(vec![
        Span::styled(label, label_style()),
        Span::styled(value, theme::text().add_modifier(Modifier::BOLD)),
        Span::styled(suffix, theme::muted()),
    ])
}

fn render_loading(frame: &mut Frame<'_>, inner: Rect, message: &str, danger: u8) {
    let style = if danger == 1 {
        theme::danger()
    } else {
        theme::warning()
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled("◐ ", style),
                Span::styled(message.to_owned(), style),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "请稍候；数据来自 Paper 官方元数据和本机 doctor 预检。",
                theme::dim(),
            )),
        ]),
        inner,
    );
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, wizard: &CreateWizard, animation_frame: u8) {
    let (message, style) = if let Some(error) = &wizard.error {
        (error.clone(), theme::danger())
    } else if let Some(error) = &wizard.options_error {
        (error.clone(), theme::danger())
    } else if wizard.options_loading {
        (
            format!(
                "{} 正在准备推荐版本与 Java…",
                theme::spinner(animation_frame)
            ),
            theme::warning(),
        )
    } else if wizard.submitting {
        (
            format!("{} 正在提交创建任务…", theme::spinner(animation_frame)),
            theme::warning(),
        )
    } else {
        (step_guidance(wizard).to_owned(), theme::muted())
    };
    let primary = match wizard.step {
        WizardStep::Review => ui::plain_hint_line(&[
            ("Space", "接受 EULA"),
            ("Enter", "创建并下载"),
            ("Esc", "返回"),
        ]),
        WizardStep::Runtime => ui::plain_hint_line(&[
            ("Tab", "切换字段"),
            ("↑/↓", "选择"),
            ("Enter", "下一步"),
            ("Esc", "返回"),
        ]),
        WizardStep::Server => ui::plain_hint_line(&[
            ("Tab", "切换字段"),
            ("↑/↓", "选择版本"),
            ("Enter", "下一步"),
            ("Esc", "返回"),
        ]),
    };
    let secondary = ui::plain_hint_line(&[
        ("PgUp/PgDn", "翻页"),
        ("Home/End", "首尾"),
        ("Ctrl-K", "命令"),
        ("F1", "帮助"),
        ("Ctrl-C", "退出"),
    ])
    .style(theme::dim());
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(message, style)),
            primary,
            secondary,
        ]),
        area,
    );
}

fn progress_line(step: WizardStep) -> Line<'static> {
    let current = match step {
        WizardStep::Server => 0,
        WizardStep::Runtime => 1,
        WizardStep::Review => 2,
    };
    let labels = ["基本信息", "运行环境", "安装计划"];
    let mut spans = Vec::new();
    for (index, label) in labels.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled("   ──   ", theme::dim()));
        }
        let (marker, style) = if index < current {
            ("● ", theme::success())
        } else if index == current {
            ("● ", theme::accent().add_modifier(Modifier::BOLD))
        } else {
            ("○ ", theme::muted())
        };
        spans.push(Span::styled(format!("{marker}{label}"), style));
    }
    Line::from(spans)
}

fn step_guidance(wizard: &CreateWizard) -> &'static str {
    match (wizard.step, wizard.focused_field) {
        (WizardStep::Server, 0) => "输入名称，按 Tab 进入版本列表；版本用 ↑/↓ 选择。",
        (WizardStep::Server, 1) => "↑/↓ 选择 Paper 稳定版本；PgUp/PgDn 翻页，Enter 下一步。",
        (WizardStep::Runtime, 0) => "↑/↓ 选择 Java 运行时；推荐项已按版本与架构识别。",
        (WizardStep::Runtime, 1) => "←/→ 或 ↑/↓ 切换内存档位；默认适合小型生存服。",
        (WizardStep::Runtime, 2) => "端口已自动选择可用值；可直接编辑，Enter 下一步。",
        (WizardStep::Review, _) if !wizard.accept_eula => {
            "按 Space 明确接受 EULA，再按 Enter 创建。"
        }
        (WizardStep::Review, _) => "确认无误后按 Enter，创建任务会进入后台下载中心。",
        _ => "使用 Tab 切换字段，↑/↓ 选择选项。",
    }
}

fn label_style() -> Style {
    theme::muted().add_modifier(Modifier::BOLD)
}

fn focus_style(focused: bool) -> Style {
    if focused {
        theme::key_hint()
    } else {
        theme::dim()
    }
}

#[cfg(test)]
mod tests;
