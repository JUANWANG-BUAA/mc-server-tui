use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::{
    app::AppState,
    server_properties::{EditorPhase, ServerPropertiesEditor},
    theme,
};

const FIELD_LABELS: [&str; 4] = ["MOTD", "人数上限", "难度", "视距"];
const FIELD_HELP: [&str; 4] = [
    "显示在客户端服务器列表中；支持中文与 Minecraft 格式码。",
    "同时可加入服务器的玩家数量，范围 1–1000。",
    "和平 / 简单 / 普通 / 困难；影响敌对生物与生存压力。",
    "服务器发送区块的半径，单位为区块；范围 3–32。",
];

pub(crate) fn render(frame: &mut Frame<'_>, area: Rect, app: &AppState) {
    let Some(editor) = app.server_properties.as_ref() else {
        frame.render_widget(
            Paragraph::new("正在准备配置编辑器…").style(theme::muted()),
            area,
        );
        return;
    };
    let instance_name = app
        .instances
        .iter()
        .find(|instance| instance.id == editor.instance_id)
        .map(|instance| instance.name.as_str())
        .unwrap_or("未知实例");
    let compact = area.width < 72 || area.height < 17;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(theme::border_type())
        .border_style(theme::border(true))
        .title(format!(" server.properties · {instance_name} "))
        .title_style(theme::title(true))
        .style(theme::surface());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let [heading, body, footer] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(if compact { 1 } else { 2 }),
            Constraint::Min(1),
            Constraint::Length(if compact { 2 } else { 3 }),
        ])
        .areas(inner);

    let phase_label = match editor.phase {
        EditorPhase::Loading => "正在读取文件",
        EditorPhase::LoadFailed => "读取失败",
        EditorPhase::Edit => "编辑配置",
        EditorPhase::Review => "保存前检查差异",
        EditorPhase::Saving => "正在原子保存",
        EditorPhase::Saved => "配置已保存",
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(phase_label, theme::accent().add_modifier(Modifier::BOLD)),
            Span::styled("  ·  保存后下次启动生效", theme::muted()),
        ])),
        heading,
    );

    match editor.phase {
        EditorPhase::Loading => frame.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(
                    "正在读取 server/server.properties…",
                    theme::text(),
                )),
                Line::from(Span::styled("文件较大时读取仍保持在后台。", theme::muted())),
            ])
            .wrap(Wrap { trim: false }),
            body,
        ),
        EditorPhase::LoadFailed => render_error(frame, body, editor, "按 F5 重试读取；Esc 返回。"),
        EditorPhase::Saving => frame.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(
                    "正在校验文件版本并提交临时文件…",
                    theme::warning(),
                )),
                Line::from(Span::styled("保存完成前不会覆盖原文件。", theme::muted())),
            ]),
            body,
        ),
        EditorPhase::Edit => render_form(frame, body, editor, compact),
        EditorPhase::Review => render_review(frame, body, editor),
        EditorPhase::Saved => render_saved(frame, body, app, editor),
    }
    render_footer(frame, footer, editor, compact);
}

