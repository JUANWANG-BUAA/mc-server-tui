use chrono::Utc;
use mc_core::{TaskEvent, TaskKind, TaskPhase, TaskProgress, TaskSnapshot, TaskStatus};
use ratatui::{backend::TestBackend, Terminal};

use crate::hit::HitMap;
use uuid::Uuid;

use crate::task_detail::TaskDetailState;

use super::render;

#[test]
fn renders_task_progress_and_event_cursor_at_reference_and_compact_sizes() {
    let mut detail = TaskDetailState::new(task_snapshot());
    detail.events.push(TaskEvent {
        sequence: 1,
        task_id: detail.task.id,
        occurred_at: Utc::now(),
        status: TaskStatus::Running,
        phase: TaskPhase::DownloadingArtifact,
        message: "Paper artifact verified".to_owned(),
        progress: Some(TaskProgress {
            completed_steps: 1,
            total_steps: 3,
            transfer: None,
        }),
        error: None,
    });
    detail.last_event_sequence = 1;

    for (width, height) in [(80, 24), (40, 12)] {
        let text = render_to_text(&detail, width, height);
        assert!(text.contains("进行中"));
        assert!(text.contains("事件"));
    }
    assert!(render_to_text(&detail, 80, 24).contains("Paperartifactverified"));
}

#[test]
fn renders_backup_page_summaries_in_task_detail() {
    let instance_id = Uuid::new_v4();
    let backup_id = Uuid::new_v4();
    let mut task = task_snapshot();
    task.kind = TaskKind::ListWorldBackups;
    task.status = TaskStatus::Succeeded;
    task.phase = TaskPhase::Completed;
    task.result = Some(mc_core::TaskResult::WorldBackupsListed {
        instance_id,
        page: mc_core::BackupPage {
            backups: vec![mc_core::BackupSummary {
                id: backup_id,
                created_at: Utc::now(),
                consistency: mc_core::BackupConsistency::ServerStopped,
                file_count: 12,
                total_bytes: 2 * 1024 * 1024,
            }],
            next_cursor: None,
        },
    });
    let detail = TaskDetailState::new(task);
    let text = render_to_text(&detail, 100, 30);
    assert!(text.contains("本页1个备份"));
    assert!(text.contains("12个文件"));
    assert!(text.contains("2.0MiB"));
}

#[test]
fn renders_byte_progress_speed_and_real_source() {
    let mut task = task_snapshot();
    task.progress = Some(TaskProgress {
        completed_steps: 1,
        total_steps: 3,
        transfer: Some(mc_core::TransferProgress {
            downloaded_bytes: 27 * 1024 * 1024,
            total_bytes: 54 * 1024 * 1024,
            bytes_per_second: Some(3 * 1024 * 1024),
            source: "Paper 官方 CDN · fill-data.papermc.io".to_owned(),
        }),
    });
    let detail = TaskDetailState::new(task);
    let wide = render_to_text(&detail, 80, 24);
    assert!(wide.contains("下载"));
    assert!(wide.contains("50%"));
    assert!(wide.contains("fill-data.papermc.io"));
    assert!(wide.contains("3.0MiB/s"));
    let compact = render_to_text(&detail, 40, 12);
    assert!(compact.contains("50%"));
}

#[test]
fn cancellation_requires_two_presses_and_only_applies_to_downloads() {
    let mut detail = TaskDetailState::new(task_snapshot());
    assert!(detail.can_cancel());
    assert_eq!(detail.request_cancel(), None);
    assert!(detail.cancel_armed);
    assert_eq!(detail.request_cancel(), Some(detail.task.id));
    assert!(detail.cancel_requested);

    detail.task.status = TaskStatus::Succeeded;
    assert!(!detail.can_cancel());
    assert_eq!(detail.request_cancel(), None);
}

fn task_snapshot() -> TaskSnapshot {
    TaskSnapshot {
        id: Uuid::new_v4(),
        kind: TaskKind::CreatePaperInstance,
        instance_id: None,
        status: TaskStatus::Running,
        phase: TaskPhase::DownloadingArtifact,
        message: "Downloading and verifying Paper".to_owned(),
        progress: Some(TaskProgress {
            completed_steps: 1,
            total_steps: 3,
            transfer: None,
        }),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        finished_at: None,
        result: None,
        error: None,
        last_event_sequence: 1,
    }
}

fn render_to_text(detail: &TaskDetailState, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    terminal
        .draw(|frame| render(frame, detail, 0, &mut HitMap::default()))
        .expect("render");
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .filter(|symbol| !symbol.chars().all(char::is_whitespace))
        .collect()
}
