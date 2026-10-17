use std::{
    fs::OpenOptions,
    io,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};
use fs2::FileExt;
use mc_core::{AppError, ErrorCode, InstanceSummary, ServerLogChunk};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use uuid::Uuid;

pub(crate) const RUNTIME_SCHEMA_VERSION: u32 = 2;
const MAX_RUNTIME_BYTES: u64 = 64 * 1024;
const LOG_TAIL_BYTES: u64 = 256 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RuntimeRecord {
    pub schema_version: u32,
    pub instance_id: Uuid,
    /// PID of the resident supervisor runner, never the Java child.
    pub pid: u32,
    /// PID of Java when the runner has finished starting it.
    #[serde(default)]
    pub java_pid: Option<u32>,
    pub marker: String,
    pub started_at: DateTime<Utc>,
    pub stopping: bool,
    pub last_exit_code: Option<i32>,
    #[serde(default)]
    pub last_error: Option<String>,
}

pub(crate) async fn acquire_lifecycle_lock(
    instance: &InstanceSummary,
) -> Result<std::fs::File, AppError> {
    let path = lifecycle_lock_path(instance);
    tokio::task::spawn_blocking(move || {
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).mode(0o600);
        options.custom_flags(libc::O_NOFOLLOW);
        let file = options
            .open(path)
            .map_err(|error| io_error("Could not open the instance lifecycle lock.", error))?;
        let metadata = file
            .metadata()
            .map_err(|error| io_error("Could not inspect the instance lifecycle lock.", error))?;
        if !metadata.file_type().is_file() {
            return Err(AppError::new(
                ErrorCode::Internal,
                "The instance lifecycle lock is not a regular file.",
            ));
        }
        file.set_permissions(std::fs::Permissions::from_mode(0o600))
            .map_err(|error| io_error("Could not protect the lifecycle lock.", error))?;
        match file.try_lock_exclusive() {
            Ok(()) => Ok(file),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => Err(AppError::new(
                ErrorCode::InstanceBusy,
                "Another lifecycle operation is active for this instance.",
            )
            .with_action("Wait for the current operation to finish and retry.")
            .retryable()),
            Err(error) => Err(io_error("Could not lock the instance lifecycle.", error)),
        }
    })
    .await
    .map_err(|error| {
        AppError::new(ErrorCode::Internal, "Lifecycle lock worker failed.")
            .with_evidence(error.to_string())
    })?
}

pub(crate) async fn read_record(
    instance: &InstanceSummary,
) -> Result<Option<RuntimeRecord>, AppError> {
    let path = runtime_path(instance);
    let metadata = match tokio::fs::symlink_metadata(&path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(io_error("Could not inspect server runtime state.", error)),
    };
    if !metadata.file_type().is_file() || metadata.len() > MAX_RUNTIME_BYTES {
        return Err(AppError::new(
            ErrorCode::SourceSchemaChanged,
            "Server runtime state is not a bounded regular file.",
        ));
    }
    let mut options = tokio::fs::OpenOptions::new();
    options.read(true).custom_flags(libc::O_NOFOLLOW);
    let file = options
        .open(path)
        .await
        .map_err(|error| io_error("Could not read server runtime state.", error))?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(MAX_RUNTIME_BYTES + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|error| io_error("Could not read server runtime state.", error))?;
    if bytes.len() as u64 > MAX_RUNTIME_BYTES {
        return Err(AppError::new(
            ErrorCode::RequestTooLarge,
            "Server runtime state exceeds its size limit.",
        ));
    }
    let record: RuntimeRecord = serde_json::from_slice(&bytes).map_err(|error| {
        AppError::new(
            ErrorCode::SourceSchemaChanged,
            "Server runtime state could not be decoded.",
        )
        .with_evidence(error.to_string())
    })?;
    #[cfg(not(test))]
    let expected_marker = format!("--internal-runner={}", instance.id);
    #[cfg(test)]
    let expected_marker = "tests::runner_entry".to_owned();
    if record.schema_version != RUNTIME_SCHEMA_VERSION
        || record.instance_id != instance.id
        || record.pid == 0
        || record.marker != expected_marker
    {
        return Err(AppError::new(
            ErrorCode::SourceSchemaChanged,
            "Server runtime state does not match this instance.",
        ));
    }
    Ok(Some(record))
}

pub(crate) async fn write_record(
    instance: &InstanceSummary,
    record: &RuntimeRecord,
) -> Result<(), AppError> {
    let path = runtime_path(instance);
    let temporary = path.with_extension(format!("{}.tmp", Uuid::new_v4()));
    let encoded = serde_json::to_vec(record).map_err(|error| {
        AppError::new(
            ErrorCode::Internal,
            "Could not encode server runtime state.",
        )
        .with_evidence(error.to_string())
    })?;
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create_new(true);
    options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    let mut file = options
        .open(&temporary)
        .await
        .map_err(|error| io_error("Could not create server runtime state.", error))?;
    use tokio::io::AsyncWriteExt;
    file.write_all(&encoded)
        .await
        .map_err(|error| io_error("Could not write server runtime state.", error))?;
    file.sync_all()
        .await
        .map_err(|error| io_error("Could not sync server runtime state.", error))?;
    drop(file);
    if let Err(error) = tokio::fs::rename(&temporary, &path).await {
        let _ = tokio::fs::remove_file(&temporary).await;
        return Err(io_error(
            "Could not atomically replace server runtime state.",
            error,
        ));
    }
    if let Some(parent) = path.parent() {
        let directory = tokio::fs::File::open(parent)
            .await
            .map_err(|error| io_error("Could not open server state directory.", error))?;
        directory
            .sync_all()
            .await
            .map_err(|error| io_error("Could not sync server state directory.", error))?;
    }
    Ok(())
}

