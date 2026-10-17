use std::{fs, path::Path};

use mc_core::InstanceSummary;
use uuid::Uuid;

use crate::{
    error::{io_error, BackupError},
    tree::sync_directory,
    worlds::{ensure_real_directory, world_candidates},
};

use super::journal::{
    has_operation_marker, read_latest_journal, read_marker, write_journal, RestoreJournal,
    RestorePhase, OPERATION_PREFIX,
};

const MAX_RECOVERY_OPERATIONS: usize = 64;

pub(super) fn recover_pending(instance: &InstanceSummary) -> Result<(), BackupError> {
    let server_dir = instance.directory.join("server");
    ensure_real_directory(&server_dir)?;
    let entries = fs::read_dir(&server_dir)
        .map_err(|error| io_error("Could not scan for restore journals", &server_dir, error))?;
    let mut operation_dirs = Vec::new();
    for entry in entries {
        let entry = entry
            .map_err(|error| io_error("Could not scan for restore journals", &server_dir, error))?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let Some(id_text) = name.strip_prefix(OPERATION_PREFIX) else {
            continue;
        };
        let Ok(operation_id) = Uuid::parse_str(id_text) else {
            continue;
        };
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| io_error("Could not inspect a restore operation", &path, error))?;
        if metadata.file_type().is_dir() && has_operation_marker(&path)? {
            operation_dirs.push((operation_id, path));
        }
    }
    if operation_dirs.len() > MAX_RECOVERY_OPERATIONS {
        return Err(BackupError::RestoreRecoveryRequired(server_dir));
    }

    for (operation_id, operation_dir) in operation_dirs {
        let marker = read_marker(&operation_dir)?;
        if marker.operation_id != operation_id || marker.instance_id != instance.id {
            return Err(BackupError::RestoreRecoveryRequired(operation_dir));
        }
        match read_latest_journal(&operation_dir, operation_id, instance.id)? {
            None => remove_operation(&operation_dir, &server_dir)?,
            Some(journal) => {
                validate_journal_roots(instance, &journal, &operation_dir)?;
                if journal.phase == RestorePhase::Committed {
                    remove_operation(&operation_dir, &server_dir)?;
                } else {
                    let mut journal = journal;
                    journal.phase = RestorePhase::RollingBack;
                    rollback(&operation_dir, &server_dir, &mut journal)?;
                }
            }
        }
    }
    Ok(())
}

pub(super) fn has_pending_restore(instance: &InstanceSummary) -> Result<bool, BackupError> {
    let server_dir = instance.directory.join("server");
    ensure_real_directory(&server_dir)?;
    let entries = fs::read_dir(&server_dir)
        .map_err(|error| io_error("Could not scan for restore journals", &server_dir, error))?;
    for entry in entries {
        let entry = entry
            .map_err(|error| io_error("Could not scan for restore journals", &server_dir, error))?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let Some(id_text) = name.strip_prefix(OPERATION_PREFIX) else {
            continue;
        };
        if Uuid::parse_str(id_text).is_err() {
            continue;
        }
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| io_error("Could not inspect a restore operation", &path, error))?;
        if metadata.file_type().is_dir() && has_operation_marker(&path)? {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn rollback(
    operation_dir: &Path,
    server_dir: &Path,
    journal: &mut RestoreJournal,
) -> Result<(), BackupError> {
    journal.phase = RestorePhase::RollingBack;
    let _ = write_journal(operation_dir, journal);
    rollback_roots(operation_dir, server_dir, journal)?;
    remove_operation(operation_dir, server_dir)
}

fn rollback_roots(
    operation_dir: &Path,
    server_dir: &Path,
    journal: &RestoreJournal,
) -> Result<(), BackupError> {
    let rollback_dir = operation_dir.join("rollback");
    ensure_real_directory(&rollback_dir)?;
    let affected = journal
        .original_worlds
        .iter()
        .chain(journal.target_worlds.iter())
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();

    for world in affected {
        let live = server_dir.join(&world);
        let saved = rollback_dir.join(&world);
        let was_original = journal.original_worlds.contains(&world);
        if was_original && path_exists(&saved)? {
            ensure_real_directory(&saved)?;
            remove_directory_if_present(&live)?;
            rename_path(
                &saved,
                &live,
                "Could not restore a world from rollback storage",
            )?;
            sync_directory(&rollback_dir)?;
            sync_directory(server_dir)?;
        } else if was_original {
            if !path_exists(&live)? {
                return Err(BackupError::RestoreRecoveryRequired(
                    operation_dir.to_path_buf(),
                ));
            }
            ensure_real_directory(&live)?;
        } else {
            if path_exists(&saved)? {
                return Err(BackupError::RestoreRecoveryRequired(
                    operation_dir.to_path_buf(),
                ));
            }
            remove_directory_if_present(&live)?;
        }
    }
    sync_directory(server_dir)?;
    Ok(())
}

fn validate_journal_roots(
    instance: &InstanceSummary,
    journal: &RestoreJournal,
    operation_dir: &Path,
) -> Result<(), BackupError> {
    let server_dir = instance.directory.join("server");
    let configured_primary = crate::worlds::configured_level_name(&server_dir)?;
    let candidates = world_candidates(&configured_primary);
    if journal
        .original_worlds
        .iter()
        .chain(journal.target_worlds.iter())
        .any(|world| !candidates.contains(world))
    {
        return Err(BackupError::RestoreRecoveryRequired(
            operation_dir.to_path_buf(),
        ));
    }
    Ok(())
}

pub(super) fn path_exists(path: &Path) -> Result<bool, BackupError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(io_error("Could not inspect a world path", path, error)),
    }
}

fn remove_directory_if_present(path: &Path) -> Result<(), BackupError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() => fs::remove_dir_all(path)
            .map_err(|error| io_error("Could not remove a replaced world directory", path, error)),
        Ok(_) => Err(BackupError::UnsafeFilesystemEntry(path.to_path_buf())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error("Could not inspect a world path", path, error)),
    }
}

pub(super) fn remove_operation(operation_dir: &Path, server_dir: &Path) -> Result<(), BackupError> {
    fs::remove_dir_all(operation_dir).map_err(|error| {
        io_error(
            "Could not remove restore transaction data",
            operation_dir,
            error,
        )
    })?;
    sync_directory(server_dir)
}

pub(super) fn cleanup_unjournaled(operation_dir: &Path, server_dir: &Path) {
    if fs::remove_dir_all(operation_dir).is_ok() {
        let _ = sync_directory(server_dir);
    }
}

fn rename_path(from: &Path, to: &Path, operation: &'static str) -> Result<(), BackupError> {
    fs::rename(from, to).map_err(|error| io_error(operation, to, error))
}