fn render_form(frame: &mut Frame<'_>, area: Rect, editor: &ServerPropertiesEditor, compact: bool) {
    let mut lines = Vec::with_capacity(16);
    for index in 0..4 {
        let focused = editor.focused_field == index;
        let base_style = if focused {
            theme::accent().add_modifier(Modifier::BOLD)
        } else {
            theme::text()
        };
        let label = format!(
            "{} {}",
            if focused { "›" } else { " " },
            FIELD_LABELS[index]
        );
        let value = if index == 2 {
            format!(
                "{}（{}）",
                editor.draft.difficulty.as_str(),
                editor.draft.difficulty.label()
            )
        } else {
            editor
                .field_text(index)
                .map(|text| {
                    if focused {
                        with_cursor(text, editor.cursors[index])
                    } else {
                        text.to_owned()
                    }
                })
                .unwrap_or_default()
        };
        let value_style = if focused {
            theme::selected_row().patch(theme::accent())
        } else {
            theme::muted()
        };
        if compact {
            lines.push(Line::from(vec![
                Span::styled(format!("{label}: "), base_style),
                Span::styled(value, value_style),
            ]));
        } else {
            lines.push(Line::from(Span::styled(label, base_style)));
            lines.push(Line::from(vec![
                Span::raw("   "),
                Span::styled(value, value_style),
            ]));
            lines.push(Line::from(Span::styled(
                format!("   {}", FIELD_HELP[index]),
                theme::dim(),
            )));
        }
    }
    if compact {
        let helper = if editor.conflict {
            "文件已外部修改；F5 重新读取会丢弃当前草稿。"
        } else {
            FIELD_HELP[editor.focused_field]
        };
        lines.push(Line::from(Span::styled(helper, theme::dim())));
    }
    if let Some(error) = &editor.error {
        lines.push(Line::from(Span::styled(
            format!("! {error}"),
            theme::danger(),
        )));
    }
    if editor.conflict {
        lines.push(Line::from(Span::styled(
            "文件已在外部修改；当前草稿已保留。F5 会重新读取并丢弃草稿。",
            theme::warning(),
        )));
    }
    for warning in editor.warnings.iter().take(2) {
        lines.push(Line::from(Span::styled(
            format!("! {warning}"),
            theme::warning(),
        )));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .style(theme::surface()),
        area,
    );
}

