use crate::TaskStore;
use chrono::Utc;
use mc_core::{AppError, TaskPhase, TaskProgress, TaskResult, TaskSnapshot, TaskStatus};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::Mutex;
use uuid::Uuid;

/// Single-task reporter; every phase change is durably appended before returning.
#[derive(Clone)]
pub struct TaskReporter {
    store: TaskStore,
    snapshot: Arc<Mutex<TaskSnapshot>>,
    cancellation: Arc<AtomicBool>,
}

impl TaskReporter {
    pub(crate) fn new(
        store: TaskStore,
        snapshot: TaskSnapshot,
        cancellation: Arc<AtomicBool>,
    ) -> Self {
        Self {
            store,
            snapshot: Arc::new(Mutex::new(snapshot)),
            cancellation,
        }
    }

    pub fn cancellation_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancellation)
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancellation.load(Ordering::Relaxed)
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
            mc_core::TaskKind::ManageInstance => {
                self.set_phase(TaskPhase::ManagingInstance, "准备扩展或整合包操作。", None)
                    .await
            }
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
                transfer: None,
            },
            TaskResult::ManagementCompleted { .. } | TaskResult::WorldBackupCreated { .. } => {
                TaskProgress {
                    completed_steps: 1,
                    total_steps: 1,
                    transfer: None,
                }
            }
            TaskResult::WorldBackupsListed { .. }
            | TaskResult::WorldBackupVerified { .. }
            | TaskResult::WorldBackupRestorePlanned { .. }
            | TaskResult::WorldBackupRestored { .. } => TaskProgress {
                completed_steps: 1,
                total_steps: 1,
                transfer: None,
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
        let cancelled = error.code == mc_core::ErrorCode::TaskCancelled;
        if let Err(persist_error) = self
            .update(|snapshot| {
                snapshot.status = if cancelled {
                    TaskStatus::Cancelled
                } else {
                    TaskStatus::Failed
                };
                snapshot.phase = if cancelled {
                    TaskPhase::Cancelled
                } else {
                    TaskPhase::Failed
                };
                snapshot.message = if cancelled {
                    "Task cancelled by user.".to_owned()
                } else {
                    error.reason.clone()
                };
                snapshot.finished_at = Some(Utc::now());
                snapshot.result = None;
                snapshot.error = if cancelled { None } else { Some(error) };
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
