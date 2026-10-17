mod records;
mod schema;

use chrono::Utc;
use mc_core::{AppError, ErrorCode, TaskEvent, TaskKind, TaskPhase, TaskSnapshot};
use rusqlite::Connection;
use std::{path::PathBuf, time::Duration};
use tokio::sync::Semaphore;
use uuid::Uuid;

const DATABASE_CONCURRENCY: usize = 4;

/// Async facade over durable task records stored in the shared index database.
/// All SQLite work runs on Tokio's blocking pool with a small admission limit.
#[derive(Clone)]
pub struct TaskStore {
    path: PathBuf,
    database_slots: std::sync::Arc<Semaphore>,
}

impl TaskStore {
    pub async fn open(path: impl Into<PathBuf>) -> Result<Self, AppError> {
        let store = Self {
            path: path.into(),
            database_slots: std::sync::Arc::new(Semaphore::new(DATABASE_CONCURRENCY)),
        };
        store.run(schema::migrate).await?;
        store.run(records::recover_incomplete).await?;
        Ok(store)
    }

    pub async fn create(
        &self,
        kind: TaskKind,
        instance_id: Option<Uuid>,
    ) -> Result<TaskSnapshot, AppError> {
        let now = Utc::now();
        let snapshot = TaskSnapshot {
            id: Uuid::new_v4(),
            kind,
            instance_id,
            status: mc_core::TaskStatus::Queued,
            phase: TaskPhase::Queued,
            message: "Waiting for an execution slot.".to_owned(),
            progress: None,
            created_at: now,
            updated_at: now,
            finished_at: None,
            result: None,
            error: None,
            last_event_sequence: 0,
        };
        self.run(move |connection| records::persist_snapshot(connection, snapshot))
            .await
            .map(|(snapshot, _event)| snapshot)
    }

    pub async fn get(&self, id: Uuid) -> Result<TaskSnapshot, AppError> {
        self.run(move |connection| records::get(connection, id))
            .await
    }

    pub async fn find_active(
        &self,
        kind: TaskKind,
        instance_id: Uuid,
    ) -> Result<Option<TaskSnapshot>, AppError> {
        self.run(move |connection| records::find_active(connection, kind, instance_id))
            .await
    }

    pub async fn list(&self, limit: u16) -> Result<Vec<TaskSnapshot>, AppError> {
        if !(1..=100).contains(&limit) {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Task list limit must be between 1 and 100.",
            ));
        }
        self.run(move |connection| records::list(connection, limit))
            .await
    }

    pub async fn events(
        &self,
        id: Uuid,
        after_sequence: i64,
        limit: u16,
    ) -> Result<Vec<TaskEvent>, AppError> {
        if after_sequence < 0 || !(1..=500).contains(&limit) {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Task event cursor or page size is invalid.",
            ));
        }
        self.run(move |connection| records::events(connection, id, after_sequence, limit))
            .await
    }

    pub(crate) async fn persist(&self, snapshot: &mut TaskSnapshot) -> Result<TaskEvent, AppError> {
        let snapshot_value = snapshot.clone();
        let (updated, event) = self
            .run(move |connection| records::persist_snapshot(connection, snapshot_value))
            .await?;
        *snapshot = updated;
        Ok(event)
    }

    async fn run<T, F>(&self, operation: F) -> Result<T, AppError>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> Result<T, AppError> + Send + 'static,
    {
        let permit = self
            .database_slots
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| AppError::new(ErrorCode::Internal, "Task database is shutting down."))?;
        let path = self.path.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let mut connection = Connection::open(path).map_err(records::storage_error)?;
            connection
                .busy_timeout(Duration::from_secs(5))
                .map_err(records::storage_error)?;
            connection
                .execute_batch("PRAGMA foreign_keys = ON; PRAGMA synchronous = FULL;")
                .map_err(records::storage_error)?;
            operation(&mut connection)
        })
        .await
        .map_err(|error| {
            AppError::new(ErrorCode::Internal, "Task database worker failed.")
                .with_evidence(error.to_string())
        })?
    }
}
