use std::fmt::Write as _;

use unicode_width::UnicodeWidthChar;

use mc_core::TaskSnapshot;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use crate::theme;

pub(crate) fn plain_hint_line(hints: &[(&'static str, &'static str)]) -> Line<'static> {
    let mut spans = Vec::new();
    for (index, (key, label)) in hints.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled("  ·  ", theme::dim()));
        }
        spans.push(Span::styled(
            *key,
            theme::text().add_modifier(ratatui::style::Modifier::BOLD),
        ));
        spans.push(Span::styled(format!(" {label}"), theme::muted()));
    }
    Line::from(spans)
}

/// A compact, eighth-of-a-cell progress bar.
pub(crate) fn progress_bar(ratio: f64, width: usize, fill: Style) -> Vec<Span<'static>> {
    const PARTIAL: [&str; 9] = ["", "▏", "▎", "▍", "▌", "▋", "▊", "▉", "█"];
    let width = width.max(1);
    let eighths = (ratio.clamp(0.0, 1.0) * width as f64 * 8.0).round() as usize;
    let full = (eighths / 8).min(width);
    let remainder = eighths % 8;
    let mut spans = Vec::with_capacity(3);
    if full > 0 {
        spans.push(Span::styled("█".repeat(full), fill));
    }
    if remainder > 0 && full < width {
        spans.push(Span::styled(PARTIAL[remainder], fill));
    }
    let used = full + usize::from(remainder > 0 && full < width);
    if used < width {
        spans.push(Span::styled("░".repeat(width - used), theme::track()));
    }
    spans
}

pub(crate) fn task_ratio(task: &TaskSnapshot) -> f64 {
    if let Some(progress) = &task.progress {
        if let Some(transfer) = &progress.transfer {
            if transfer.total_bytes > 0 {
                return (transfer.downloaded_bytes as f64 / transfer.total_bytes as f64)
                    .clamp(0.0, 1.0);
            }
        }
        if progress.total_steps > 0 {
            return (progress.completed_steps as f64 / progress.total_steps as f64).clamp(0.0, 1.0);
        }
    }
    match task.status {
        mc_core::TaskStatus::Succeeded => 1.0,
        _ => 0.0,
    }
}

pub(crate) fn task_percent(task: &TaskSnapshot) -> u8 {
    (task_ratio(task) * 100.0).round() as u8
}

pub(crate) fn task_step_label(task: &TaskSnapshot) -> String {
    task.progress
        .as_ref()
        .map(|progress| format!("{}/{}", progress.completed_steps, progress.total_steps))
        .unwrap_or_else(|| "—".to_owned())
}

pub(crate) fn task_detail_label(task: &TaskSnapshot) -> String {
    if let Some(transfer) = task
        .progress
        .as_ref()
        .and_then(|progress| progress.transfer.as_ref())
    {
        let rate = transfer
            .bytes_per_second
            .map(format_rate)
            .unwrap_or_else(|| "测速中".to_owned());
        let eta = transfer
            .bytes_per_second
            .filter(|rate| *rate > 0)
            .map(|rate| {
                let remaining = transfer
                    .total_bytes
                    .saturating_sub(transfer.downloaded_bytes);
                format_eta(remaining, rate)
            })
            .unwrap_or_else(|| "—".to_owned());
        format!(
            "{} / {} · {} · 剩余 {}",
            format_bytes(transfer.downloaded_bytes),
            format_bytes(transfer.total_bytes),
            rate,
            eta
        )
    } else {
        task.message.clone()
    }
}

pub(crate) fn format_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * KIB;
    const GIB: u64 = 1024 * MIB;
    if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} B")
    }
}

pub(crate) fn format_rate(bytes: u64) -> String {
    format!("{}/s", format_bytes(bytes))
}

pub(crate) fn format_eta(remaining_bytes: u64, bytes_per_second: u64) -> String {
    if bytes_per_second == 0 {
        return "—".to_owned();
    }
    let seconds = remaining_bytes.div_ceil(bytes_per_second);
    if seconds >= 60 {
        format!("{}m{:02}s", seconds / 60, seconds % 60)
    } else {
        format!("{seconds}s")
    }
}

/// A one-column scrollbar drawn over the right border of a panel.
pub(crate) fn scrollbar(
    frame: &mut Frame<'_>,
    panel: Rect,
    content_length: usize,
    position: usize,
    viewport_content_length: usize,
) {
    if content_length <= viewport_content_length.max(1) || panel.width < 3 || panel.height < 4 {
        return;
    }
    let area = Rect::new(
        panel.x + panel.width - 1,
        panel.y + 1,
        1,
        panel.height.saturating_sub(2),
    );
    let mut state = ScrollbarState::new(content_length)
        .position(position.min(content_length.saturating_sub(1)))
        .viewport_content_length(viewport_content_length.max(1));
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"))
        .track_symbol(Some("│"))
        .thumb_symbol("█")
        .begin_style(theme::dim())
        .end_style(theme::dim())
        .track_style(theme::scrollbar_track())
        .thumb_style(theme::scrollbar_thumb());
    frame.render_stateful_widget(scrollbar, area, &mut state);
}

/// Render a small "n/m" window label for paged lists.
pub(crate) fn window_label(selected: usize, total: usize, visible: usize) -> String {
    if total == 0 {
        return "0/0".to_owned();
    }
    let visible = visible.max(1);
    let start = window_start(selected, total, visible);
    let end = (start + visible).min(total);
    format!("{}-{end}/{total}", start + 1)
}

pub(crate) fn window_start(selected: usize, total: usize, visible: usize) -> usize {
    if total <= visible {
        0
    } else {
        selected
            .saturating_add(1)
            .saturating_sub(visible)
            .min(total - visible)
    }
}

pub(crate) fn display_width(text: &str) -> usize {
    text.chars()
        .map(|character| character.width().unwrap_or(0))
        .sum()
}

pub(crate) fn truncate(text: &str, max_columns: usize) -> String {
    if max_columns == 0 {
        return String::new();
    }
    let mut output = String::new();
    let mut width = 0;
    for character in text.chars() {
        let character_width = character.width().unwrap_or(0);
        if width + character_width > max_columns {
            let _ = write!(output, "…");
            break;
        }
        output.push(character);
        width += character_width;
    }
    output
}

/// Trim a styled line without splitting wide characters; style survives per span.
pub(crate) fn clamp_line(line: Line<'static>, max_columns: usize) -> Line<'static> {
    let mut spans = Vec::new();
    let mut used = 0;
    for span in line.spans {
        if used >= max_columns {
            break;
        }
        let remaining = max_columns - used;
        let text = truncate(span.content.as_ref(), remaining);
        used += display_width(&text);
        spans.push(Span::styled(text, span.style));
    }
    Line::from(spans)
}
