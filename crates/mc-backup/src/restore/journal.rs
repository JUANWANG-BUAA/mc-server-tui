use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{io_error, BackupError},
    manifest::is_safe_world_name,
    tree::sync_directory,
};

pub(super) const OPERATION_PREFIX: &str = ".mc-restore-";
const MARKER_FILE: &str = "restore-operation.json";
const JOURNAL_PREFIX: &str = "journal-";
pub(super) const JOURNAL_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum RestorePhase {
    Prepared,
    Swapping,
    Swapped,
    RollingBack,
    Committed,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OperationMarker {
    pub(super) schema_version: u32,
    pub(super) operation_id: Uuid,
    pub(super) instance_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RestoreJournal {
    pub(super) schema_version: u32,
    pub(super) sequence: u64,
    pub(super) operation_id: Uuid,
    pub(super) instance_id: Uuid,
    pub(super) backup_id: Uuid,
    pub(super) phase: RestorePhase,
    pub(super) original_worlds: Vec<String>,
    pub(super) target_worlds: Vec<String>,
}

pub(super) fn write_marker(
    operation_dir: &Path,
    operation_id: Uuid,
    instance_id: Uuid,
) -> Result<(), BackupError> {
    let marker = OperationMarker {
        schema_version: JOURNAL_SCHEMA_VERSION,
        operation_id,
        instance_id,
    };
    let path = operation_dir.join(MARKER_FILE);
    write_new_json(
        &path,
        &marker,
        "Could not create a restore operation marker",
    )?;
    sync_directory(operation_dir)
}

pub(super) fn has_operation_marker(operation_dir: &Path) -> Result<bool, BackupError> {
    let path = operation_dir.join(MARKER_FILE);
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(true),
        Ok(_) => Err(BackupError::RestoreRecoveryRequired(
            operation_dir.to_path_buf(),
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(io_error(
            "Could not inspect a restore operation marker",
            path,
            error,
        )),
    }
}

pub(super) fn read_marker(operation_dir: &Path) -> Result<OperationMarker, BackupError> {
    let path = operation_dir.join(MARKER_FILE);
    let metadata = fs::symlink_metadata(&path)
        .map_err(|error| io_error("Could not inspect a restore operation marker", &path, error))?;
    if !metadata.file_type().is_file() || metadata.len() > 4096 {
        return Err(BackupError::RestoreRecoveryRequired(
            operation_dir.to_path_buf(),
        ));
    }
    let bytes = fs::read(&path)
        .map_err(|error| io_error("Could not read a restore operation marker", &path, error))?;
    let marker: OperationMarker = serde_json::from_slice(&bytes)
        .map_err(|_| BackupError::RestoreRecoveryRequired(operation_dir.to_path_buf()))?;
    if marker.schema_version != JOURNAL_SCHEMA_VERSION {
        return Err(BackupError::RestoreRecoveryRequired(
            operation_dir.to_path_buf(),
        ));
    }
    Ok(marker)
}

pub(super) fn write_journal(
    operation_dir: &Path,
    journal: &mut RestoreJournal,
) -> Result<(), BackupError> {
    journal.sequence = journal.sequence.checked_add(1).ok_or_else(|| {
        BackupError::InvalidManifest("restore journal sequence overflow".to_owned())
    })?;
    let path = journal_path(operation_dir, journal.sequence);
    write_new_json(&path, journal, "Could not persist a restore journal")?;
    sync_directory(operation_dir)
}

fn write_new_json<T: Serialize>(
    path: &Path,
    value: &T,
    operation: &'static str,
) -> Result<(), BackupError> {
    let contents = serde_json::to_vec_pretty(value)?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|error| io_error(operation, path, error))?;
    file.write_all(&contents)
        .map_err(|error| io_error(operation, path, error))?;
    file.sync_all()
        .map_err(|error| io_error(operation, path, error))
}

pub(super) fn read_latest_journal(
    operation_dir: &Path,
    operation_id: Uuid,
    instance_id: Uuid,
) -> Result<Option<RestoreJournal>, BackupError> {
    let entries = fs::read_dir(operation_dir)
        .map_err(|error| io_error("Could not list restore journal files", operation_dir, error))?;
    let mut journals = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            io_error("Could not list restore journal files", operation_dir, error)
        })?;
        let Some(sequence) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.strip_prefix(JOURNAL_PREFIX))
            .and_then(|name| name.strip_suffix(".json"))
            .and_then(|value| value.parse::<u64>().ok())
        else {
            continue;
        };
        journals.push((sequence, entry.path()));
    }
    journals.sort_by_key(|(sequence, _)| *sequence);
    let Some((sequence, path)) = journals.pop() else {
        return Ok(None);
    };
    let metadata = fs::symlink_metadata(&path)
        .map_err(|error| io_error("Could not inspect a restore journal", &path, error))?;
    if !metadata.file_type().is_file() || metadata.len() > 1024 * 1024 {
        return Err(BackupError::RestoreRecoveryRequired(
            operation_dir.to_path_buf(),
        ));
    }
    let bytes = fs::read(&path)
        .map_err(|error| io_error("Could not read a restore journal", &path, error))?;
    let journal: RestoreJournal = serde_json::from_slice(&bytes)
        .map_err(|_| BackupError::RestoreRecoveryRequired(operation_dir.to_path_buf()))?;
    validate_journal(&journal, operation_id, instance_id, operation_dir)?;
    if sequence == 0 || journal.sequence != sequence {
        return Err(BackupError::RestoreRecoveryRequired(
            operation_dir.to_path_buf(),
        ));
    }
    Ok(Some(journal))
}

pub(super) fn validate_journal(
    journal: &RestoreJournal,
    operation_id: Uuid,
    instance_id: Uuid,
    operation_dir: &Path,
) -> Result<(), BackupError> {
    if journal.schema_version != JOURNAL_SCHEMA_VERSION
        || journal.operation_id != operation_id
        || journal.instance_id != instance_id
        || !strictly_sorted_safe(&journal.original_worlds)
        || !strictly_sorted_safe(&journal.target_worlds)
        || journal.target_worlds.is_empty()
    {
        return Err(BackupError::RestoreRecoveryRequired(
            operation_dir.to_path_buf(),
        ));
    }
    Ok(())
}

fn strictly_sorted_safe(worlds: &[String]) -> bool {
    worlds.windows(2).all(|pair| pair[0] < pair[1])
        && worlds.iter().all(|world| is_safe_world_name(world))
}

fn journal_path(operation_dir: &Path, sequence: u64) -> PathBuf {
    operation_dir.join(format!("{JOURNAL_PREFIX}{sequence:020}.json"))
}
