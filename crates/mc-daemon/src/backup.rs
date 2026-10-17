use mc_backup::BackupService;
use mc_core::{AppError, TaskKind, TaskPhase, TaskProgress, TaskResult, TaskSnapshot};
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use mc_core::{ErrorCode, TaskKind, TaskStatus};
    use mc_instance::InstanceService;
    use mc_supervisor::ProcessSupervisor;
    use mc_task::TaskManager;
    use tempfile::TempDir;
    use tokio::time::{sleep, timeout};
    use uuid::Uuid;

    use super::{create_world_backup_task, BackupService};

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
}
