use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU8, Ordering},
};

use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ThemeId {
    Claude,
    DeepSeek,
    Codex,
    Kimi,
    Pcl2,
    Mono,
    Ocean,
}

impl ThemeId {
    pub(crate) const ALL: [ThemeId; 7] = [
        ThemeId::Claude,
        ThemeId::DeepSeek,
        ThemeId::Codex,
        ThemeId::Kimi,
        ThemeId::Pcl2,
        ThemeId::Mono,
        ThemeId::Ocean,
    ];

    pub(crate) fn next(self) -> Self {
        match self {
            ThemeId::Claude => ThemeId::DeepSeek,
            ThemeId::DeepSeek => ThemeId::Codex,
            ThemeId::Codex => ThemeId::Kimi,
            ThemeId::Kimi => ThemeId::Pcl2,
            ThemeId::Pcl2 => ThemeId::Mono,
            ThemeId::Mono => ThemeId::Ocean,
            ThemeId::Ocean => ThemeId::Claude,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            ThemeId::Claude => "Claude",
            ThemeId::DeepSeek => "DeepSeek",
            ThemeId::Codex => "Codex",
            ThemeId::Kimi => "Kimi",
            ThemeId::Pcl2 => "PCL2",
            ThemeId::Mono => "Mono",
            ThemeId::Ocean => "Ocean",
        }
    }

