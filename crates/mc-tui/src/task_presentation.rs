use ratatui::style::{Color, Style};

use mc_core::{TaskKind, TaskPhase, TaskStatus};

pub(crate) fn kind_label(kind: TaskKind) -> &'static str {
    match kind {
        TaskKind::CreatePaperInstance => "创建 Paper 实例",
    }
}

pub(crate) fn status_label(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Queued => "排队中",
        TaskStatus::Running => "进行中",
        TaskStatus::Succeeded => "已完成",
        TaskStatus::Failed => "失败",
        TaskStatus::Interrupted => "已中断",
    }
}

pub(crate) fn phase_label(phase: TaskPhase) -> &'static str {
    match phase {
        TaskPhase::Queued => "等待执行",
        TaskPhase::ResolvingArtifact => "解析版本",
        TaskPhase::DownloadingArtifact => "下载校验",
        TaskPhase::InstallingInstance => "安装实例",
        TaskPhase::Completed => "完成",
        TaskPhase::Failed => "失败",
        TaskPhase::Interrupted => "中断",
    }
}

pub(crate) fn status_style(status: TaskStatus) -> Style {
    match status {
        TaskStatus::Queued | TaskStatus::Running => Style::default().fg(Color::Yellow),
        TaskStatus::Succeeded => Style::default().fg(Color::Green),
        TaskStatus::Failed | TaskStatus::Interrupted => Style::default().fg(Color::Red),
    }
}
