use crate::{instance_locks::InstanceLockRegistry, TaskReporter, TaskStore};
use futures_util::FutureExt;
use mc_core::{AppError, ErrorCode, TaskEvent, TaskKind, TaskResult, TaskSnapshot};
use std::{future::Future, panic::AssertUnwindSafe, path::Path, sync::Arc};
use tokio::sync::Semaphore;
use uuid::Uuid;

const DEFAULT_MAX_CONCURRENT: usize = 2;
const DEFAULT_MAX_OUTSTANDING: usize = 8;
const MAX_OUTSTANDING_TASKS: usize = 256;

/// Runs long operations off the socket request path with bounded admission.
#[derive(Clone)]
pub struct TaskManager {
    store: TaskStore,
    execution_slots: Arc<Semaphore>,
    outstanding_slots: Arc<Semaphore>,
    instance_locks: InstanceLockRegistry,
    submission_lock: Arc<tokio::sync::Mutex<()>>,
}

impl TaskManager {
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, AppError> {
        Self::with_limits(path, DEFAULT_MAX_CONCURRENT, DEFAULT_MAX_OUTSTANDING).await
    }

    pub async fn with_limits(
        path: impl AsRef<Path>,
        max_concurrent: usize,
        max_outstanding: usize,
    ) -> Result<Self, AppError> {
        if max_concurrent == 0
            || max_outstanding < max_concurrent
            || max_outstanding > MAX_OUTSTANDING_TASKS
        {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Task limits require at least one worker, enough outstanding slots, and no more than 256 outstanding tasks.",
            ));
        }
        Ok(Self {
            store: TaskStore::open(path.as_ref().to_path_buf()).await?,
            execution_slots: Arc::new(Semaphore::new(max_concurrent)),
            outstanding_slots: Arc::new(Semaphore::new(max_outstanding)),
            instance_locks: InstanceLockRegistry::default(),
            submission_lock: Arc::new(tokio::sync::Mutex::new(())),
        })
    }

    /// Durably records a task, then schedules it under active and queued limits.
    pub async fn submit<F, Fut>(
        &self,
        kind: TaskKind,
        instance_id: Option<Uuid>,
        work: F,
    ) -> Result<TaskSnapshot, AppError>
    where
        F: FnOnce(TaskReporter) -> Fut + Send + 'static,
        Fut: Future<Output = Result<TaskResult, AppError>> + Send + 'static,
    {
        let outstanding_permit =
            self.outstanding_slots
                .clone()
                .try_acquire_owned()
                .map_err(|_| {
                    AppError::new(
                        ErrorCode::TaskQueueFull,
                        "The background task queue is full.",
                    )
                    .with_action("Wait for a task to finish, then retry this operation.")
                    .retryable()
                })?;
        let snapshot = self.store.create(kind, instance_id).await?;
        let reporter = TaskReporter::new(self.store.clone(), snapshot.clone());
        let execution_slots = Arc::clone(&self.execution_slots);
        let instance_locks = self.instance_locks.clone();

        tokio::spawn(async move {
            let _outstanding_permit = outstanding_permit;
            let _instance_permit = match snapshot.instance_id {
                Some(instance_id) => Some(instance_locks.acquire(instance_id).await),
                None => None,
            };
            let execution_permit = match execution_slots.acquire_owned().await {
                Ok(permit) => permit,
                Err(_) => {
                    reporter
                        .fail(AppError::new(
                            ErrorCode::Internal,
                            "The background task executor stopped before this task started.",
                        ))
                        .await;
                    return;
                }
            };
            if let Err(error) = reporter.start().await {
                tracing::error!(task_id = %snapshot.id, %error, "could not mark task as running");
                reporter.fail(error).await;
                return;
            }

            let task_reporter = reporter.clone();
            let outcome = AssertUnwindSafe(async move { work(task_reporter).await })
                .catch_unwind()
                .await;
            match outcome {
                Ok(Ok(result)) => reporter.succeed(result).await,
                Ok(Err(error)) => reporter.fail(error).await,
                Err(payload) => {
                    let detail = payload
                        .downcast_ref::<&str>()
                        .map(|message| (*message).to_owned())
                        .or_else(|| payload.downcast_ref::<String>().cloned())
                        .unwrap_or_else(|| "non-string panic payload".to_owned());
                    reporter
                        .fail(
                            AppError::new(
                                ErrorCode::Internal,
                                "The background task stopped unexpectedly.",
                            )
                            .with_action("Inspect the task event history, then retry if safe.")
                            .with_evidence(detail),
                        )
                        .await;
                }
            }
            drop(execution_permit);
        });
        Ok(snapshot)
    }

    /// Reuses an active task with the same kind and instance, or submits one.
    /// The admission lock serializes this check with other deduplicated submits.
    pub async fn submit_or_reuse_active<F, Fut>(
        &self,
        kind: TaskKind,
        instance_id: Uuid,
        work: F,
    ) -> Result<TaskSnapshot, AppError>
    where
        F: FnOnce(TaskReporter) -> Fut + Send + 'static,
        Fut: Future<Output = Result<TaskResult, AppError>> + Send + 'static,
    {
        let _submission_guard = self.submission_lock.lock().await;
        if let Some(active) = self.store.find_active(kind, instance_id).await? {
            return Ok(active);
        }
        self.submit(kind, Some(instance_id), work).await
    }

    pub async fn get(&self, id: Uuid) -> Result<TaskSnapshot, AppError> {
        self.store.get(id).await
    }

    pub async fn list(&self, limit: u16) -> Result<Vec<TaskSnapshot>, AppError> {
        self.store.list(limit).await
    }

    pub async fn events(
        &self,
        id: Uuid,
        after_sequence: i64,
        limit: u16,
    ) -> Result<Vec<TaskEvent>, AppError> {
        self.store.events(id, after_sequence, limit).await
    }
}
