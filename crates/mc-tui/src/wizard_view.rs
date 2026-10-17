use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::wizard::{CreateWizard, WizardStep};

pub(crate) fn render(frame: &mut Frame<'_>, wizard: &CreateWizard) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(frame.area());

    let step = match wizard.step {
        WizardStep::Server => "1/3 · 服务信息",
        WizardStep::Runtime => "2/3 · Java 与资源",
        WizardStep::Review => "3/3 · 预览与确认",
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                " MC SERVER TUI   ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw("新建 Paper 实例 · "),
            Span::styled(step, Style::default().fg(Color::Cyan)),
        ]))
        .block(Block::default().borders(Borders::BOTTOM)),
        areas[0],
    );

    let compact = frame.area().width < 80 || frame.area().height < 20;
    let memory = format!("{}–{} MiB", wizard.min_memory_mb, wizard.max_memory_mb);
    let server_summary = format!("{} · {}", wizard.name, wizard.game_version);
    let resource_summary = format!("{memory} · 端口 {}", wizard.port);
    let content = if compact {
        match wizard.step {
            WizardStep::Server => vec![
                field("实例名称", &wizard.name, wizard.focused_field == 0),
                field(
                    "Minecraft 版本",
                    &wizard.game_version,
                    wizard.focused_field == 1,
                ),
            ],
            WizardStep::Runtime => vec![
                field("Java", &wizard.java_path, wizard.focused_field == 0),
                field("最小 MiB", &wizard.min_memory_mb, wizard.focused_field == 1),
                field("最大 MiB", &wizard.max_memory_mb, wizard.focused_field == 2),
                field("服务端口", &wizard.port, wizard.focused_field == 3),
            ],
            WizardStep::Review => vec![
                value("实例", &server_summary),
                value("Java", &wizard.java_path),
                value("资源", &resource_summary),
                eula_line(wizard, true),
            ],
        }
    } else {
        match wizard.step {
            WizardStep::Server => vec![
                Line::from("为新实例设置名称和 Minecraft 版本。"),
                Line::from(""),
                field("实例名称", &wizard.name, wizard.focused_field == 0),
                field(
                    "Minecraft 版本",
                    &wizard.game_version,
                    wizard.focused_field == 1,
                ),
            ],
            WizardStep::Runtime => vec![
                Line::from("Java 路径需指向可执行文件；daemon 会再次检查。"),
                Line::from(""),
                field(
                    "Java 可执行文件",
                    &wizard.java_path,
                    wizard.focused_field == 0,
                ),
                field(
                    "最小内存 MiB",
                    &wizard.min_memory_mb,
                    wizard.focused_field == 1,
                ),
                field(
                    "最大内存 MiB",
                    &wizard.max_memory_mb,
                    wizard.focused_field == 2,
                ),
                field("服务端口", &wizard.port, wizard.focused_field == 3),
            ],
            WizardStep::Review => vec![
                Line::from("请确认创建参数。提交后 daemon 会在后台下载并安装。"),
                Line::from(""),
                value("实例名称", &wizard.name),
                value("Minecraft 版本", &wizard.game_version),
                value("Java", &wizard.java_path),
                value("内存", &memory),
                value("服务端口", &wizard.port),
                Line::from(""),
                eula_line(wizard, false),
            ],
        }
    };
    frame.render_widget(
        Paragraph::new(content).block(Block::default().borders(Borders::ALL).title("创建向导")),
        areas[1],
    );

    let message = wizard
        .error
        .as_deref()
        .map(|error| (error, Style::default().fg(Color::Red)))
        .unwrap_or_else(|| {
            if wizard.submitting {
                ("正在提交…", Style::default().fg(Color::Yellow))
            } else {
                (
                    "Tab 编辑 · Enter 下一步/提交 · Esc 返回",
                    Style::default().fg(Color::DarkGray),
                )
            }
        });
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(message.0, message.1)),
            Line::from("Ctrl-C 退出"),
        ]),
        areas[2],
    );
}

fn field<'a>(label: &'a str, value: &'a str, focused: bool) -> Line<'a> {
    let shown = if value.is_empty() {
        "（输入内容）"
    } else {
        value
    };
    Line::from(vec![
        Span::styled(if focused { "› " } else { "  " }, focus_style(focused)),
        Span::styled(
            format!("{label}："),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(shown),
    ])
}

fn value<'a>(label: &'a str, value: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("{label}："),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(value),
    ])
}

fn eula_line(wizard: &CreateWizard, compact: bool) -> Line<'static> {
    let label = if compact {
        format!(
            "[{}] 已阅读并接受 EULA (Space)",
            if wizard.accept_eula { "x" } else { " " }
        )
    } else {
        format!(
            "[{}] 我已阅读并接受 Minecraft EULA（Space 切换）",
            if wizard.accept_eula { "x" } else { " " }
        )
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
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            },
        ),
    ])
}

fn focus_style(focused: bool) -> Style {
    if focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{backend::TestBackend, Terminal};

    use crate::wizard::{CreateWizard, WizardAction, WizardStep};

    use super::render;

    #[test]
    fn renders_all_steps_and_explicit_eula_control_at_reference_width() {
        let mut wizard = CreateWizard {
            name: "Survival".to_owned(),
            game_version: "1.21.11".to_owned(),
            java_path: "/usr/bin/java".to_owned(),
            ..CreateWizard::default()
        };
        let server = render_to_text(&wizard, 80, 24);
        assert!(server.contains("服务信息"));
        assert!(server.contains("Survival"));
        assert!(render_to_text(&wizard, 40, 12).contains("Survival"));

        wizard.handle(WizardAction::Advance);
        let runtime = render_to_text(&wizard, 80, 24);
        assert!(runtime.contains("Java与资源"));
        assert!(runtime.contains("/usr/bin/java"));
        assert!(render_to_text(&wizard, 40, 12).contains("/usr/bin/java"));

        wizard.handle(WizardAction::Advance);
        assert_eq!(wizard.step, WizardStep::Review);
        let review = render_to_text(&wizard, 80, 24);
        assert!(review.contains("预览与确认"));
        assert!(review.contains("我已阅读并接受MinecraftEULA"));
        assert!(render_to_text(&wizard, 40, 12).contains("已阅读并接受EULA"));
    }

    #[test]
    fn compact_wizard_render_does_not_panic() {
        let text = render_to_text(&CreateWizard::default(), 40, 12);
        assert!(text.contains("新建Paper实例"));
    }

    fn render_to_text(wizard: &CreateWizard, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|frame| render(frame, wizard))
            .expect("render");
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
