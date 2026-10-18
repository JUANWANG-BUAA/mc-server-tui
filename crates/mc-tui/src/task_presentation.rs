use ratatui::style::Style;

use mc_core::{TaskKind, TaskPhase, TaskStatus};

use crate::theme;

pub(crate) fn kind_label(kind: TaskKind) -> &'static str {
    match kind {
        TaskKind::CreatePaperInstance => "创建 Paper 实例",
        TaskKind::CreateWorldBackup => "备份世界",
        TaskKind::ListWorldBackups => "列出世界备份",
        TaskKind::VerifyWorldBackup => "校验世界备份",
        TaskKind::PlanWorldBackupRestore => "预览备份恢复",
        TaskKind::RestoreWorldBackup => "恢复世界备份",
    }
}

pub(crate) fn status_label(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Queued => "排队中",
        TaskStatus::Running => "进行中",
        TaskStatus::Succeeded => "已完成",
        TaskStatus::Failed => "失败",
        TaskStatus::Interrupted => "已中断",
        TaskStatus::Cancelled => "已取消",
    }
}

pub(crate) fn phase_label(phase: TaskPhase) -> &'static str {
    match phase {
        TaskPhase::Queued => "等待执行",
        TaskPhase::WaitingForInstance => "等待实例锁",
        TaskPhase::ResolvingArtifact => "解析版本",
        TaskPhase::DownloadingArtifact => "下载校验",
        TaskPhase::InstallingInstance => "安装实例",
        TaskPhase::BackingUpWorld => "备份世界",
        TaskPhase::ListingBackups => "读取备份列表",
        TaskPhase::VerifyingBackup => "校验备份",
        TaskPhase::PlanningBackupRestore => "生成恢复计划",
        TaskPhase::RestoringWorldBackup => "恢复世界数据",
        TaskPhase::Completed => "完成",
        TaskPhase::Failed => "失败",
        TaskPhase::Interrupted => "中断",
        TaskPhase::Cancelled => "已取消",
    }
}

pub(crate) fn status_style(status: TaskStatus) -> Style {
    match status {
        TaskStatus::Queued => theme::warning(),
        TaskStatus::Running => theme::accent(),
        TaskStatus::Succeeded => theme::success(),
        TaskStatus::Failed | TaskStatus::Interrupted => theme::danger(),
        TaskStatus::Cancelled => theme::muted(),
    }
}

pub(crate) fn activity_symbol(status: TaskStatus, frame: u8) -> &'static str {
    match status {
        TaskStatus::Queued => "◌",
        TaskStatus::Running => theme::spinner(frame),
        TaskStatus::Succeeded => "✓",
        TaskStatus::Failed => "×",
        TaskStatus::Interrupted => "!",
        TaskStatus::Cancelled => "−",
    }
}
