use crate::TaskStore;
use chrono::Utc;
use mc_core::{AppError, TaskPhase, TaskProgress, TaskResult, TaskSnapshot, TaskStatus};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Single-task reporter; every phase change is durably appended before returning.
#[derive(Clone)]
pub struct TaskReporter {
    store: TaskStore,
    snapshot: Arc<Mutex<TaskSnapshot>>,
}

impl TaskReporter {
    pub(crate) fn new(store: TaskStore, snapshot: TaskSnapshot) -> Self {
        Self {
            store,
            snapshot: Arc::new(Mutex::new(snapshot)),
        }
    }

    pub async fn set_phase(
        &self,
        phase: TaskPhase,
        message: impl Into<String>,
        progress: Option<TaskProgress>,
    ) -> Result<(), AppError> {
        self.update(|snapshot| {
            snapshot.status = TaskStatus::Running;
            snapshot.phase = phase;
            snapshot.message = message.into();
            snapshot.progress = progress;
            snapshot.error = None;
        })
        .await
    }

    pub(crate) async fn start(&self) -> Result<(), AppError> {
        let kind = self.snapshot.lock().await.kind;
        match kind {
            mc_core::TaskKind::CreatePaperInstance => {
                self.set_phase(TaskPhase::ResolvingArtifact, "Starting task.", None)
                    .await
            }
            mc_core::TaskKind::CreateWorldBackup => {
                self.set_phase(
                    TaskPhase::WaitingForInstance,
                    "Waiting to reserve the stopped instance.",
                    None,
                )
                .await
            }
            mc_core::TaskKind::ListWorldBackups
            | mc_core::TaskKind::VerifyWorldBackup
            | mc_core::TaskKind::PlanWorldBackupRestore
            | mc_core::TaskKind::RestoreWorldBackup => {
                self.set_phase(
                    TaskPhase::WaitingForInstance,
                    "Waiting for other work on this instance to finish.",
                    None,
                )
                .await
            }
        }
    }

    pub(crate) async fn succeed(&self, result: TaskResult) {
        let instance_id = result.instance_id();
        let progress = match &result {
            TaskResult::InstanceCreated(_) => TaskProgress {
                completed_steps: 3,
                total_steps: 3,
            },
            TaskResult::WorldBackupCreated { .. } => TaskProgress {
                completed_steps: 1,
                total_steps: 1,
            },
            TaskResult::WorldBackupsListed { .. }
            | TaskResult::WorldBackupVerified { .. }
            | TaskResult::WorldBackupRestorePlanned { .. }
            | TaskResult::WorldBackupRestored { .. } => TaskProgress {
                completed_steps: 1,
                total_steps: 1,
            },
        };
        if let Err(error) = self
            .update(|snapshot| {
                snapshot.status = TaskStatus::Succeeded;
                snapshot.phase = TaskPhase::Completed;
                snapshot.message = "Task completed successfully.".to_owned();
                snapshot.progress = Some(progress);
                snapshot.instance_id = Some(instance_id);
                snapshot.finished_at = Some(Utc::now());
                snapshot.result = Some(result);
                snapshot.error = None;
            })
            .await
        {
            let task_id = self.id().await;
            tracing::error!(task_id = %task_id, %error, "could not persist task result");
        }
    }

    pub(crate) async fn fail(&self, error: AppError) {
        if let Err(persist_error) = self
            .update(|snapshot| {
                snapshot.status = TaskStatus::Failed;
                snapshot.phase = TaskPhase::Failed;
                snapshot.message = error.reason.clone();
                snapshot.finished_at = Some(Utc::now());
                snapshot.result = None;
                snapshot.error = Some(error);
            })
            .await
        {
            let task_id = self.id().await;
            tracing::error!(task_id = %task_id, %persist_error, "could not persist task failure");
        }
    }

    async fn update<F>(&self, update: F) -> Result<(), AppError>
    where
        F: FnOnce(&mut TaskSnapshot),
    {
        let mut snapshot = self.snapshot.lock().await;
        update(&mut snapshot);
        snapshot.updated_at = Utc::now();
        self.store.persist(&mut snapshot).await?;
        Ok(())
    }

    async fn id(&self) -> Uuid {
        self.snapshot.lock().await.id
    }
}
