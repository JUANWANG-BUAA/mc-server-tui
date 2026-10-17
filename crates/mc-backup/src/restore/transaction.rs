use std::{fs, path::Path};

use mc_core::InstanceSummary;
use uuid::Uuid;

use crate::{
    error::{io_error, BackupError},
    manifest::BackupManifest,
    store::BackupService,
    tree::{copy_worlds, create_private_directory, scan_backup_tree, scan_worlds, sync_directory},
    worlds::{ensure_real_directory, inspect_existing_worlds},
};

use super::{
    journal::{
        validate_journal, write_journal, write_marker, RestoreJournal, RestorePhase,
        JOURNAL_SCHEMA_VERSION, OPERATION_PREFIX,
    },
    recovery::{cleanup_unjournaled, path_exists, remove_operation, rollback},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RestoreStep {
    Prepared,
    OldRootMoved(String),
    NewRootInstalled(String),
    Swapped,
    Verified,
}

pub(super) fn execute_restore<F>(
    instance: &InstanceSummary,
    manifest: &BackupManifest,
    confirmation_token: &str,
    after_step: &mut F,
) -> Result<(), BackupError>
where
    F: FnMut(RestoreStep) -> Result<(), BackupError>,
{
    let server_dir = instance.directory.join("server");
    ensure_real_directory(&server_dir)?;
    let required_bytes = super::plan::required_restore_space(manifest)?;
    let available_bytes = fs2::available_space(&server_dir)
        .map_err(|error| io_error("Could not inspect free restore space", &server_dir, error))?;
    if required_bytes > available_bytes {
        return Err(BackupError::RestoreInsufficientSpace {
            needed: required_bytes,
            available: available_bytes,
        });
    }
    let operation_id = choose_operation_id(&server_dir, &manifest.worlds)?;
    let operation_dir = server_dir.join(format!("{OPERATION_PREFIX}{operation_id}"));
    create_private_directory(&operation_dir)?;

    let preparation = prepare_operation(
        instance,
        manifest,
        confirmation_token,
        operation_id,
        &operation_dir,
        &server_dir,
    );
    let mut journal = match preparation {
        Ok(journal) => journal,
        Err(error) => {
            cleanup_unjournaled(&operation_dir, &server_dir);
            return Err(error);
        }
    };

    if let Err(error) = after_step(RestoreStep::Prepared) {
        return finish_failed_transaction(&operation_dir, &server_dir, &mut journal, error);
    }

    let transaction = swap_and_verify(
        manifest,
        &operation_dir,
        &server_dir,
        &mut journal,
        after_step,
    );
    if let Err(error) = transaction {
        return finish_failed_transaction(&operation_dir, &server_dir, &mut journal, error);
    }
    let _ = remove_operation(&operation_dir, &server_dir);
    Ok(())
}

fn prepare_operation(
    instance: &InstanceSummary,
    manifest: &BackupManifest,
    confirmation_token: &str,
    operation_id: Uuid,
    operation_dir: &Path,
    server_dir: &Path,
) -> Result<RestoreJournal, BackupError> {
    write_marker(operation_dir, operation_id, instance.id)?;
    let staging_dir = operation_dir.join("staging");
    let rollback_dir = operation_dir.join("rollback");
    create_private_directory(&staging_dir)?;
    create_private_directory(&rollback_dir)?;

    let backup_dir = instance
        .directory
        .join("backups")
        .join(manifest.id.to_string());
    let staged_entries = copy_worlds(&backup_dir, &staging_dir, &manifest.worlds)?;
    if staged_entries != manifest.entries || scan_backup_tree(&staging_dir)? != manifest.entries {
        return Err(BackupError::IntegrityMismatch(backup_dir));
    }
    sync_directory(&staging_dir)?;
    sync_directory(&rollback_dir)?;
    sync_directory(operation_dir)?;
    sync_directory(server_dir)?;

    let latest_manifest = BackupService::read_manifest_blocking(instance, manifest.id)?;
    let current_worlds = inspect_existing_worlds(server_dir)?;
    let current_entries = if current_worlds.is_empty() {
        Vec::new()
    } else {
        scan_worlds(server_dir, &current_worlds)?
    };
    let available_space_bytes = fs2::available_space(server_dir)
        .map_err(|error| io_error("Could not inspect free restore space", server_dir, error))?;
    let current_plan = super::plan::make_restore_plan(
        instance,
        &latest_manifest,
        current_worlds,
        current_entries,
        available_space_bytes,
    )?;
    if current_plan.confirmation_token != confirmation_token {
        return Err(BackupError::RestorePlanChanged);
    }
    if latest_manifest != *manifest {
        return Err(BackupError::RestorePlanChanged);
    }

    let mut journal = RestoreJournal {
        schema_version: JOURNAL_SCHEMA_VERSION,
        sequence: 0,
        operation_id,
        instance_id: instance.id,
        backup_id: manifest.id,
        phase: RestorePhase::Prepared,
        original_worlds: current_plan.current_worlds,
        target_worlds: manifest.worlds.clone(),
    };
    validate_journal(&journal, operation_id, instance.id, operation_dir)?;
    write_journal(operation_dir, &mut journal)?;
    Ok(journal)
}

fn swap_and_verify<F>(
    manifest: &BackupManifest,
    operation_dir: &Path,
    server_dir: &Path,
    journal: &mut RestoreJournal,
    after_step: &mut F,
) -> Result<(), BackupError>
where
    F: FnMut(RestoreStep) -> Result<(), BackupError>,
{
    journal.phase = RestorePhase::Swapping;
    write_journal(operation_dir, journal)?;
    let rollback_dir = operation_dir.join("rollback");
    let staging_dir = operation_dir.join("staging");

    for world in &journal.original_worlds {
        let live = server_dir.join(world);
        let rollback = rollback_dir.join(world);
        ensure_real_directory(&live)?;
        rename_path(
            &live,
            &rollback,
            "Could not move a live world into rollback storage",
        )?;
        sync_directory(server_dir)?;
        sync_directory(&rollback_dir)?;
        after_step(RestoreStep::OldRootMoved(world.clone()))?;
    }

    for world in &journal.target_worlds {
        let staged = staging_dir.join(world);
        let live = server_dir.join(world);
        ensure_real_directory(&staged)?;
        rename_path(&staged, &live, "Could not install a staged world")?;
        sync_directory(&staging_dir)?;
        sync_directory(server_dir)?;
        after_step(RestoreStep::NewRootInstalled(world.clone()))?;
    }

    journal.phase = RestorePhase::Swapped;
    write_journal(operation_dir, journal)?;
    after_step(RestoreStep::Swapped)?;

    let actual_entries = scan_worlds(server_dir, &manifest.worlds)?;
    if actual_entries != manifest.entries {
        return Err(BackupError::IntegrityMismatch(server_dir.to_path_buf()));
    }
    for removed in journal
        .original_worlds
        .iter()
        .filter(|world| !journal.target_worlds.contains(world))
    {
        if path_exists(&server_dir.join(removed))? {
            return Err(BackupError::SourceChanged(server_dir.join(removed)));
        }
    }
    sync_directory(server_dir)?;
    after_step(RestoreStep::Verified)?;

    journal.phase = RestorePhase::Committed;
    write_journal(operation_dir, journal)?;
    let _ = sync_directory(&operation_dir.join("rollback"));
    Ok(())
}

fn finish_failed_transaction(
    operation_dir: &Path,
    server_dir: &Path,
    journal: &mut RestoreJournal,
    cause: BackupError,
) -> Result<(), BackupError> {
    match rollback(operation_dir, server_dir, journal) {
        Ok(()) => Err(cause),
        Err(rollback_error) => {
            let detail = format!("{cause}; rollback failed: {rollback_error}");
            Err(BackupError::RestoreRollbackFailed {
                operation_dir: operation_dir.to_path_buf(),
                detail,
            })
        }
    }
}

fn choose_operation_id(server_dir: &Path, target_worlds: &[String]) -> Result<Uuid, BackupError> {
    for _ in 0..8 {
        let id = Uuid::new_v4();
        let name = format!("{OPERATION_PREFIX}{id}");
        if target_worlds.iter().any(|world| world == &name) {
            continue;
        }
        let path = server_dir.join(name);
        match fs::symlink_metadata(&path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(id),
            Ok(_) => continue,
            Err(error) => {
                return Err(io_error(
                    "Could not inspect a restore operation path",
                    path,
                    error,
                ));
            }
        }
    }
    Err(BackupError::InvalidManifest(
        "could not reserve a unique restore operation directory".to_owned(),
    ))
}

fn rename_path(from: &Path, to: &Path, operation: &'static str) -> Result<(), BackupError> {
    fs::rename(from, to).map_err(|error| io_error(operation, to, error))
}
