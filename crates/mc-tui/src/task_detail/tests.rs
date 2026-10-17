use chrono::Utc;
use mc_core::{ErrorCode, TaskEvent, TaskKind, TaskPhase, TaskSnapshot, TaskStatus};
use uuid::Uuid;

use super::{TaskDetailState, TaskDetailUpdate};

#[test]
fn event_cursor_advances_monotonically_and_deduplicates_replayed_events() {
    let task = task_snapshot();
    let id = task.id;
    let mut detail = TaskDetailState::new(task.clone());
    detail.apply_result(Ok(TaskDetailUpdate {
        task: task.clone(),
        events: vec![event(id, 1, "queued"), event(id, 2, "downloading")],
    }));
    assert_eq!(detail.last_event_sequence, 2);
    assert_eq!(detail.query().after_sequence, 2);

    detail.apply_result(Ok(TaskDetailUpdate {
        task,
        events: vec![event(id, 2, "duplicate"), event(id, 3, "installed")],
    }));
    assert_eq!(detail.last_event_sequence, 3);
    assert_eq!(detail.events.len(), 3);
    assert_eq!(detail.events[1].message, "downloading");
}

#[test]
fn failed_refresh_preserves_task_events_and_cursor_for_reconnect() {
    let task = task_snapshot();
    let id = task.id;
    let mut detail = TaskDetailState::new(task.clone());
    detail.apply_result(Ok(TaskDetailUpdate {
        task,
        events: vec![event(id, 1, "queued")],
    }));
    detail.apply_result(Err(mc_core::AppError::new(
        ErrorCode::SourceUnavailable,
        "daemon unavailable",
    )));

    assert_eq!(detail.query().after_sequence, 1);
    assert_eq!(detail.events.len(), 1);
    assert_eq!(detail.error.as_deref(), Some("daemon unavailable"));
}

fn task_snapshot() -> TaskSnapshot {
    TaskSnapshot {
        id: Uuid::new_v4(),
        kind: TaskKind::CreatePaperInstance,
        instance_id: None,
        status: TaskStatus::Running,
        phase: TaskPhase::ResolvingArtifact,
        message: "Resolving Paper build".to_owned(),
        progress: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        finished_at: None,
        result: None,
        error: None,
        last_event_sequence: 0,
    }
}

fn event(task_id: Uuid, sequence: i64, message: &str) -> TaskEvent {
    TaskEvent {
        sequence,
        task_id,
        occurred_at: Utc::now(),
        status: TaskStatus::Running,
        phase: TaskPhase::DownloadingArtifact,
        message: message.to_owned(),
        progress: None,
        error: None,
    }
}
