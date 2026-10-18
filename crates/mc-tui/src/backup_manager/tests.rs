use chrono::Utc;
use mc_core::{
    BackupCursor, BackupPage, BackupRestorePlan, BackupSummary, TaskKind, TaskPhase, TaskProgress,
    TaskResult, TaskSnapshot, TaskStatus,
};
use uuid::Uuid;

use super::{BackupManagerState, BackupOperation};

#[test]
fn backup_pages_append_without_duplicates_and_advance_the_cursor() {
    let instance_id = Uuid::new_v4();
    let first_id = Uuid::new_v4();
    let second_id = Uuid::new_v4();
    let cursor = BackupCursor {
        created_at: Utc::now(),
        id: first_id,
    };
    let mut state = BackupManagerState::new(instance_id, "生存服".to_owned());
    state.begin_list(None);

    state.track_task(&list_task(
        instance_id,
        TaskStatus::Succeeded,
        BackupPage {
            backups: vec![summary(first_id)],
            next_cursor: Some(cursor.clone()),
        },
    ));
    assert_eq!(state.backups.len(), 1);
    assert_eq!(state.next_cursor, Some(cursor.clone()));

    state.begin_list(Some(cursor));
    state.track_task(&list_task(
        instance_id,
        TaskStatus::Succeeded,
        BackupPage {
            backups: vec![summary(first_id), summary(second_id)],
            next_cursor: None,
        },
    ));
    assert_eq!(state.backups.len(), 2);
    assert_eq!(state.backups[1].id, second_id);
    assert!(!state.has_more());
    assert!(state
        .notice
        .as_deref()
        .unwrap_or_default()
        .contains("完整性校验"));
}

#[test]
fn restore_plan_requires_the_exact_confirmation_phrase() {
    let instance_id = Uuid::new_v4();
    let backup_id = Uuid::new_v4();
    let mut state = BackupManagerState::new(instance_id, "生存服".to_owned());
    state.begin_plan();
    state.track_task(&task(
        instance_id,
        TaskKind::PlanWorldBackupRestore,
        TaskStatus::Succeeded,
        TaskResult::WorldBackupRestorePlanned {
            instance_id,
            plan: restore_plan(instance_id, backup_id),
        },
    ));

    assert!(state.is_reviewing_plan());
    state.begin_confirmation();
    for character in "restore".chars() {
        state.insert_confirmation_char(character);
    }
    assert_eq!(state.confirmation_input, "RESTORE");
    assert!(state.confirmation_is_exact());
    for _ in 0..7 {
        state.backspace_confirmation();
    }
    assert!(!state.confirmation_is_exact());
}

#[test]
fn failed_verification_keeps_the_loaded_backup_list() {
    let instance_id = Uuid::new_v4();
    let backup_id = Uuid::new_v4();
    let mut state = BackupManagerState::new(instance_id, "生存服".to_owned());
    state.begin_list(None);
    state.track_task(&list_task(
        instance_id,
        TaskStatus::Succeeded,
        BackupPage {
            backups: vec![summary(backup_id)],
            next_cursor: None,
        },
    ));
    state.begin_verify();
    state.track_task(&TaskSnapshot {
        error: Some(mc_core::AppError::new(
            mc_core::ErrorCode::BackupIntegrityFailed,
            "backup manifest does not match stored files",
        )),
        status: TaskStatus::Failed,
        ..task(
            instance_id,
            TaskKind::VerifyWorldBackup,
            TaskStatus::Failed,
            TaskResult::WorldBackupVerified {
                instance_id,
                backup: summary(backup_id),
            },
        )
    });

    assert_eq!(state.backups.len(), 1);
    assert_eq!(state.operation(), None);
    assert!(state
        .error
        .as_deref()
        .unwrap_or_default()
        .contains("manifest"));
}

#[test]
fn failed_restore_clears_the_stale_plan_but_keeps_the_catalog() {
    let instance_id = Uuid::new_v4();
    let backup_id = Uuid::new_v4();
    let mut state = BackupManagerState::new(instance_id, "生存服".to_owned());
    state.begin_list(None);
    state.track_task(&list_task(
        instance_id,
        TaskStatus::Succeeded,
        BackupPage {
            backups: vec![summary(backup_id)],
            next_cursor: None,
        },
    ));
    state.plan = Some(restore_plan(instance_id, backup_id));
    state.begin_restore();
    let mut failed = task(
        instance_id,
        TaskKind::RestoreWorldBackup,
        TaskStatus::Failed,
        TaskResult::WorldBackupRestored {
            instance_id,
            backup_id,
        },
    );
    failed.error = Some(mc_core::AppError::new(
        mc_core::ErrorCode::BackupRestoreRollbackFailed,
        "restore failed and rollback requires attention",
    ));
    state.track_task(&failed);

    assert!(state.plan.is_none());
    assert_eq!(state.operation(), None);
    assert_eq!(state.backups.len(), 1);
    assert!(state
        .error
        .as_deref()
        .unwrap_or_default()
        .contains("rollback"),);
}

fn summary(id: Uuid) -> BackupSummary {
    BackupSummary {
        id,
        created_at: Utc::now(),
        consistency: mc_core::BackupConsistency::ServerStopped,
        file_count: 12,
        total_bytes: 2 * 1024 * 1024,
    }
}

fn list_task(instance_id: Uuid, status: TaskStatus, page: BackupPage) -> TaskSnapshot {
    task(
        instance_id,
        TaskKind::ListWorldBackups,
        status,
        TaskResult::WorldBackupsListed { instance_id, page },
    )
}

fn task(instance_id: Uuid, kind: TaskKind, status: TaskStatus, result: TaskResult) -> TaskSnapshot {
    TaskSnapshot {
        id: Uuid::new_v4(),
        kind,
        instance_id: Some(instance_id),
        status,
        phase: TaskPhase::Completed,
        message: "task completed".to_owned(),
        progress: Some(TaskProgress {
            completed_steps: 1,
            total_steps: 1,
            transfer: None,
        }),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        finished_at: Some(Utc::now()),
        result: Some(result),
        error: None,
        last_event_sequence: 2,
    }
}

fn restore_plan(instance_id: Uuid, backup_id: Uuid) -> BackupRestorePlan {
    BackupRestorePlan {
        instance_id,
        backup_id,
        created_at: Utc::now(),
        current_worlds: vec!["world".to_owned(), "world_nether".to_owned()],
        target_worlds: vec!["world".to_owned(), "world_the_end".to_owned()],
        replaced_worlds: vec!["world".to_owned()],
        added_worlds: vec!["world_the_end".to_owned()],
        removed_worlds: vec!["world_nether".to_owned()],
        required_space_bytes: 2 * 1024 * 1024,
        available_space_bytes: 10 * 1024 * 1024 * 1024,
        confirmation_token: format!("v1:{}", "a".repeat(64)),
    }
}

#[test]
fn restoring_operation_is_exposed_for_the_renderer() {
    let mut state = BackupManagerState::new(Uuid::new_v4(), "生存服".to_owned());
    state.begin_restore();
    assert_eq!(state.operation(), Some(BackupOperation::Restoring));
    assert!(state.is_restoring());
    assert!(state.is_busy());
}
