use std::collections::BTreeSet;

use mc_core::{BackupRestorePlan, InstanceSummary};
use serde::Serialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    error::{io_error, BackupError},
    manifest::{is_safe_world_name, BackupEntry, BackupManifest},
    store::BackupService,
    tree::scan_worlds,
    worlds::{configured_level_name, inspect_existing_worlds, world_candidates},
};

const RESTORE_SPACE_SAFETY_MARGIN_BYTES: u64 = 1024 * 1024;

#[derive(Serialize)]
struct ConfirmationPayload<'a> {
    instance_id: Uuid,
    backup_id: Uuid,
    manifest: &'a BackupManifest,
    current_worlds: &'a [String],
    current_entries: &'a [BackupEntry],
    replaced_worlds: &'a [String],
    added_worlds: &'a [String],
    removed_worlds: &'a [String],
}

pub(super) fn build_restore_plan(
    instance: &InstanceSummary,
    backup_id: Uuid,
) -> Result<BackupRestorePlan, BackupError> {
    let manifest = BackupService::verify_blocking(instance, backup_id)?;
    let server_dir = instance.directory.join("server");
    let current_worlds = inspect_existing_worlds(&server_dir)?;
    let current_entries = if current_worlds.is_empty() {
        Vec::new()
    } else {
        scan_worlds(&server_dir, &current_worlds)?
    };
    let server_dir = instance.directory.join("server");
    make_restore_plan(
        instance,
        &manifest,
        current_worlds,
        current_entries,
        fs2::available_space(&server_dir).map_err(|error| {
            io_error("Could not inspect free restore space", &server_dir, error)
        })?,
    )
}

pub(super) fn make_restore_plan(
    instance: &InstanceSummary,
    manifest: &BackupManifest,
    current_worlds: Vec<String>,
    current_entries: Vec<BackupEntry>,
    available_space_bytes: u64,
) -> Result<BackupRestorePlan, BackupError> {
    let server_dir = instance.directory.join("server");
    let configured_primary = configured_level_name(&server_dir)?;
    if !is_safe_world_name(&configured_primary) {
        return Err(BackupError::InvalidWorldName(configured_primary));
    }
    let candidates = world_candidates(&configured_primary);
    let candidate_set = candidates
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if !manifest
        .worlds
        .iter()
        .any(|world| world == &configured_primary)
        || manifest
            .worlds
            .iter()
            .any(|world| !candidate_set.contains(world.as_str()))
    {
        return Err(BackupError::RestorePrimaryWorldMismatch {
            configured: configured_primary,
            backup_worlds: manifest.worlds.clone(),
        });
    }

    let current_set = current_worlds
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let target_set = manifest
        .worlds
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let replaced_worlds = manifest
        .worlds
        .iter()
        .filter(|world| current_set.contains(world.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let added_worlds = manifest
        .worlds
        .iter()
        .filter(|world| !current_set.contains(world.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let removed_worlds = current_worlds
        .iter()
        .filter(|world| !target_set.contains(world.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let required_space_bytes = required_restore_space(manifest)?;

    let payload = ConfirmationPayload {
        instance_id: instance.id,
        backup_id: manifest.id,
        manifest,
        current_worlds: &current_worlds,
        current_entries: &current_entries,
        replaced_worlds: &replaced_worlds,
        added_worlds: &added_worlds,
        removed_worlds: &removed_worlds,
    };
    let encoded = serde_json::to_vec(&payload)?;
    let digest = Sha256::digest(encoded);

    Ok(BackupRestorePlan {
        instance_id: instance.id,
        backup_id: manifest.id,
        created_at: manifest.created_at,
        current_worlds,
        target_worlds: manifest.worlds.clone(),
        replaced_worlds,
        added_worlds,
        removed_worlds,
        required_space_bytes,
        available_space_bytes,
        confirmation_token: format!("v1:{digest:x}"),
    })
}

pub(super) fn required_restore_space(manifest: &BackupManifest) -> Result<u64, BackupError> {
    let payload_bytes = manifest
        .entries
        .iter()
        .filter(|entry| entry.kind == crate::manifest::BackupEntryKind::File)
        .try_fold(0_u64, |total, entry| total.checked_add(entry.size_bytes))
        .ok_or_else(|| BackupError::InvalidManifest("restore size overflow".to_owned()))?;
    payload_bytes
        .checked_add(RESTORE_SPACE_SAFETY_MARGIN_BYTES)
        .ok_or_else(|| BackupError::InvalidManifest("restore size overflow".to_owned()))
}
