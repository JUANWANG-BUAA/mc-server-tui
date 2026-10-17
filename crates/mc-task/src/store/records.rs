use chrono::Utc;
use mc_core::{AppError, ErrorCode, TaskEvent, TaskPhase, TaskSnapshot, TaskStatus};
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

pub(super) fn get(connection: &Connection, id: Uuid) -> Result<TaskSnapshot, AppError> {
    let encoded = connection
        .query_row(
            "SELECT snapshot_json FROM mc_tasks WHERE id = ?1",
            [id.to_string()],
            |row| row.get::<_, String>(0),
        )
        .map_err(|error| {
            if matches!(error, rusqlite::Error::QueryReturnedNoRows) {
                task_not_found(id)
            } else {
                storage_error(error)
            }
        })?;
    decode_snapshot(&encoded)
}

pub(super) fn list(connection: &Connection, limit: u16) -> Result<Vec<TaskSnapshot>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT snapshot_json FROM mc_tasks \
             ORDER BY created_at DESC, id DESC LIMIT ?1",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map([limit], |row| row.get::<_, String>(0))
        .map_err(storage_error)?;
    rows.map(|row| decode_snapshot(&row.map_err(storage_error)?))
        .collect()
}

pub(super) fn find_active(
    connection: &Connection,
    kind: mc_core::TaskKind,
    instance_id: Uuid,
) -> Result<Option<TaskSnapshot>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT snapshot_json FROM mc_tasks \
             WHERE status IN ('queued', 'running') \
             ORDER BY created_at DESC, id DESC",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(storage_error)?;
    for row in rows {
        let snapshot = decode_snapshot(&row.map_err(storage_error)?)?;
        if snapshot.kind == kind && snapshot.instance_id == Some(instance_id) {
            return Ok(Some(snapshot));
        }
    }
    Ok(None)
}

pub(super) fn events(
    connection: &Connection,
    id: Uuid,
    after_sequence: i64,
    limit: u16,
) -> Result<Vec<TaskEvent>, AppError> {
    let exists = connection
        .query_row(
            "SELECT 1 FROM mc_tasks WHERE id = ?1",
            [id.to_string()],
            |_| Ok(()),
        )
        .optional()
        .map_err(storage_error)?
        .is_some();
    if !exists {
        return Err(task_not_found(id));
    }
    let mut statement = connection
        .prepare(
            "SELECT event_json FROM mc_task_events \
             WHERE task_id = ?1 AND sequence > ?2 ORDER BY sequence LIMIT ?3",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![id.to_string(), after_sequence, limit], |row| {
            row.get::<_, String>(0)
        })
        .map_err(storage_error)?;
    rows.map(|row| decode_event(&row.map_err(storage_error)?))
        .collect()
}

pub(super) fn persist_snapshot(
    connection: &mut Connection,
    mut snapshot: TaskSnapshot,
) -> Result<(TaskSnapshot, TaskEvent), AppError> {
    let transaction = connection.transaction().map_err(storage_error)?;
    let mut event = TaskEvent {
        sequence: 0,
        task_id: snapshot.id,
        occurred_at: snapshot.updated_at,
        status: snapshot.status,
        phase: snapshot.phase,
        message: snapshot.message.clone(),
        progress: snapshot.progress,
        error: snapshot.error.clone(),
    };
    let encoded_snapshot = serde_json::to_string(&snapshot).map_err(encoding_error)?;
    transaction
        .execute(
            "INSERT INTO mc_tasks (id, created_at, updated_at, status, snapshot_json) \
             VALUES (?1, ?2, ?3, ?4, ?5) \
             ON CONFLICT(id) DO UPDATE SET updated_at = excluded.updated_at, \
                 status = excluded.status, snapshot_json = excluded.snapshot_json",
            params![
                snapshot.id.to_string(),
                snapshot.created_at.to_rfc3339(),
                snapshot.updated_at.to_rfc3339(),
                status_name(snapshot.status),
                encoded_snapshot,
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO mc_task_events (task_id, occurred_at, event_json) \
             VALUES (?1, ?2, '{}')",
            params![snapshot.id.to_string(), snapshot.updated_at.to_rfc3339()],
        )
        .map_err(storage_error)?;
    event.sequence = transaction.last_insert_rowid();
    snapshot.last_event_sequence = event.sequence;
    let encoded_snapshot = serde_json::to_string(&snapshot).map_err(encoding_error)?;
    let encoded_event = serde_json::to_string(&event).map_err(encoding_error)?;
    transaction
        .execute(
            "UPDATE mc_tasks SET snapshot_json = ?1 WHERE id = ?2",
            params![encoded_snapshot, snapshot.id.to_string()],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "UPDATE mc_task_events SET event_json = ?1 WHERE sequence = ?2",
            params![encoded_event, event.sequence],
        )
        .map_err(storage_error)?;
    transaction.commit().map_err(storage_error)?;
    Ok((snapshot, event))
}

pub(super) fn recover_incomplete(connection: &mut Connection) -> Result<(), AppError> {
    let mut statement = connection
        .prepare("SELECT snapshot_json FROM mc_tasks WHERE status IN ('queued', 'running')")
        .map_err(storage_error)?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(storage_error)?;
    let mut snapshots = rows
        .map(|row| decode_snapshot(&row.map_err(storage_error)?))
        .collect::<Result<Vec<_>, _>>()?;
    drop(statement);

    for snapshot in &mut snapshots {
        snapshot.status = TaskStatus::Interrupted;
        snapshot.phase = TaskPhase::Interrupted;
        snapshot.message = "The daemon stopped before this task completed.".to_owned();
        snapshot.finished_at = Some(Utc::now());
        snapshot.error = Some(
            AppError::new(
                ErrorCode::TaskInterrupted,
                "The task was interrupted when the daemon stopped.",
            )
            .with_action("Review the task details, then submit the operation again.")
            .retryable(),
        );
        persist_snapshot(connection, snapshot.clone())?;
    }
    Ok(())
}

fn status_name(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Queued => "queued",
        TaskStatus::Running => "running",
        TaskStatus::Succeeded => "succeeded",
        TaskStatus::Failed => "failed",
        TaskStatus::Interrupted => "interrupted",
    }
}

fn decode_snapshot(value: &str) -> Result<TaskSnapshot, AppError> {
    serde_json::from_str(value).map_err(schema_error)
}

fn decode_event(value: &str) -> Result<TaskEvent, AppError> {
    serde_json::from_str(value).map_err(schema_error)
}

fn task_not_found(id: Uuid) -> AppError {
    AppError::new(
        ErrorCode::TaskNotFound,
        "The requested task does not exist.",
    )
    .with_evidence(format!("task_id={id}"))
}

fn encoding_error(error: serde_json::Error) -> AppError {
    AppError::new(ErrorCode::Internal, "Could not encode task data.")
        .with_evidence(error.to_string())
}

fn schema_error(error: serde_json::Error) -> AppError {
    AppError::new(
        ErrorCode::SourceSchemaChanged,
        "Task data could not be decoded.",
    )
    .with_evidence(error.to_string())
}

pub(super) fn storage_error(error: impl std::fmt::Display) -> AppError {
    AppError::new(ErrorCode::Internal, "Could not access the task database.")
        .with_evidence(error.to_string())
}