    pub(crate) fn slug(self) -> &'static str {
        match self {
            ThemeId::Claude => "claude",
            ThemeId::DeepSeek => "deepseek",
            ThemeId::Codex => "codex",
            ThemeId::Kimi => "kimi",
            ThemeId::Pcl2 => "pcl2",
            ThemeId::Mono => "mono",
            ThemeId::Ocean => "ocean",
        }
    }

    pub(crate) fn from_slug(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "claude" | "warm" | "default" => Some(ThemeId::Claude),
            "deepseek" | "ds" | "blue" => Some(ThemeId::DeepSeek),
            "codex" | "openai" | "green" => Some(ThemeId::Codex),
            "kimi" | "moonshot" | "violet" => Some(ThemeId::Kimi),
            "pcl2" | "pcl" | "minecraft" | "grass" => Some(ThemeId::Pcl2),
            "mono" | "black" | "gray" | "grey" => Some(ThemeId::Mono),
            "ocean" | "teal" | "cyan" => Some(ThemeId::Ocean),
            _ => None,
        }
    }

    fn from_u8(value: u8) -> Self {
        match value {
            1 => ThemeId::DeepSeek,
            2 => ThemeId::Codex,
            3 => ThemeId::Kimi,
            4 => ThemeId::Pcl2,
            5 => ThemeId::Mono,
            6 => ThemeId::Ocean,
            _ => ThemeId::Claude,
        }
    }

    fn as_u8(self) -> u8 {
        match self {
            ThemeId::Claude => 0,
            ThemeId::DeepSeek => 1,
            ThemeId::Codex => 2,
            ThemeId::Kimi => 3,
            ThemeId::Pcl2 => 4,
            ThemeId::Mono => 5,
            ThemeId::Ocean => 6,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Palette {
    background: Color,
    surface: Color,
    elevated: Color,
    border: Color,
    text: Color,
    muted: Color,
    dim: Color,
    accent: Color,
    success: Color,
    warning: Color,
    danger: Color,
    selected_bg: Color,
    keycap_bg: Color,
    track: Color,
}

const CLAUDE: Palette = Palette {
    background: Color::Rgb(10, 10, 10),
    surface: Color::Rgb(16, 16, 16),
    elevated: Color::Rgb(24, 24, 24),
    border: Color::Rgb(43, 43, 43),
    text: Color::Rgb(232, 232, 232),
    muted: Color::Rgb(154, 154, 154),
    dim: Color::Rgb(125, 125, 125),
    accent: Color::Rgb(217, 119, 87),
    success: Color::Rgb(127, 166, 80),
    warning: Color::Rgb(214, 168, 75),
    danger: Color::Rgb(217, 106, 106),
    selected_bg: Color::Rgb(32, 27, 24),
    keycap_bg: Color::Rgb(26, 26, 26),
    track: Color::Rgb(38, 38, 38),
};

const MONO: Palette = Palette {
    background: Color::Rgb(0, 0, 0),
    surface: Color::Rgb(10, 10, 10),
    elevated: Color::Rgb(20, 20, 20),
    border: Color::Rgb(46, 46, 46),
    text: Color::Rgb(242, 242, 242),
    muted: Color::Rgb(160, 160, 160),
    dim: Color::Rgb(126, 126, 126),
    accent: Color::Rgb(216, 216, 216),
    success: Color::Rgb(184, 184, 184),
    warning: Color::Rgb(208, 208, 208),
    danger: Color::Rgb(224, 108, 117),
    selected_bg: Color::Rgb(31, 31, 31),
    keycap_bg: Color::Rgb(26, 26, 26),
    track: Color::Rgb(38, 38, 38),
};

const DEEPSEEK: Palette = Palette {
    background: Color::Rgb(8, 13, 26),
    surface: Color::Rgb(14, 21, 38),
    elevated: Color::Rgb(20, 30, 51),
    border: Color::Rgb(38, 50, 74),
    text: Color::Rgb(230, 236, 255),
    muted: Color::Rgb(138, 152, 189),
    dim: Color::Rgb(121, 134, 166),
    accent: Color::Rgb(77, 107, 254),
    success: Color::Rgb(91, 214, 162),
    warning: Color::Rgb(232, 196, 106),
    danger: Color::Rgb(242, 124, 138),
    selected_bg: Color::Rgb(22, 34, 74),
    keycap_bg: Color::Rgb(18, 27, 48),
    track: Color::Rgb(38, 52, 92),
};

const CODEX: Palette = Palette {
    background: Color::Rgb(10, 10, 10),
    surface: Color::Rgb(17, 17, 17),
    elevated: Color::Rgb(24, 24, 24),
    border: Color::Rgb(40, 46, 43),
    text: Color::Rgb(235, 241, 238),
    muted: Color::Rgb(152, 165, 159),
    dim: Color::Rgb(124, 137, 131),
    accent: Color::Rgb(16, 163, 127),
    success: Color::Rgb(123, 201, 111),
    warning: Color::Rgb(226, 179, 65),
    danger: Color::Rgb(229, 115, 115),
    selected_bg: Color::Rgb(20, 37, 31),
    keycap_bg: Color::Rgb(26, 26, 26),
    track: Color::Rgb(38, 48, 41),
};

const KIMI: Palette = Palette {
    background: Color::Rgb(9, 7, 17),
    surface: Color::Rgb(17, 14, 29),
    elevated: Color::Rgb(25, 20, 40),
    border: Color::Rgb(50, 42, 74),
    text: Color::Rgb(236, 232, 255),
    muted: Color::Rgb(161, 151, 194),
    dim: Color::Rgb(141, 131, 174),
    accent: Color::Rgb(167, 139, 250),
    success: Color::Rgb(123, 216, 143),
    warning: Color::Rgb(230, 193, 90),
    danger: Color::Rgb(240, 124, 147),
    selected_bg: Color::Rgb(36, 26, 60),
    keycap_bg: Color::Rgb(26, 21, 40),
    track: Color::Rgb(46, 39, 69),
};

const PCL2: Palette = Palette {
    background: Color::Rgb(8, 17, 26),
    surface: Color::Rgb(14, 27, 39),
    elevated: Color::Rgb(21, 39, 54),
    border: Color::Rgb(39, 65, 79),
    text: Color::Rgb(232, 244, 247),
    muted: Color::Rgb(147, 174, 184),
    dim: Color::Rgb(125, 152, 163),
    accent: Color::Rgb(86, 184, 232),
    success: Color::Rgb(123, 216, 143),
    warning: Color::Rgb(242, 201, 76),
    danger: Color::Rgb(240, 124, 124),
    selected_bg: Color::Rgb(19, 48, 67),
    keycap_bg: Color::Rgb(16, 36, 48),
    track: Color::Rgb(34, 64, 79),
};

const OCEAN: Palette = Palette {
    background: Color::Rgb(6, 9, 13),
    surface: Color::Rgb(12, 18, 24),
    elevated: Color::Rgb(18, 26, 34),
    border: Color::Rgb(35, 48, 59),
    text: Color::Rgb(224, 230, 236),
    muted: Color::Rgb(143, 161, 176),
    dim: Color::Rgb(115, 132, 145),
    accent: Color::Rgb(108, 182, 201),
    success: Color::Rgb(127, 184, 138),
    warning: Color::Rgb(211, 178, 106),
    danger: Color::Rgb(217, 124, 124),
    selected_bg: Color::Rgb(19, 33, 42),
    keycap_bg: Color::Rgb(20, 28, 36),
    track: Color::Rgb(30, 42, 51),
};

static THEME: AtomicU8 = AtomicU8::new(0);

pub(crate) fn current_theme() -> ThemeId {
    ThemeId::from_u8(THEME.load(Ordering::Relaxed))
}

pub(crate) fn set_theme(theme: ThemeId) {
    THEME.store(theme.as_u8(), Ordering::Relaxed);
    save_preference(theme);
}

pub(crate) fn cycle_theme() -> ThemeId {
    let next = current_theme().next();
    set_theme(next);
    next
}

pub(crate) fn init() {
    if let Ok(value) = std::env::var("MC_SERVER_TUI_THEME") {
        if let Some(theme) = ThemeId::from_slug(&value) {
            THEME.store(theme.as_u8(), Ordering::Relaxed);
            return;
        }
    }
    if let Some(path) = preference_path() {
        if let Ok(value) = fs::read_to_string(path) {
            if let Some(theme) = ThemeId::from_slug(&value) {
                THEME.store(theme.as_u8(), Ordering::Relaxed);
            }
        }
    }
}

fn preference_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("org", "mc-server-tui", "mc-server-tui")
        .map(|directories| directories.config_dir().join("theme"))
}

fn save_preference(theme: ThemeId) {
    let Some(path) = preference_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        if fs::create_dir_all(parent).is_err() {
            return;
        }
    }
    let _ = fs::write(path, theme.slug());
}

