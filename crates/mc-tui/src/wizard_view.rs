use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::theme;
use crate::wizard::{CreateWizard, WizardStep};

pub(crate) fn render(frame: &mut Frame<'_>, wizard: &CreateWizard, animation_frame: u8) {
    let compact = frame.area().width < 80 || frame.area().height < 20;
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(if compact { 2 } else { 3 }),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(frame.area());

    let title = Line::from(vec![
        Span::styled(" ◆ MC SERVER TUI ", theme::brand()),
        Span::styled("新建 Paper 服务器", theme::accent()),
    ]);
    let header = if compact {
        vec![title]
    } else {
        vec![title, progress_line(wizard.step)]
    };
    frame.render_widget(
        Paragraph::new(header).block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(theme::border(false)),
        ),
        areas[0],
    );

    let body = match wizard.step {
        WizardStep::Server => server_panel(wizard, compact),
        WizardStep::Runtime => runtime_panel(wizard, compact),
        WizardStep::Review => review_panel(wizard, compact),
    };
    let title = match wizard.step {
        WizardStep::Server => "1 · 实例与游戏版本",
        WizardStep::Runtime => "2 · Java、内存与端口",
        WizardStep::Review => "3 · 安装计划与 EULA",
    };
    frame.render_widget(
        Paragraph::new(body)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(theme::border(true))
                    .title(title)
                    .title_style(theme::title(true))
                    .style(theme::surface()),
            )
            .style(theme::surface()),
        areas[1],
    );

    render_footer(frame, areas[2], wizard, animation_frame);
}

fn server_panel(wizard: &CreateWizard, compact: bool) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    if wizard.options_loading {
        lines.push(Line::from(vec![
            Span::styled("◐ ", theme::warning()),
            Span::styled(
                "正在读取 Paper 稳定版本和本机 Java 运行时…",
                theme::warning(),
            ),
        ]));
    }
    if let Some(error) = &wizard.options_error {
        lines.push(Line::from(Span::styled(error.clone(), theme::danger())));
    }
    lines.push(Line::from(vec![
        Span::styled(
            if wizard.focused_field == 0 {
                "› "
            } else {
                "  "
            },
            focus_style(wizard.focused_field == 0),
        ),
        Span::styled("实例名称  ", label_style()),
        Span::styled(
            if wizard.name.is_empty() {
                "输入一个容易识别的名称".to_owned()
            } else {
                wizard.name.clone()
            },
            if wizard.focused_field == 0 {
                theme::selected_row()
            } else {
                Style::default()
            },
        ),
    ]));
    if !compact {
        lines.push(Line::from(Span::styled(
            "名称只用于识别实例，不会写入 Minecraft 世界。",
            theme::muted(),
        )));
        lines.push(Line::from(""));
    }
    lines.push(Line::from(vec![
        Span::styled(
            if wizard.focused_field == 1 {
                "› "
            } else {
                "  "
            },
            focus_style(wizard.focused_field == 1),
        ),
        Span::styled("Minecraft 版本  ", label_style()),
        Span::styled("从 Paper 官方列表选择", theme::muted()),
    ]));
    if wizard.versions.is_empty() {
        lines.push(Line::from(Span::styled(
            "版本列表尚未就绪。",
            theme::muted(),
        )));
    } else {
        let visible: usize = if compact { 3 } else { 8 };
        let start = wizard
            .selected_version
            .saturating_sub(visible.saturating_sub(1));
        for (offset, version) in wizard.versions.iter().enumerate().skip(start).take(visible) {
            let selected = offset == wizard.selected_version;
            let marker = if selected { "  ▸ " } else { "    " };
            let style = if selected {
                theme::selected_row()
            } else {
                theme::muted()
            };
            let group_style = if selected {
                theme::accent()
            } else {
                theme::muted()
            };
            lines.push(Line::from(vec![
                Span::styled(marker, style),
                Span::styled(format!("{:<12}", version.id), theme::accent()),
                Span::styled("  ", theme::muted()),
                Span::styled(version.group.clone(), group_style),
            ]));
        }
        if !compact {
            lines.push(Line::from(Span::styled(
                "RC、pre-release 已隐藏；创建时锁定最新稳定构建和 SHA-256。",
                theme::muted(),
            )));
        }
    }
    lines
}

