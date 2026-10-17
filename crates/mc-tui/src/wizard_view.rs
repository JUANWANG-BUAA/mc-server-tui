use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::theme;
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
            Span::styled(" ◆ MC SERVER TUI ", theme::brand()),
            Span::raw("新建 Paper 实例 · "),
            Span::styled(step, theme::accent()),
        ]))
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(theme::border(false)),
        ),
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
        Paragraph::new(content).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme::border(true))
                .title("创建向导")
                .title_style(theme::title(true))
                .style(theme::surface()),
        ),
        areas[1],
    );

    let message = wizard
        .error
        .as_deref()
        .map(|error| (error, theme::danger()))
        .unwrap_or_else(|| {
            if wizard.submitting {
                ("◐ 正在提交…", theme::warning())
            } else {
                ("Tab 编辑 · Enter 下一步/提交 · Esc 返回", theme::muted())
            }
        });
    let hint = if wizard.submitting {
        Line::from(vec![
            Span::styled("Ctrl-C", theme::key_hint()),
            Span::raw(" 退出"),
        ])
    } else {
        Line::from(vec![
            Span::styled("Tab", theme::key_hint()),
            Span::raw(" 编辑 · "),
            Span::styled("Enter", theme::key_hint()),
            Span::raw(" 下一步/提交 · "),
            Span::styled("Esc", theme::key_hint()),
            Span::raw(" 返回 · "),
            Span::styled("Ctrl-C", theme::key_hint()),
            Span::raw(" 退出"),
        ])
    };
    frame.render_widget(
        Paragraph::new(vec![Line::from(Span::styled(message.0, message.1)), hint]),
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
            theme::muted().add_modifier(Modifier::BOLD),
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
                theme::success()
            } else {
                Style::default()
            },
        ),
    ])
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