fn palette() -> Palette {
    match current_theme() {
        ThemeId::Claude => CLAUDE,
        ThemeId::DeepSeek => DEEPSEEK,
        ThemeId::Codex => CODEX,
        ThemeId::Kimi => KIMI,
        ThemeId::Pcl2 => PCL2,
        ThemeId::Mono => MONO,
        ThemeId::Ocean => OCEAN,
    }
}

pub(crate) fn page() -> Style {
    let palette = palette();
    Style::default().fg(palette.text).bg(palette.background)
}

pub(crate) fn surface() -> Style {
    let palette = palette();
    Style::default().fg(palette.text).bg(palette.surface)
}

pub(crate) fn elevated() -> Style {
    let palette = palette();
    Style::default().fg(palette.text).bg(palette.elevated)
}

pub(crate) fn border(focused: bool) -> Style {
    let palette = palette();
    Style::default().fg(if focused {
        palette.accent
    } else {
        palette.border
    })
}

pub(crate) fn border_type() -> BorderType {
    BorderType::Rounded
}

pub(crate) fn title(focused: bool) -> Style {
    let palette = palette();
    Style::default()
        .fg(if focused {
            palette.accent
        } else {
            palette.muted
        })
        .add_modifier(Modifier::BOLD)
}

pub(crate) fn brand() -> Style {
    let palette = palette();
    Style::default()
        .fg(palette.accent)
        .add_modifier(Modifier::BOLD)
}

pub(crate) fn accent() -> Style {
    Style::default().fg(palette().accent)
}

pub(crate) fn muted() -> Style {
    Style::default().fg(palette().muted)
}

pub(crate) fn dim() -> Style {
    Style::default().fg(palette().dim)
}

pub(crate) fn success() -> Style {
    Style::default().fg(palette().success)
}

pub(crate) fn warning() -> Style {
    Style::default().fg(palette().warning)
}

pub(crate) fn danger() -> Style {
    Style::default().fg(palette().danger)
}

pub(crate) fn text() -> Style {
    Style::default().fg(palette().text)
}

pub(crate) fn selected_row() -> Style {
    let palette = palette();
    Style::default()
        .bg(palette.selected_bg)
        .add_modifier(Modifier::BOLD)
}

pub(crate) fn selected_chip() -> Style {
    let palette = palette();
    Style::default()
        .fg(palette.background)
        .bg(palette.accent)
        .add_modifier(Modifier::BOLD)
}

/// Dark keycaps keep the footer quiet; the accent is reserved for focus.
pub(crate) fn keycap() -> Style {
    let palette = palette();
    Style::default()
        .fg(palette.text)
        .bg(palette.keycap_bg)
        .add_modifier(Modifier::BOLD)
}

pub(crate) fn keycap_alt() -> Style {
    let palette = palette();
    Style::default().fg(palette.muted).bg(palette.keycap_bg)
}

pub(crate) fn key_hint() -> Style {
    Style::default()
        .fg(palette().accent)
        .add_modifier(Modifier::BOLD)
}

pub(crate) fn progress() -> Style {
    Style::default().fg(palette().accent)
}

pub(crate) fn track() -> Style {
    Style::default().fg(palette().track)
}

pub(crate) fn scrollbar_thumb() -> Style {
    Style::default().fg(palette().accent)
}

pub(crate) fn scrollbar_track() -> Style {
    Style::default().fg(palette().track)
}

pub(crate) fn panel(panel_title: impl Into<String>, focused: bool) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(border_type())
        .border_style(border(focused))
        .title(panel_title.into())
        .title_style(title(focused))
        .style(surface())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_names_are_stable_and_parsable() {
        assert_eq!(ThemeId::Claude.next(), ThemeId::DeepSeek);
        assert_eq!(ThemeId::Ocean.next(), ThemeId::Claude);
        for theme in ThemeId::ALL {
            assert_eq!(ThemeId::from_slug(theme.slug()), Some(theme));
            assert!(!theme.name().is_empty());
        }
        assert_eq!(ThemeId::from_slug("unknown"), None);
    }
}