fn runtime_panel(wizard: &CreateWizard, compact: bool) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    if let Some(error) = &wizard.options_error {
        lines.push(Line::from(Span::styled(error.clone(), theme::danger())));
    }
    lines.push(Line::from(vec![
        Span::styled(
            if wizard.focused_field == 0 {
                "› "
            } else {
                "  "
            },
            focus_style(wizard.focused_field == 0),
        ),
        Span::styled("Java 运行时  ", label_style()),
        Span::styled("自动检测 · ←/→ 更换", theme::muted()),
    ]));
    if wizard.java_runtimes.is_empty() {
        lines.push(Line::from(Span::styled(
            "未找到 Java。普通用户不需要填写路径：请先安装 Java 21+。",
            theme::danger(),
        )));
    } else {
        let visible: usize = if compact { 2 } else { 4 };
        let start = wizard
            .selected_java
            .saturating_sub(visible.saturating_sub(1));
        for (index, _) in wizard
            .java_runtimes
            .iter()
            .enumerate()
            .skip(start)
            .take(visible)
        {
            let selected = index == wizard.selected_java;
            let marker = if selected { "  ▸ " } else { "    " };
            let row_style = if selected {
                theme::selected_row()
            } else {
                theme::muted()
            };
            let mut spans = vec![
                Span::styled(marker, row_style),
                Span::styled(wizard.java_choice_label(index), theme::accent()),
            ];
            if wizard.java_is_recommended(index) {
                spans.push(Span::styled(
                    "  推荐",
                    theme::success().add_modifier(Modifier::BOLD),
                ));
            }
            lines.push(Line::from(spans));
            if !compact && selected {
                if let Some(runtime) = wizard.java_runtimes.get(index) {
                    lines.push(Line::from(Span::styled(
                        format!("      {}", runtime.executable),
                        theme::muted(),
                    )));
                }
            }
        }
    }
    lines.push(Line::from(""));
    let mut memory_line = vec![
        Span::styled(
            if wizard.focused_field == 1 {
                "› "
            } else {
                "  "
            },
            focus_style(wizard.focused_field == 1),
        ),
        Span::styled("内存预设  ", label_style()),
    ];
    let labels = ["0.5-1G", "1-2G", "2-4G", "4-8G"];
    for (index, label) in labels.iter().enumerate() {
        let selected = index == wizard.memory_profile;
        let style = if selected {
            theme::selected_row().add_modifier(Modifier::BOLD)
        } else {
            theme::muted()
        };
        memory_line.push(Span::styled(format!(" {label} "), style));
        if index < 3 {
            memory_line.push(Span::styled("·", theme::muted()));
        }
    }
    lines.push(Line::from(memory_line));
    lines.push(Line::from(vec![
        Span::styled(
            if wizard.focused_field == 2 {
                "› "
            } else {
                "  "
            },
            focus_style(wizard.focused_field == 2),
        ),
        Span::styled("服务端口  ", label_style()),
        Span::styled(
            format!("{} · 已为你选择可用端口", wizard.port),
            if wizard.focused_field == 2 {
                theme::selected_row()
            } else {
                theme::accent()
            },
        ),
    ]));
    if !compact {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Java 路径只在系统未检测到运行时时才需要人工处理。",
            theme::muted(),
        )));
    }
    lines
}

