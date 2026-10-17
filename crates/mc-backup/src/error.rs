use std::{io, path::PathBuf};

use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum BackupError {
    #[error("{operation}: {path}: {source}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("The instance server directory is not a regular directory: {0}")]
    InvalidServerDirectory(PathBuf),
    #[error("The configured world directory was not found: {0}")]
    WorldNotFound(String),
    #[error("The world name is not a safe single directory name: {0}")]
    InvalidWorldName(String),
    #[error("The filesystem contains an unsupported symbolic link or special entry: {0}")]
    UnsafeFilesystemEntry(PathBuf),
    #[error("There are no world directories to back up")]
    NoWorlds,
    #[error("World data changed while it was being copied or verified: {0}")]
    SourceChanged(PathBuf),
    #[error("Not enough free space for a world backup (need {needed} bytes, have {available})")]
    InsufficientSpace { needed: u64, available: u64 },
    #[error("Backup {0} already exists")]
    AlreadyExists(Uuid),
    #[error("Backup {0} does not exist")]
    NotFound(Uuid),
    #[error("Backup {backup_id} was renamed into place, but durability could not be confirmed at {path}: {detail}")]
    CommitUncertain {
        backup_id: Uuid,
        path: PathBuf,
        detail: String,
    },
    #[error("Backup {backup_id} was removed after its commit could not be synced: {detail}")]
    CommitRolledBack { backup_id: Uuid, detail: String },
    #[error("Backup manifest is invalid: {0}")]
    InvalidManifest(String),
    #[error("Backup content failed integrity verification: {0}")]
    IntegrityMismatch(PathBuf),
    #[error("Could not encode or decode backup metadata: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Backup worker could not complete: {0}")]
    WorkerFailed(String),
}

pub(crate) fn io_error(
    operation: &'static str,
    path: impl Into<PathBuf>,
    source: io::Error,
) -> BackupError {
    BackupError::Io {
        operation,
        path: path.into(),
        source,
    }
}
