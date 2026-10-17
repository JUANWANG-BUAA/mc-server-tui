use mc_core::{AppError, ErrorCode};
use rusqlite::{Connection, OptionalExtension};

const TASK_SCHEMA_VERSION: i64 = 1;

pub(super) fn migrate(connection: &mut Connection) -> Result<(), AppError> {
    let transaction = connection
        .transaction()
        .map_err(super::records::storage_error)?;
    transaction
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS mc_task_schema (
                 id INTEGER PRIMARY KEY CHECK (id = 1),
                 version INTEGER NOT NULL
             );",
        )
        .map_err(super::records::storage_error)?;
    let version = transaction
        .query_row(
            "SELECT version FROM mc_task_schema WHERE id = 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(super::records::storage_error)?
        .unwrap_or(0);
    if version > TASK_SCHEMA_VERSION {
        return Err(AppError::new(
            ErrorCode::SourceSchemaChanged,
            "The task store was created by a newer application version.",
        )
        .with_evidence(format!(
            "task_schema={version}, supported_schema={TASK_SCHEMA_VERSION}"
        )));
    }
    if version == 0 {
        transaction
            .execute_batch(
                "CREATE TABLE mc_tasks (
                     id TEXT PRIMARY KEY NOT NULL,
                     created_at TEXT NOT NULL,
                     updated_at TEXT NOT NULL,
                     status TEXT NOT NULL,
                     snapshot_json TEXT NOT NULL
                 );
                 CREATE INDEX mc_tasks_created_idx ON mc_tasks(created_at DESC, id DESC);
                 CREATE TABLE mc_task_events (
                     sequence INTEGER PRIMARY KEY AUTOINCREMENT,
                     task_id TEXT NOT NULL REFERENCES mc_tasks(id) ON DELETE CASCADE,
                     occurred_at TEXT NOT NULL,
                     event_json TEXT NOT NULL
                 );
                 CREATE INDEX mc_task_events_task_idx \
                     ON mc_task_events(task_id, sequence);
                 INSERT INTO mc_task_schema (id, version) VALUES (1, 1);",
            )
            .map_err(super::records::storage_error)?;
    } else {
        let has_tasks = table_exists(&transaction, "mc_tasks")?;
        let has_events = table_exists(&transaction, "mc_task_events")?;
        if !has_tasks || !has_events {
            return Err(AppError::new(
                ErrorCode::SourceSchemaChanged,
                "The task store schema is incomplete.",
            ));
        }
    }
    transaction.commit().map_err(super::records::storage_error)
}

fn table_exists(connection: &Connection, name: &str) -> Result<bool, AppError> {
    connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [name],
            |_| Ok(()),
        )
        .optional()
        .map(|row| row.is_some())
        .map_err(super::records::storage_error)
}