fn review_panel(wizard: &CreateWizard, compact: bool) -> Vec<Line<'static>> {
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
        Line::from(vec![
            Span::styled("服务端  ", label_style()),
            Span::styled(format!("Paper {version}"), theme::accent()),
            Span::styled(" · 最新稳定构建", theme::muted()),
        ]),
        Line::from(vec![
            Span::styled("Java    ", label_style()),
            Span::styled(java, theme::accent()),
        ]),
    ];
    lines.push(Line::from(vec![
        Span::styled("资源    ", label_style()),
        Span::styled(
            format!(
                "{}-{} GiB · 端口 {}",
                min_memory / 1024,
                max_memory / 1024,
                wizard.port
            ),
            theme::accent(),
        ),
    ]));
    if !compact {
        lines.push(Line::from(vec![
            Span::styled("下载    ", label_style()),
            Span::styled("Paper 官方 CDN · 完成后校验 SHA-256", theme::muted()),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "下载可在任务详情中查看字节进度，并可用 x 安全取消。",
            theme::muted(),
        )));
        lines.push(Line::from(Span::styled(
            "已完成的部分会保留，重新创建同一版本时可继续使用。",
            theme::muted(),
        )));
        lines.push(Line::from(""));
    }
    lines.push(eula_line(wizard, compact));
    lines
}

fn render_footer(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    wizard: &CreateWizard,
    animation_frame: u8,
) {
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
    let keys = if wizard.step == WizardStep::Review {
        Line::from(vec![
            Span::styled("Space", theme::key_hint()),
            Span::raw(" 接受 EULA · "),
            Span::styled("Enter", theme::key_hint()),
            Span::raw(" 创建并下载 · "),
            Span::styled("Esc", theme::key_hint()),
            Span::raw(" 返回"),
        ])
    } else {
        Line::from(vec![
            Span::styled("Tab", theme::key_hint()),
            Span::raw(" 切换字段 · "),
            Span::styled("←/→", theme::key_hint()),
            Span::raw(" 更换选项 · "),
            Span::styled("Enter", theme::key_hint()),
            Span::raw(" 下一步 · "),
            Span::styled("Esc", theme::key_hint()),
            Span::raw(" 返回"),
        ])
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(message, style)),
            keys,
            Line::from(vec![
                Span::styled("Ctrl-K", theme::key_hint()),
                Span::raw(" 命令面板 · "),
                Span::styled("Ctrl-C", theme::key_hint()),
                Span::raw(" 退出 · 下载前不会写入实例目录"),
            ]),
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
            spans.push(Span::styled("  ──  ", theme::muted()));
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
        (WizardStep::Server, 0) => "给服务器起一个名称，然后按 Enter 继续。",
        (WizardStep::Server, 1) => "使用 ←/→ 选择 Paper 稳定版本；不会让你手工猜版本号。",
        (WizardStep::Runtime, 0) => "已自动检测 Java；推荐项经过版本与架构识别。",
        (WizardStep::Runtime, 1) => "选择内存预设即可；默认适合小型生存服。",
        (WizardStep::Runtime, 2) => "端口已自动选择可用值；通常无需修改。",
        (WizardStep::Review, _) if !wizard.accept_eula => {
            "Space 明确接受 EULA，再按 Enter 创建；不会替你默认接受。"
        }
        (WizardStep::Review, _) => "确认无误后按 Enter，创建任务会进入后台下载中心。",
        _ => "使用 Tab 切换字段，←/→ 更换选项。",
    }
}

fn eula_line(wizard: &CreateWizard, compact: bool) -> Line<'static> {
    let marker = if wizard.accept_eula { "✓" } else { "○" };
    let label = if compact {
        format!("[{marker}] 接受 Minecraft EULA（Space）")
    } else {
        format!("[{marker}] 我已阅读并接受 Minecraft EULA（Space 切换）")
    };
    Line::from(vec![
        Span::styled(
            if wizard.focused_field == 0 {
                "› "
            } else {
                "  "
            },
            focus_style(wizard.focused_field == 0),
        ),
        Span::styled(
            label,
            if wizard.accept_eula {
                theme::success()
            } else {
                theme::warning()
            },
        ),
    ])
}

fn label_style() -> Style {
    theme::muted().add_modifier(Modifier::BOLD)
}

fn focus_style(focused: bool) -> Style {
    if focused {
        theme::key_hint()
    } else {
        theme::muted()
    }
}

#[cfg(test)]
mod tests;
