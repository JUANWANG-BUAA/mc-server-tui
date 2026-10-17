use ratatui::style::{Color, Modifier, Style};

const BACKGROUND: Color = Color::Rgb(12, 18, 28);
const SURFACE: Color = Color::Rgb(19, 30, 45);
const BORDER: Color = Color::Rgb(54, 70, 91);
const TEXT: Color = Color::Rgb(226, 232, 240);
const MUTED: Color = Color::Rgb(148, 163, 184);
const ACCENT: Color = Color::Rgb(56, 189, 248);
const TEAL: Color = Color::Rgb(45, 212, 191);
const GREEN: Color = Color::Rgb(74, 222, 128);
const YELLOW: Color = Color::Rgb(251, 191, 36);
const RED: Color = Color::Rgb(251, 113, 133);

pub(crate) fn page() -> Style {
    Style::default().fg(TEXT).bg(BACKGROUND)
}

pub(crate) fn surface() -> Style {
    Style::default().bg(SURFACE)
}

pub(crate) fn border(focused: bool) -> Style {
    Style::default().fg(if focused { ACCENT } else { BORDER })
}

pub(crate) fn title(focused: bool) -> Style {
    Style::default()
        .fg(if focused { ACCENT } else { MUTED })
        .add_modifier(Modifier::BOLD)
}

pub(crate) fn brand() -> Style {
    Style::default().fg(TEAL).add_modifier(Modifier::BOLD)
}

pub(crate) fn accent() -> Style {
    Style::default().fg(ACCENT)
}

pub(crate) fn muted() -> Style {
    Style::default().fg(MUTED)
}

pub(crate) fn success() -> Style {
    Style::default().fg(GREEN)
}

pub(crate) fn warning() -> Style {
    Style::default().fg(YELLOW)
}

pub(crate) fn danger() -> Style {
    Style::default().fg(RED)
}

pub(crate) fn selected_row() -> Style {
    Style::default()
        .fg(TEXT)
        .bg(Color::Rgb(23, 58, 75))
        .add_modifier(Modifier::BOLD)
}

pub(crate) fn key_hint() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

pub(crate) fn spinner(frame: u8) -> &'static str {
    ["◐", "◓", "◑", "◒"][usize::from(frame) % 4]
}

pub(crate) fn server_marker(state: mc_core::ServerState, frame: u8) -> &'static str {
    match state {
        mc_core::ServerState::Starting | mc_core::ServerState::Stopping => spinner(frame),
        mc_core::ServerState::Running => "●",
        mc_core::ServerState::Installed | mc_core::ServerState::Stopped => "○",
        mc_core::ServerState::Failed => "×",
        mc_core::ServerState::Unknown => "?",
    }
}
