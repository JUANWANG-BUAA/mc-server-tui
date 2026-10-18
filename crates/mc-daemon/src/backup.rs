use mc_backup::BackupService;
use mc_core::{
    AppError, BackupCursor, ErrorCode, TaskKind, TaskPhase, TaskProgress, TaskResult, TaskSnapshot,
    MAX_BACKUP_PAGE_LIMIT,
};
use mc_instance::InstanceService;
use mc_supervisor::ProcessSupervisor;
use mc_task::{TaskManager, TaskReporter};
use uuid::Uuid;

/// Queues a durable backup task while keeping filesystem work off the request path.
pub(crate) async fn create_world_backup_task(
    tasks: &TaskManager,
    supervisor: &ProcessSupervisor,
    backups: BackupService,
    instance_id: Uuid,
) -> Result<TaskSnapshot, AppError> {
    let supervisor = supervisor.clone();
    tasks
        .submit_or_reuse_active(
            TaskKind::CreateWorldBackup,
            instance_id,
            move |reporter: TaskReporter| async move {
                let guard = supervisor.lock_stopped_instance(instance_id).await?;
                reporter
                    .set_phase(
                        TaskPhase::BackingUpWorld,
                        "Copying and hashing stopped world directories.",
                        Some(TaskProgress {
                            completed_steps: 0,
                            total_steps: 1,
                            transfer: None,
                        }),
                    )
                    .await?;
                let manifest = backups
                    .create(guard, Uuid::new_v4())
                    .await
                    .map_err(AppError::from)?;
                reporter
                    .set_phase(
                        TaskPhase::BackingUpWorld,
                        "World snapshot committed and verified.",
                        Some(TaskProgress {
                            completed_steps: 1,
                            total_steps: 1,
                            transfer: None,
                        }),
                    )
                    .await?;
                Ok(TaskResult::WorldBackupCreated {
                    instance_id,
                    backup_id: manifest.id,
                })
            },
        )
        .await
}

/// Queues a bounded backup page scan and integrity-checks the returned page.
pub(crate) async fn list_world_backups_task(
    tasks: &TaskManager,
    instances: InstanceService,
    backups: BackupService,
    instance_id: Uuid,
    limit: u16,
    cursor: Option<BackupCursor>,
) -> Result<TaskSnapshot, AppError> {
    if !(1..=MAX_BACKUP_PAGE_LIMIT).contains(&limit) {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            format!("Backup page limit must be between 1 and {MAX_BACKUP_PAGE_LIMIT}."),
        )
        .with_action("Choose a limit from 1 to 50, then retry."));
    }
    tasks
        .submit(
            TaskKind::ListWorldBackups,
            Some(instance_id),
            move |reporter: TaskReporter| async move {
                let instance = instances.get_async(instance_id).await?;
                reporter
                    .set_phase(
                        TaskPhase::ListingBackups,
                        "Reading backup metadata and verifying this page.",
                        Some(TaskProgress {
                            completed_steps: 0,
                            total_steps: 1,
                            transfer: None,
                        }),
                    )
                    .await?;
                let page = backups
                    .list_page(&instance, limit, cursor)
                    .await
                    .map_err(AppError::from)?;
                reporter
                    .set_phase(
                        TaskPhase::ListingBackups,
                        "Backup page listed and its contents verified.",
                        Some(TaskProgress {
                            completed_steps: 1,
                            total_steps: 1,
                            transfer: None,
                        }),
                    )
                    .await?;
                Ok(TaskResult::WorldBackupsListed { instance_id, page })
            },
        )
        .await
}