fn render_review(frame: &mut Frame<'_>, area: Rect, editor: &ServerPropertiesEditor) {
    let original = editor.original.as_ref();
    let current = editor.current_draft_values().ok();
    let old = original
        .map(|values| {
            [
                values.motd.clone(),
                values.max_players.to_string(),
                format!(
                    "{}（{}）",
                    values.difficulty.as_str(),
                    values.difficulty.label()
                ),
                values.view_distance.to_string(),
            ]
        })
        .unwrap_or_default();
    let new = current
        .map(|values| {
            [
                values.motd,
                values.max_players.to_string(),
                format!(
                    "{}（{}）",
                    values.difficulty.as_str(),
                    values.difficulty.label()
                ),
                values.view_distance.to_string(),
            ]
        })
        .unwrap_or_default();
    let mut lines = vec![Line::from(Span::styled(
        "将写入以下差异：",
        theme::text().add_modifier(Modifier::BOLD),
    ))];
    for index in 0..4 {
        if old[index] != new[index] {
            lines.push(Line::from(vec![
                Span::styled(format!("{}  ", FIELD_LABELS[index]), theme::muted()),
                Span::styled(
                    format!(
                        "{}  →  {}",
                        clipped(&old[index], 14),
                        clipped(&new[index], 14)
                    ),
                    theme::accent(),
                ),
            ]));
        }
    }
    lines.push(Line::from(Span::styled(
        "未知键、注释及其他行会保留原样。",
        theme::success(),
    )));
    lines.push(Line::from(Span::styled(
        "保存会原子替换文件，不会自动停止或重启服务端。",
        theme::warning(),
    )));
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

fn render_saved(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &AppState,
    editor: &ServerPropertiesEditor,
) {
    let running = app.servers.get(&editor.instance_id).is_some_and(|server| {
        matches!(
            server.state,
            mc_core::ServerState::Starting
                | mc_core::ServerState::Running
                | mc_core::ServerState::Stopping
        )
    });
    let lines = if running {
        vec![
            Line::from(Span::styled("✓ 配置文件已安全保存。", theme::success())),
            Line::from(Span::styled(
                "当前进程仍使用旧值；优雅停止后再启动，才会读取新配置。",
                theme::warning(),
            )),
            Line::from(Span::styled(
                "本次保存不会触发重启，也不会中断在线玩家。",
                theme::muted(),
            )),
        ]
    } else {
        vec![
            Line::from(Span::styled("✓ 配置文件已安全保存。", theme::success())),
            Line::from(Span::styled(
                "服务器下次启动时会读取这些值。",
                theme::text(),
            )),
            Line::from(Span::styled("当前没有运行中的进程。", theme::muted())),
        ]
    };
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

fn render_error(frame: &mut Frame<'_>, area: Rect, editor: &ServerPropertiesEditor, hint: &str) {
    let reason = editor.error.as_deref().unwrap_or("未知错误");
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                "读取配置失败",
                theme::danger().add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(reason, theme::text())),
            Line::from(Span::styled(hint, theme::muted())),
        ])
        .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_footer(
    frame: &mut Frame<'_>,
    area: Rect,
    editor: &ServerPropertiesEditor,
    compact: bool,
) {
    let lines = match editor.phase {
        EditorPhase::Loading => vec![Line::from(vec![
            Span::styled("Esc", theme::key_hint()),
            Span::raw(if compact {
                " 返回"
            } else {
                " 返回；读取完成后草稿会保留。"
            }),
        ])],
        EditorPhase::LoadFailed => vec![Line::from(vec![
            Span::styled("F5 / Enter", theme::key_hint()),
            Span::raw(" 重试读取  ·  "),
            Span::styled("Esc", theme::key_hint()),
            Span::raw(" 返回"),
        ])],
        EditorPhase::Edit if compact => vec![Line::from(vec![
            Span::styled("Tab", theme::key_hint()),
            Span::raw(" 切字段 · ↑/↓ 调整 · Enter 预览 · Esc 返回"),
        ])],
        EditorPhase::Edit => vec![
            Line::from(vec![
                Span::styled("Tab", theme::key_hint()),
                Span::raw(" 切换字段  ·  "),
                Span::styled("↑/↓", theme::key_hint()),
                Span::raw(" 调整  ·  ←/→ 移动光标"),
            ]),
            Line::from(vec![
                Span::styled("Enter", theme::key_hint()),
                Span::raw(" 查看差异  ·  "),
                Span::styled("Esc", theme::key_hint()),
                Span::raw(" 返回并保留草稿"),
            ]),
        ],
        EditorPhase::Review => vec![Line::from(vec![
            Span::styled("Enter", theme::key_hint()),
            Span::raw(" 保存  ·  "),
            Span::styled("Esc", theme::key_hint()),
            Span::raw(" 回到编辑"),
        ])],
        EditorPhase::Saving => vec![Line::from(Span::styled(
            "请等待文件提交结果；如果失败，草稿会继续保留。",
            theme::muted(),
        ))],
        EditorPhase::Saved => vec![Line::from(vec![
            Span::styled("Enter / Esc", theme::key_hint()),
            Span::raw(" 返回"),
        ])],
    };
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(theme::border(false)),
        ),
        area,
    );
}

fn with_cursor(value: &str, cursor: usize) -> String {
    let cursor = cursor.min(value.len());
    let cursor = (0..=cursor)
        .rev()
        .find(|index| value.is_char_boundary(*index))
        .unwrap_or(0);
    const WINDOW: usize = 28;
    let cursor_char = value[..cursor].chars().count();
    let total_chars = value.chars().count();
    let start_char = cursor_char
        .saturating_sub(WINDOW / 2)
        .min(total_chars.saturating_sub(WINDOW));
    let end_char = (start_char + WINDOW).min(total_chars);
    let start_byte = value
        .char_indices()
        .nth(start_char)
        .map(|(index, _)| index)
        .unwrap_or(value.len());
    let end_byte = value
        .char_indices()
        .nth(end_char)
        .map(|(index, _)| index)
        .unwrap_or(value.len());
    let mut visible = value[start_byte..end_byte].to_owned();
    visible.insert(cursor.saturating_sub(start_byte).min(visible.len()), '▏');
    if start_char > 0 {
        visible.insert(0, '…');
    }
    if end_char < total_chars {
        visible.push('…');
    }
    visible
}

fn clipped(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let prefix = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{prefix}…")
    } else {
        prefix
    }
}
