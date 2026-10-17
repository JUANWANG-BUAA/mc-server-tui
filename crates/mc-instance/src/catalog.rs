use chrono::{DateTime, Utc};
use mc_core::{AppError, ErrorCode, InstanceSummary};
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use uuid::Uuid;

const INDEX_SCHEMA_VERSION: i64 = 1;

/// Rebuildable catalog boundary; instance files remain authoritative.
pub trait InstanceCatalog: Send + Sync {
    fn list(&self) -> Result<Vec<InstanceSummary>, AppError>;
    fn replace_snapshot(&self, entries: &[InstanceSummary]) -> Result<(), AppError>;
}

/// SQLite implementation used by the local daemon.
pub struct SqliteCatalog {
    connection: Mutex<Connection>,
}

impl SqliteCatalog {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AppError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(storage_error)?;
        }
        if let Ok(metadata) = std::fs::symlink_metadata(path) {
            if !metadata.file_type().is_file() {
                return Err(AppError::new(
                    ErrorCode::Internal,
                    "Instance index path is not a regular file.",
                ));
            }
        }
        let connection = Connection::open(path).map_err(storage_error)?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(storage_error)?;
        connection
            .execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = DELETE; PRAGMA synchronous = FULL;")
            .map_err(storage_error)?;
        migrate(&connection)?;
        set_private_file_permissions(path)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
}

impl InstanceCatalog for SqliteCatalog {
    fn list(&self) -> Result<Vec<InstanceSummary>, AppError> {
        let connection = self.connection.lock().map_err(|_| {
            AppError::new(ErrorCode::Internal, "Instance index lock is unavailable.")
        })?;
        let mut statement = connection
            .prepare(
                "SELECT id, name, provider, game_version, build_id, java_path, \
                 min_memory_mb, max_memory_mb, port, directory, created_at \
                 FROM instances ORDER BY created_at, name, id",
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map([], decode_summary)
            .map_err(storage_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(storage_error)
    }

    fn replace_snapshot(&self, entries: &[InstanceSummary]) -> Result<(), AppError> {
        let mut connection = self.connection.lock().map_err(|_| {
            AppError::new(ErrorCode::Internal, "Instance index lock is unavailable.")
        })?;
        let transaction = connection.transaction().map_err(storage_error)?;
        transaction
            .execute("DELETE FROM instances", [])
            .map_err(storage_error)?;
        {
            let mut statement = transaction
                .prepare(
                    "INSERT INTO instances \
                     (id, name, provider, game_version, build_id, java_path, \
                      min_memory_mb, max_memory_mb, port, directory, created_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                )
                .map_err(storage_error)?;
            for entry in entries {
                let build_id = i64::try_from(entry.build_id).map_err(|_| {
                    AppError::new(
                        ErrorCode::InvalidInput,
                        "Instance build ID exceeds the SQLite integer range.",
                    )
                })?;
                statement
                    .execute(params![
                        entry.id.to_string(),
                        entry.name,
                        entry.provider,
                        entry.game_version,
                        build_id,
                        entry.java_path.to_string_lossy(),
                        entry.min_memory_mb,
                        entry.max_memory_mb,
                        entry.port,
                        entry.directory.to_string_lossy(),
                        entry.created_at.to_rfc3339(),
                    ])
                    .map_err(storage_error)?;
            }
        }
        transaction.commit().map_err(storage_error)
    }
}

fn migrate(connection: &Connection) -> Result<(), AppError> {
    let version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(storage_error)?;
    if version > INDEX_SCHEMA_VERSION {
        return Err(AppError::new(
            ErrorCode::SourceSchemaChanged,
            "The instance index was created by a newer application version.",
        )
        .with_evidence(format!(
            "database_schema={version}, supported_schema={INDEX_SCHEMA_VERSION}"
        )));
    }
    if version == 0 {
        connection
            .execute_batch(
                "BEGIN IMMEDIATE;
                 CREATE TABLE instances (
                     id TEXT PRIMARY KEY NOT NULL,
                     name TEXT NOT NULL,
                     provider TEXT NOT NULL,
                     game_version TEXT NOT NULL,
                     build_id INTEGER NOT NULL,
                     java_path TEXT NOT NULL,
                     min_memory_mb INTEGER NOT NULL,
                     max_memory_mb INTEGER NOT NULL,
                     port INTEGER NOT NULL,
                     directory TEXT NOT NULL UNIQUE,
                     created_at TEXT NOT NULL
                 );
                 CREATE INDEX instances_name_idx ON instances(name COLLATE NOCASE);
                 PRAGMA user_version = 1;
                 COMMIT;",
            )
            .map_err(storage_error)?;
    }
    Ok(())
}

fn decode_summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<InstanceSummary> {
    let id: String = row.get(0)?;
    let build_id: i64 = row.get(4)?;
    let created_at: String = row.get(10)?;
    let created_at = DateTime::parse_from_rfc3339(&created_at)
        .map(|time| time.with_timezone(&Utc))
        .map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                10,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?;
    Ok(InstanceSummary {
        id: Uuid::parse_str(&id).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?,
        name: row.get(1)?,
        provider: row.get(2)?,
        game_version: row.get(3)?,
        build_id: u64::try_from(build_id).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                4,
                rusqlite::types::Type::Integer,
                Box::new(error),
            )
        })?,
        java_path: PathBuf::from(row.get::<_, String>(5)?),
        min_memory_mb: row.get(6)?,
        max_memory_mb: row.get(7)?,
        port: row.get(8)?,
        directory: PathBuf::from(row.get::<_, String>(9)?),
        created_at,
    })
}

fn storage_error(error: impl std::fmt::Display) -> AppError {
    AppError::new(
        ErrorCode::Internal,
        "Could not read or update the instance index.",
    )
    .with_evidence(error.to_string())
}

fn set_private_file_permissions(path: &Path) -> Result<(), AppError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .map_err(storage_error)?;
    }
    Ok(())
}
