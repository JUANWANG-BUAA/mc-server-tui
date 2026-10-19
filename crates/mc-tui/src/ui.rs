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

/// Moving highlights indicate activity without inventing download progress.
pub(crate) fn animated_progress(
    ratio: f64,
    width: usize,
    frame: u8,
    active: bool,
) -> Vec<Span<'static>> {
    if !active || ratio >= 1.0 {
        return progress_bar(ratio, width, theme::progress());
    }
    let filled = (ratio.clamp(0.0, 1.0) * width as f64).floor() as usize;
    let phase = usize::from(frame) % width.max(1);
    (0..width)
        .map(|i| {
            if i < filled {
                Span::styled(
                    if i == phase { "▓" } else { "█" },
                    if i == phase {
                        theme::success()
                    } else {
                        theme::progress()
                    },
                )
            } else if i == filled || (filled == 0 && i == phase) {
                Span::styled(
                    ["▏", "▎", "▍", "▌", "▋", "▊", "▉", "█"][usize::from(frame) % 8],
                    theme::accent(),
                )
            } else {
                Span::styled("░", theme::track())
            }
        })
        .collect()
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
        .position(position.min(content_length.saturating_sub(viewport_content_length.max(1))))
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
    if display_width(text) <= max_columns {
        return text.to_owned();
    }
    let mut out = String::new();
    let mut width = 0;
    for c in text.chars() {
        let size = c.width().unwrap_or(0);
        if width + size > max_columns - 1 {
            break;
        }
        out.push(c);
        width += size;
    }
    out.push('…');
    out
}

pub(crate) fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let width = width.max(2);
    let text = clean_terminal_text(text);
    let mut lines = Vec::new();
    let mut line = String::new();
    let mut used = 0;
    for c in text.chars() {
        let size = c.width().unwrap_or(0);
        if used + size > width {
            lines.push(std::mem::take(&mut line));
            used = 0;
        }
        line.push(c);
        used += size;
    }
    lines.push(line);
    lines
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

/// Semantic coloring is applied after removing terminal control sequences.
pub(crate) fn clean_terminal_text(input: &str) -> String {
    let mut out = String::new();
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            match chars.next() {
                Some('[') => {
                    for c in chars.by_ref() {
                        if ('@'..='~').contains(&c) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    while let Some(c) = chars.next() {
                        if c == '\x07' {
                            break;
                        }
                        if c == '\x1b' && chars.peek() == Some(&'\\') {
                            chars.next();
                            break;
                        }
                    }
                }
                _ => {}
            }
        } else if c == '\t' {
            out.push_str("    ");
        } else if !c.is_control() {
            out.push(c);
        }
    }
    out
}
pub(crate) fn log_line(input: &str, query: &str) -> Line<'static> {
    let text = clean_terminal_text(input);
    let upper = text.to_ascii_uppercase();
    let style = if upper.contains("ERROR") || upper.contains("FATAL") || upper.contains("EXCEPTION")
    {
        theme::danger()
    } else if upper.contains("WARN") {
        theme::warning()
    } else if text.contains("Done (") || text.contains("joined the game") || text.contains("Saved")
    {
        theme::success()
    } else if upper.contains("CHAT") || text.contains("<") {
        ratatui::style::Style::default().fg(ratatui::style::Color::Cyan)
    } else {
        theme::text()
    };
    let query = clean_terminal_text(query);
    if query.is_empty() {
        let mut spans = Vec::new();
        let mut offset = 0;
        if text.starts_with('[') {
            if let Some(end) = text.find(']').filter(|end| *end < 64) {
                let level = if text[..=end].contains("INFO") {
                    ratatui::style::Style::default().fg(ratatui::style::Color::Cyan)
                } else {
                    theme::muted()
                };
                spans.push(Span::styled(text[..=end].to_owned(), level));
                offset = end + 1;
                let rest = &text[offset..];
                if let Some(begin) = rest.find('[').filter(|begin| *begin <= 2) {
                    if let Some(end) = rest.find(']').filter(|end| *end < 80) {
                        let tag = &rest[begin..=end];
                        let level = if tag.contains("ERROR") {
                            theme::danger()
                        } else if tag.contains("WARN") {
                            theme::warning()
                        } else if tag.contains("INFO") {
                            ratatui::style::Style::default().fg(ratatui::style::Color::Cyan)
                        } else {
                            theme::muted()
                        };
                        spans.push(Span::styled(rest[..=end].to_owned(), level));
                        offset += end + 1;
                    }
                }
            }
        }
        spans.push(Span::styled(text[offset..].to_owned(), style));
        return Line::from(spans);
    }
    // Case-sensitive byte matching keeps Unicode boundaries intact. Filtering
    // is case-insensitive; ASCII case-folding is used only when lengths agree.
    let haystack = text.to_ascii_lowercase();
    let needle = query.to_ascii_lowercase();
    let mut spans = Vec::new();
    let mut from = 0;
    for (start, _) in haystack.match_indices(&needle) {
        if start < from {
            continue;
        }
        let end = start + needle.len();
        spans.push(Span::styled(text[from..start].to_owned(), style));
        spans.push(Span::styled(
            text[start..end].to_owned(),
            ratatui::style::Style::default()
                .fg(ratatui::style::Color::Black)
                .bg(ratatui::style::Color::Yellow)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ));
        from = end;
    }
    spans.push(Span::styled(text[from..].to_owned(), style));
    Line::from(spans)
}
/// Single source of truth for the short server-state label used by the
/// instance detail, session dock, and status transcript.
pub(crate) fn server_state_label(state: mc_core::ServerState) -> &'static str {
    match state {
        mc_core::ServerState::Installed => "已安装",
        mc_core::ServerState::Starting => "启动中",
        mc_core::ServerState::Running => "运行中",
        mc_core::ServerState::Stopping => "停止中",
        mc_core::ServerState::Stopped => "已停止",
        mc_core::ServerState::Failed => "启动失败",
        mc_core::ServerState::Unknown => "状态未知",
    }
}

