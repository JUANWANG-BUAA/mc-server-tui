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

pub(crate) fn kind_short(kind: TaskKind) -> &'static str {
    match kind {
        TaskKind::CreatePaperInstance => "建服",
        TaskKind::CreateWorldBackup => "备份",
        TaskKind::ListWorldBackups => "列表",
        TaskKind::VerifyWorldBackup => "校验",
        TaskKind::PlanWorldBackupRestore => "恢复预览",
        TaskKind::RestoreWorldBackup => "恢复",
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

pub(crate) fn is_cancelable_download(task: &mc_core::TaskSnapshot) -> bool {
    if task.kind != TaskKind::CreatePaperInstance {
        return false;
    }
    match task.status {
        TaskStatus::Queued => true,
        TaskStatus::Running => {
            task.phase == TaskPhase::DownloadingArtifact
                || task
                    .progress
                    .as_ref()
                    .is_some_and(|progress| progress.transfer.is_some())
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_download_or_queued_create_tasks_are_cancelable() {
        let mut task = mc_core::TaskSnapshot {
            id: uuid::Uuid::new_v4(),
            kind: TaskKind::CreatePaperInstance,
            instance_id: None,
            status: TaskStatus::Running,
            phase: TaskPhase::DownloadingArtifact,
            message: String::new(),
            progress: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            finished_at: None,
            result: None,
            error: None,
            last_event_sequence: 0,
        };
        assert!(is_cancelable_download(&task));
        task.phase = TaskPhase::InstallingInstance;
        assert!(!is_cancelable_download(&task));
        task.status = TaskStatus::Queued;
        assert!(is_cancelable_download(&task));
        task.status = TaskStatus::Succeeded;
        assert!(!is_cancelable_download(&task));
    }
}