/// Queues an integrity scan for one selected backup.
pub(crate) async fn verify_world_backup_task(
    tasks: &TaskManager,
    instances: InstanceService,
    backups: BackupService,
    instance_id: Uuid,
    backup_id: Uuid,
) -> Result<TaskSnapshot, AppError> {
    tasks
        .submit(
            TaskKind::VerifyWorldBackup,
            Some(instance_id),
            move |reporter: TaskReporter| async move {
                let instance = instances.get_async(instance_id).await?;
                reporter
                    .set_phase(
                        TaskPhase::VerifyingBackup,
                        format!("Verifying backup {backup_id}.",),
                        Some(TaskProgress {
                            completed_steps: 0,
                            total_steps: 1,
                            transfer: None,
                        }),
                    )
                    .await?;
                let backup = backups
                    .verify_summary(&instance, backup_id)
                    .await
                    .map_err(AppError::from)?;
                reporter
                    .set_phase(
                        TaskPhase::VerifyingBackup,
                        "Backup integrity verified.",
                        Some(TaskProgress {
                            completed_steps: 1,
                            total_steps: 1,
                            transfer: None,
                        }),
                    )
                    .await?;
                Ok(TaskResult::WorldBackupVerified {
                    instance_id,
                    backup,
                })
            },
        )
        .await
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use mc_core::{ErrorCode, TaskKind, TaskStatus, MAX_BACKUP_PAGE_LIMIT};
    use mc_instance::InstanceService;
    use mc_supervisor::ProcessSupervisor;
    use mc_task::TaskManager;
    use tempfile::TempDir;
    use tokio::time::{sleep, timeout};
    use uuid::Uuid;

    use super::{
        create_world_backup_task, list_world_backups_task, verify_world_backup_task, BackupService,
    };

    #[tokio::test]
    async fn backup_api_returns_queued_task_and_persists_instance_errors() {
        let temp = TempDir::new().expect("temporary state directory");
        let instances = InstanceService::open(temp.path())
            .await
            .expect("instance store opens");
        let supervisor = ProcessSupervisor::new(instances);
        let tasks = TaskManager::open(temp.path().join("index.sqlite3"))
            .await
            .expect("task store opens");
        let instance_id = Uuid::new_v4();

        let queued = create_world_backup_task(&tasks, &supervisor, BackupService, instance_id)
            .await
            .expect("API accepts a durable task before doing backup work");
        assert_eq!(queued.kind, TaskKind::CreateWorldBackup);
        assert_eq!(queued.status, TaskStatus::Queued);

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
        .expect("missing-instance task reaches a terminal state");
        assert_eq!(
            completed
                .error
                .expect("failure has a structured error")
                .code,
            ErrorCode::InstanceNotFound
        );
    }

    #[tokio::test]
    async fn list_and_verify_apis_queue_work_and_persist_missing_instance_errors() {
        let temp = TempDir::new().expect("temporary state directory");
        let instances = InstanceService::open(temp.path())
            .await
            .expect("instance store opens");
        let tasks = TaskManager::open(temp.path().join("tasks.sqlite3"))
            .await
            .expect("task store opens");

        let invalid_limit = list_world_backups_task(
            &tasks,
            instances.clone(),
            BackupService,
            Uuid::new_v4(),
            MAX_BACKUP_PAGE_LIMIT + 1,
            None,
        )
        .await
        .expect_err("invalid page limit is rejected before queuing");
        assert_eq!(invalid_limit.code, ErrorCode::InvalidInput);
        assert!(tasks.list(10).await.expect("list queued tasks").is_empty());

        let listed = list_world_backups_task(
            &tasks,
            instances.clone(),
            BackupService,
            Uuid::new_v4(),
            10,
            None,
        )
        .await
        .expect("listing is queued");
        let verified = verify_world_backup_task(
            &tasks,
            instances,
            BackupService,
            Uuid::new_v4(),
            Uuid::new_v4(),
        )
        .await
        .expect("verification is queued");
        assert_eq!(listed.kind, TaskKind::ListWorldBackups);
        assert_eq!(verified.kind, TaskKind::VerifyWorldBackup);
        assert_eq!(listed.status, TaskStatus::Queued);
        assert_eq!(verified.status, TaskStatus::Queued);

        for queued in [listed, verified] {
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
            .expect("missing-instance task reaches a terminal state");
            assert_eq!(
                completed.error.expect("failure has an error").code,
                ErrorCode::InstanceNotFound
            );
        }
    }
}