pub(crate) fn server_state_style(state: mc_core::ServerState) -> Style {
    match state {
        mc_core::ServerState::Running => {
            theme::success().add_modifier(ratatui::style::Modifier::BOLD)
        }
        mc_core::ServerState::Starting | mc_core::ServerState::Stopping => theme::warning(),
        mc_core::ServerState::Failed | mc_core::ServerState::Unknown => theme::danger(),
        mc_core::ServerState::Installed | mc_core::ServerState::Stopped => theme::muted(),
    }
}

/// Chinese text for the supervisor's fixed server status messages. Anything
/// else, such as error evidence, is shown verbatim rather than guessed at.
pub(crate) fn server_message(message: &str) -> String {
    let translated = match message {
        "Installed and stopped." => "已安装，尚未启动。",
        "Waiting for the server readiness message." => "Java 已启动，正在等待服务器就绪…",
        "Server is ready." => "服务器已就绪。",
        "Graceful stop requested." => "已请求优雅停服，正在保存世界…",
        "Server stopped successfully." => "服务器已正常停止。",
        "Server stopped." => "服务器已停止。",
        "Server exited with an error." => "服务器异常退出；请查看日志。",
        "Server failed to start; inspect the instance logs." => "服务器启动失败；请查看实例日志。",
        "Server process is no longer running." => "服务器进程已不在运行。",
        "This instance has not been started by the supervisor." => "此实例尚未由本工具启动。",
        "The configured port is occupied; process ownership is unknown." => {
            "配置的端口已被其他进程占用。"
        }
        "The server did not exit before the graceful stop timeout." => "服务器未在超时前退出。",
        _ => return message.to_owned(),
    };
    translated.to_owned()
}

/// Short, recognizable label for a Java executable: the full path when short,
/// otherwise the distribution directory (`openjdk@21`, `temurin-21.jdk`).
pub(crate) fn java_label(path: &std::path::Path) -> String {
    let full = path.display().to_string();
    if display_width(&full) <= 28 {
        return full;
    }
    path.iter()
        .rev()
        .filter_map(|part| part.to_str())
        .find(|part| !matches!(*part, "java" | "bin" | "Home" | "Contents" | "jre" | "/"))
        .map(str::to_owned)
        .unwrap_or(full)
}

pub(crate) fn history_plot(
    values: &std::collections::VecDeque<u64>,
    width: usize,
    ceiling: u64,
) -> String {
    const BARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let max = ceiling
        .max(values.iter().copied().max().unwrap_or(1))
        .max(1);
    values
        .iter()
        .skip(values.len().saturating_sub(width))
        .map(|v| BARS[((v.saturating_mul(7) / max) as usize).min(7)])
        .collect()
}

/// Scrollbar coordinates use a viewport's top offset, never distance from tail.
pub(crate) fn scrollbar_hits(
    panel: Rect,
    total: usize,
    viewport: usize,
    hits: &mut crate::hit::HitMap,
    view: crate::hit::ScrollView,
) {
    if total <= viewport.max(1) || panel.width < 3 || panel.height < 4 {
        return;
    }
    let max = total.saturating_sub(viewport.max(1));
    let height = panel.height.saturating_sub(2);
    for row in 0..height {
        let position = (usize::from(row) * max) / usize::from(height.saturating_sub(1).max(1));
        hits.push(
            Rect::new(panel.right() - 1, panel.y + 1 + row, 1, 1),
            crate::hit::HitTarget::Scrollbar(view, position, max, viewport),
        );
    }
}

#[cfg(test)]
mod display_tests {
    use super::*;
    #[test]
    fn unsafe_escape_sequences_are_removed_but_log_text_survives() {
        assert_eq!(
            clean_terminal_text("\x1b[31mERROR\x1b[0m\x1b]52;c;secret\x07 hello"),
            "ERROR hello"
        );
    }
    #[test]
    fn animation_changes_without_claiming_extra_bytes() {
        let a = animated_progress(0.4, 12, 1, true);
        let b = animated_progress(0.4, 12, 4, true);
        assert_ne!(a, b);
        assert_eq!(
            a.iter().map(|s| display_width(&s.content)).sum::<usize>(),
            12
        );
        assert_eq!(
            animated_progress(1.0, 12, 1, true),
            animated_progress(1.0, 12, 4, true)
        );
    }
    #[test]
    fn java_label_keeps_the_distribution_name() {
        use std::path::Path;
        assert_eq!(java_label(Path::new("/usr/bin/java")), "/usr/bin/java");
        assert_eq!(
            java_label(Path::new("/opt/homebrew/opt/openjdk@21/bin/java")),
            "openjdk@21"
        );
        assert_eq!(
            java_label(Path::new(
                "/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java"
            )),
            "temurin-21.jdk"
        );
    }
    #[test]
    fn unicode_highlight_is_safe() {
        let line = log_line("[INFO] 玩家 Steve joined the game", "玩家");
        assert!(line.spans.iter().any(|s| s.content == "玩家"));
    }
}