pub(crate) async fn open_console_log(
    instance: &InstanceSummary,
) -> Result<std::fs::File, AppError> {
    let path = console_log_path(instance);
    ensure_regular_or_missing(&path, "Server console log").await?;
    let mut options = tokio::fs::OpenOptions::new();
    options
        .append(true)
        .create(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW);
    let file = options
        .open(path)
        .await
        .map_err(|error| io_error("Could not open server console log.", error))?;
    Ok(file.into_std().await)
}

pub(crate) async fn rotate_console_log(instance: &InstanceSummary) -> Result<(), AppError> {
    let path = console_log_path(instance);
    let rotated = path.with_extension("log.1");
    ensure_regular_or_missing(&path, "Server console log").await?;
    ensure_regular_or_missing(&rotated, "Rotated server console log").await?;
    for old_log in [path, rotated] {
        match tokio::fs::remove_file(old_log).await {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(io_error(
                    "Could not clear previous server console log.",
                    error,
                ))
            }
        }
    }
    Ok(())
}

pub(crate) async fn log_has_ready_marker(
    instance: &InstanceSummary,
    marker: &str,
) -> Result<bool, AppError> {
    for path in [
        console_log_path(instance),
        console_log_path(instance).with_extension("log.1"),
    ] {
        let metadata = match tokio::fs::symlink_metadata(&path).await {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(io_error("Could not inspect server console log.", error)),
        };
        if !metadata.file_type().is_file() {
            return Err(AppError::new(
                ErrorCode::Internal,
                "Server console log path is not a regular file.",
            ));
        }
        let mut options = tokio::fs::OpenOptions::new();
        options.read(true).custom_flags(libc::O_NOFOLLOW);
        let mut file = options
            .open(path)
            .await
            .map_err(|error| io_error("Could not read server console log.", error))?;
        let start = metadata.len().saturating_sub(LOG_TAIL_BYTES);
        file.seek(std::io::SeekFrom::Start(start))
            .await
            .map_err(|error| io_error("Could not seek server console log.", error))?;
        let mut bytes = Vec::with_capacity((metadata.len() - start) as usize);
        file.take(LOG_TAIL_BYTES)
            .read_to_end(&mut bytes)
            .await
            .map_err(|error| io_error("Could not read server console log.", error))?;
        if String::from_utf8_lossy(&bytes).contains(marker) {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(crate) async fn read_logs(
    instance: &InstanceSummary,
    after_offset: u64,
    max_bytes: u32,
) -> Result<ServerLogChunk, AppError> {
    let path = console_log_path(instance);
    let metadata = match tokio::fs::symlink_metadata(&path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(ServerLogChunk {
                instance_id: instance.id,
                start_offset: 0,
                next_offset: 0,
                content: String::new(),
                truncated: false,
            });
        }
        Err(error) => return Err(io_error("Could not inspect server console log.", error)),
    };
    if !metadata.file_type().is_file() {
        return Err(AppError::new(
            ErrorCode::Internal,
            "Server console log path is not a regular file.",
        ));
    }
    let chunk_size = (max_bytes as usize).clamp(1, super::MAX_LOG_CHUNK_BYTES);
    let truncated = after_offset > metadata.len();
    let start = if truncated {
        metadata.len().saturating_sub(chunk_size as u64)
    } else {
        after_offset
    };
    let count = metadata.len().saturating_sub(start).min(chunk_size as u64) as usize;
    let mut options = tokio::fs::OpenOptions::new();
    options.read(true).custom_flags(libc::O_NOFOLLOW);
    let mut file = options
        .open(path)
        .await
        .map_err(|error| io_error("Could not open server console log.", error))?;
    file.seek(std::io::SeekFrom::Start(start))
        .await
        .map_err(|error| io_error("Could not seek server console log.", error))?;
    let mut bytes = vec![0; count];
    let bytes_read = file
        .read(&mut bytes)
        .await
        .map_err(|error| io_error("Could not read server console log.", error))?;
    bytes.truncate(bytes_read);
    Ok(ServerLogChunk {
        instance_id: instance.id,
        start_offset: start,
        next_offset: start + bytes_read as u64,
        content: String::from_utf8_lossy(&bytes).into_owned(),
        truncated,
    })
}

async fn ensure_regular_or_missing(path: &Path, label: &str) -> Result<(), AppError> {
    match tokio::fs::symlink_metadata(path).await {
        Ok(metadata) if metadata.file_type().is_file() => Ok(()),
        Ok(_) => Err(AppError::new(
            ErrorCode::Internal,
            format!("{label} path is not a regular file."),
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error(&format!("Could not inspect {label} path."), error)),
    }
}

fn lifecycle_lock_path(instance: &InstanceSummary) -> PathBuf {
    instance.directory.join("server").join(".lifecycle.lock")
}

fn runtime_path(instance: &InstanceSummary) -> PathBuf {
    instance.directory.join("server").join("supervisor.json")
}

fn console_log_path(instance: &InstanceSummary) -> PathBuf {
    instance.directory.join("logs").join("console.log")
}

fn io_error(reason: &str, error: io::Error) -> AppError {
    AppError::new(ErrorCode::Internal, reason).with_evidence(error.to_string())
}
