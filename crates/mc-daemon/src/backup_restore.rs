use mc_backup::BackupService;
use mc_core::{AppError, ErrorCode, TaskKind, TaskPhase, TaskProgress, TaskResult, TaskSnapshot};
use mc_supervisor::ProcessSupervisor;
use mc_task::{TaskManager, TaskReporter};
use uuid::Uuid;

/// Queues an integrity-checked restore preview while holding the stopped lease.
pub(crate) async fn plan_world_backup_restore_task(
    tasks: &TaskManager,
    supervisor: &ProcessSupervisor,
    backups: BackupService,
    instance_id: Uuid,
    backup_id: Uuid,
) -> Result<TaskSnapshot, AppError> {
    let supervisor = supervisor.clone();
    tasks
        .submit(
            TaskKind::PlanWorldBackupRestore,
            Some(instance_id),
            move |reporter: TaskReporter| async move {
                let guard = supervisor.lock_stopped_instance(instance_id).await?;
                reporter
                    .set_phase(
                        TaskPhase::PlanningBackupRestore,
                        format!("Verifying backup {backup_id} and measuring restore impact."),
                        Some(TaskProgress {
                            completed_steps: 0,
                            total_steps: 1,
                        }),
                    )
                    .await?;
                let plan = backups
                    .plan_restore(guard, backup_id)
                    .await
                    .map_err(AppError::from)?;
                reporter
                    .set_phase(
                        TaskPhase::PlanningBackupRestore,
                        "Restore plan verified; review the impact and confirmation token.",
                        Some(TaskProgress {
                            completed_steps: 1,
                            total_steps: 1,
                        }),
                    )
                    .await?;
                Ok(TaskResult::WorldBackupRestorePlanned { instance_id, plan })
            },
        )
        .await
}

/// Queues restore only with a well-formed token from a completed preview task.
pub(crate) async fn restore_world_backup_task(
    tasks: &TaskManager,
    supervisor: &ProcessSupervisor,
    backups: BackupService,
    instance_id: Uuid,
    backup_id: Uuid,
    confirmation_token: String,
) -> Result<TaskSnapshot, AppError> {
    if !is_confirmation_token(&confirmation_token) {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Restore confirmation token is malformed.",
        )
        .with_action("Copy the complete versioned token from a successful restore-plan task."));
    }
    let supervisor = supervisor.clone();
    tasks
        .submit(
            TaskKind::RestoreWorldBackup,
            Some(instance_id),
            move |reporter: TaskReporter| async move {
                let guard = supervisor.lock_stopped_instance(instance_id).await?;
                reporter
                    .set_phase(
                        TaskPhase::RestoringWorldBackup,
                        format!("Restoring verified backup {backup_id}."),
                        Some(TaskProgress {
                            completed_steps: 0,
                            total_steps: 1,
                        }),
                    )
                    .await?;
                let manifest = backups
                    .restore(guard, backup_id, confirmation_token)
                    .await
                    .map_err(AppError::from)?;
                reporter
                    .set_phase(
                        TaskPhase::RestoringWorldBackup,
                        "World restore committed and verified.",
                        Some(TaskProgress {
                            completed_steps: 1,
                            total_steps: 1,
                        }),
                    )
                    .await?;
                Ok(TaskResult::WorldBackupRestored {
                    instance_id,
                    backup_id: manifest.id,
                })
            },
        )
        .await
}

fn is_confirmation_token(token: &str) -> bool {
    let Some(digest) = token.strip_prefix("v1:") else {
        return false;
    };
    digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use mc_core::{ErrorCode, TaskKind, TaskStatus};
    use mc_instance::InstanceService;
    use mc_task::TaskManager;
    use tempfile::TempDir;
    use tokio::time::{sleep, timeout};
    use uuid::Uuid;

    use super::{
        is_confirmation_token, plan_world_backup_restore_task, restore_world_backup_task,
        BackupService,
    };

    #[test]
    fn confirmation_tokens_are_bounded_versioned_sha256_values() {
        assert!(is_confirmation_token(&format!("v1:{}", "a".repeat(64))));
        assert!(!is_confirmation_token("v1:short"));
        assert!(!is_confirmation_token(&format!("v2:{}", "a".repeat(64))));
        assert!(!is_confirmation_token(&format!("v1:{}", "A".repeat(64))));
    }

    #[tokio::test]
    async fn plan_and_restore_are_durable_tasks_and_reject_bad_confirmation_early() {
        let temp = TempDir::new().expect("temporary state directory");
        let instances = InstanceService::open(temp.path())
            .await
            .expect("instance store opens");
        let supervisor = mc_supervisor::ProcessSupervisor::new(instances);
        let tasks = TaskManager::open(temp.path().join("tasks.sqlite3"))
            .await
            .expect("task store opens");
        let instance_id = Uuid::new_v4();
        let backup_id = Uuid::new_v4();

        let malformed = restore_world_backup_task(
            &tasks,
            &supervisor,
            BackupService,
            instance_id,
            backup_id,
            "invalid".to_owned(),
        )
        .await
        .expect_err("malformed token is rejected before queueing");
        assert_eq!(malformed.code, ErrorCode::InvalidInput);
        assert!(tasks.list(10).await.expect("list tasks").is_empty());

        let planned = plan_world_backup_restore_task(
            &tasks,
            &supervisor,
            BackupService,
            instance_id,
            backup_id,
        )
        .await
        .expect("plan request is queued");
        let restored = restore_world_backup_task(
            &tasks,
            &supervisor,
            BackupService,
            instance_id,
            backup_id,
            format!("v1:{}", "c".repeat(64)),
        )
        .await
        .expect("restore request is queued");
        assert_eq!(planned.kind, TaskKind::PlanWorldBackupRestore);
        assert_eq!(restored.kind, TaskKind::RestoreWorldBackup);
        assert_eq!(planned.status, TaskStatus::Queued);
        assert_eq!(restored.status, TaskStatus::Queued);

        for queued in [planned, restored] {
            let completed = timeout(Duration::from_secs(3), async {
                loop {
                    let task = tasks.get(queued.id).await.expect("task remains readable");
                    if task.status == TaskStatus::Failed {
                        return task;
                    }
                    sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .expect("missing instance reaches a terminal state");
            assert_eq!(
                completed
                    .error
                    .expect("failure has a structured error")
                    .code,
                ErrorCode::InstanceNotFound
            );
        }
    }
}
