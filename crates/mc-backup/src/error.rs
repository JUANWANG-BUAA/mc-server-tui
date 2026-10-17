use std::{io, path::PathBuf};

use mc_core::{AppError, ErrorCode};
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

impl From<BackupError> for AppError {
    fn from(error: BackupError) -> Self {
        let evidence = error.to_string();
        let reason = evidence.clone();
        match error {
            BackupError::InsufficientSpace { .. } => AppError::new(ErrorCode::DiskSpaceLow, reason)
                .with_action("Free space on the instance volume, then retry the backup.")
                .retryable(),
            BackupError::IntegrityMismatch(_) | BackupError::InvalidManifest(_) => {
                AppError::new(ErrorCode::BackupIntegrityFailed, reason).with_action(
                    "Do not restore this backup; inspect the backup directory and its manifest.",
                )
            }
            BackupError::CommitUncertain { .. } => AppError::new(ErrorCode::BackupFailed, reason)
                .with_action("Check the backup list and verify this backup before retrying.")
                .with_evidence(evidence),
            BackupError::CommitRolledBack { .. } => AppError::new(ErrorCode::BackupFailed, reason)
                .with_action("Inspect the filesystem error and retry when the volume is healthy.")
                .retryable(),
            BackupError::InvalidServerDirectory(_)
            | BackupError::WorldNotFound(_)
            | BackupError::InvalidWorldName(_)
            | BackupError::UnsafeFilesystemEntry(_)
            | BackupError::NoWorlds
            | BackupError::AlreadyExists(_)
            | BackupError::NotFound(_) => AppError::new(ErrorCode::InvalidInput, reason)
                .with_action("Check the instance world directories and backup ID, then retry."),
            BackupError::SourceChanged(_) => AppError::new(ErrorCode::BackupFailed, reason)
                .with_action("Ensure no process is changing world files, then retry.")
                .retryable(),
            BackupError::Io { .. }
            | BackupError::Serialization(_)
            | BackupError::WorkerFailed(_) => AppError::new(ErrorCode::BackupFailed, reason)
                .with_action("Check the instance volume and daemon logs, then retry if safe.")
                .with_evidence(evidence),
        }
    }
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
