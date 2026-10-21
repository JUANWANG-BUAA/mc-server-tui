//! First-use actions shared by the session and classic dashboard.
use crate::{
    app::AppState,
    hit::{HitMap, HitTarget},
    theme, ui,
};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub(crate) fn render(frame: &mut Frame<'_>, area: Rect, app: &AppState, hits: &mut HitMap) {
    let content = if area.width >= 80 && area.height >= 10 {
        let columns = Layout::horizontal([Constraint::Length(11), Constraint::Min(1)]).split(area);
        render_mascot(frame, columns[0], 2);
        columns[1]
    } else {
        area
    };
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(1),
    ])
    .split(content);
    frame.render_widget(
        Paragraph::new("还没有服务器 · 从这里开始")
            .style(theme::text().add_modifier(Modifier::BOLD)),
        rows[0],
    );
    let actions = [
        (
            if app.shell_visible { "/create" } else { "n" },
            "创建 Paper 服务器",
            HitTarget::WelcomeCreatePaper,
        ),
        (
            if app.shell_visible { "/fabric" } else { "f" },
            "创建 Fabric · 填写名称和版本",
            HitTarget::WelcomeCreateFabric,
        ),
        (
            if app.shell_visible { "/import" } else { "i" },
            "检查 ZIP / MRPACK 整合包",
            HitTarget::WelcomeImportModpack,
        ),
    ];
    for (index, (key, label, target)) in actions.into_iter().enumerate() {
        frame.render_widget(
            Paragraph::new(ui::clamp_line(
                Line::from(vec![
                    Span::styled(format!("{key:<8} "), theme::accent()),
                    Span::styled(label, theme::text()),
                ]),
                content.width as usize,
            )),
            rows[index + 2],
        );
        hits.push(rows[index + 2], target);
    }
    frame.render_widget(
        Paragraph::new(if app.shell_visible {
            "输入命令后 Enter；也可点击入口。F1 帮助"
        } else {
            "n / f / i 选择入口 · t 会话 · ? 帮助 · q 退出"
        })
        .style(theme::muted()),
        rows[6],
    );
}

fn render_mascot(frame: &mut Frame<'_>, area: Rect, animation_frame: u8) {
    // Steve's 8×8 face, two pixel rows per terminal cell; half blocks preserve
    // the square silhouette, brown hair/beard, blue eyes and broad nose.
    const PIXELS: [&str; 8] = [
        "hhhhhhhh", "hhhhhhhh", "hssssssh", "ssssssss", "swessews", "ssnnnnss", "ssdbbdss",
        "sssbbsss",
    ];
    let color = |pixel| match pixel {
        'h' => Color::Rgb(54, 37, 25),
        's' | ' ' => Color::Rgb(181, 136, 108),
        'w' => Color::Rgb(237, 234, 225),
        'e' => Color::Rgb(75, 65, 145),
        'n' => Color::Rgb(156, 105, 78),
        'd' => Color::Rgb(106, 69, 46),
        _ => Color::Rgb(67, 43, 29),
    };
    let mut lines = vec![Line::from(Span::styled("STEVE", theme::muted()))];
    for pair in 0..4 {
        let top: Vec<_> = PIXELS[pair * 2].chars().take(8).collect();
        let bottom: Vec<_> = PIXELS[pair * 2 + 1].chars().take(8).collect();
        let spans = (0..8)
            .map(|x| {
                let upper = if pair == 2 && (x == 2 || x == 5) && animation_frame % 40 < 2 {
                    's'
                } else {
                    top[x]
                };
                Span::styled("▀", Style::default().fg(color(upper)).bg(color(bottom[x])))
            })
            .collect::<Vec<_>>();
        lines.push(Line::from(spans));
    }
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), area);
}
