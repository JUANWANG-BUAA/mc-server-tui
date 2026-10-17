use std::fs;

use mc_core::{BackupCursor, BackupPage, BackupSummary, InstanceSummary, MAX_BACKUP_PAGE_LIMIT};
use uuid::Uuid;

use crate::{
    error::{io_error, BackupError},
    manifest::{BackupEntryKind, BackupManifest},
    store::BackupService,
    worlds::ensure_real_directory,
};

const BACKUP_CATALOG_LIMIT: usize = 1_000;

impl BackupService {
    /// Lists valid backups newest-first. Any corrupt backup fails the listing
    /// rather than silently disappearing from the user's history.
    pub async fn list(
        &self,
        instance: &InstanceSummary,
    ) -> Result<Vec<BackupManifest>, BackupError> {
        let instance = instance.clone();
        tokio::task::spawn_blocking(move || list_all_blocking(&instance))
            .await
            .map_err(|error| BackupError::WorkerFailed(error.to_string()))?
    }

    /// Lists one bounded page. Candidate manifests are read for ordering, while
    /// only summaries returned in this page are fully rehashed.
    pub async fn list_page(
        &self,
        instance: &InstanceSummary,
        limit: u16,
        cursor: Option<BackupCursor>,
    ) -> Result<BackupPage, BackupError> {
        if !(1..=MAX_BACKUP_PAGE_LIMIT).contains(&limit) {
            return Err(BackupError::InvalidPageLimit {
                limit,
                maximum: MAX_BACKUP_PAGE_LIMIT,
            });
        }
        let instance = instance.clone();
        tokio::task::spawn_blocking(move || list_page_blocking(&instance, limit, cursor))
            .await
            .map_err(|error| BackupError::WorkerFailed(error.to_string()))?
    }

    /// Verifies a backup in the blocking pool and returns only bounded metadata.
    pub async fn verify_summary(
        &self,
        instance: &InstanceSummary,
        backup_id: Uuid,
    ) -> Result<BackupSummary, BackupError> {
        let instance = instance.clone();
        tokio::task::spawn_blocking(move || {
            let manifest = Self::verify_blocking(&instance, backup_id)?;
            summarize(&manifest)
        })
        .await
        .map_err(|error| BackupError::WorkerFailed(error.to_string()))?
    }
}

fn list_page_blocking(
    instance: &InstanceSummary,
    limit: u16,
    cursor: Option<BackupCursor>,
) -> Result<BackupPage, BackupError> {
    let mut candidates = read_catalog_summaries(instance)?;
    candidates.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| right.id.cmp(&left.id))
    });
    let mut eligible = candidates.into_iter().filter(|summary| {
        cursor.as_ref().is_none_or(|cursor| {
            summary.created_at < cursor.created_at
                || (summary.created_at == cursor.created_at && summary.id < cursor.id)
        })
    });
    let mut selected = eligible
        .by_ref()
        .take(usize::from(limit) + 1)
        .collect::<Vec<_>>();
    let has_more = selected.len() > usize::from(limit);
    if has_more {
        selected.pop();
    }

    let mut verified = Vec::with_capacity(selected.len());
    for candidate in selected {
        let manifest = BackupService::verify_blocking(instance, candidate.id)?;
        let summary = summarize(&manifest)?;
        if summary.created_at != candidate.created_at
            || summary.consistency != candidate.consistency
        {
            return Err(BackupError::SourceChanged(
                instance
                    .directory
                    .join("backups")
                    .join(candidate.id.to_string()),
            ));
        }
        verified.push(summary);
    }
    let next_cursor = if has_more {
        verified.last().map(|summary| BackupCursor {
            created_at: summary.created_at,
            id: summary.id,
        })
    } else {
        None
    };
    Ok(BackupPage {
        backups: verified,
        next_cursor,
    })
}

fn read_catalog_summaries(instance: &InstanceSummary) -> Result<Vec<BackupSummary>, BackupError> {
    read_backup_ids(instance, Some(BACKUP_CATALOG_LIMIT))?
        .into_iter()
        .map(|id| {
            BackupService::read_manifest_blocking(instance, id)
                .and_then(|manifest| summarize(&manifest))
        })
        .collect()
}

fn list_all_blocking(instance: &InstanceSummary) -> Result<Vec<BackupManifest>, BackupError> {
    let mut manifests = read_backup_ids(instance, None)?
        .into_iter()
        .map(|id| BackupService::verify_blocking(instance, id))
        .collect::<Result<Vec<_>, _>>()?;
    manifests.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| right.id.cmp(&left.id))
    });
    Ok(manifests)
}

fn read_backup_ids(
    instance: &InstanceSummary,
    catalog_limit: Option<usize>,
) -> Result<Vec<Uuid>, BackupError> {
    ensure_real_directory(&instance.directory)?;
    let backup_root = instance.directory.join("backups");
    let metadata = match fs::symlink_metadata(&backup_root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(io_error(
                "Could not inspect backup storage",
                &backup_root,
                error,
            ));
        }
    };
    if !metadata.file_type().is_dir() {
        return Err(BackupError::UnsafeFilesystemEntry(backup_root));
    }

    let entries = fs::read_dir(&backup_root)
        .map_err(|error| io_error("Could not list backups", &backup_root, error))?;
    let mut backup_ids = Vec::new();
    for (index, entry) in entries.enumerate() {
        let entry =
            entry.map_err(|error| io_error("Could not list backups", &backup_root, error))?;
        let scanned_entries = index + 1;
        if let Some(limit) = catalog_limit.filter(|limit| scanned_entries > *limit) {
            return Err(BackupError::CatalogTooLarge { maximum: limit });
        }
        let name = entry
            .file_name()
            .to_str()
            .ok_or_else(|| BackupError::UnsafeFilesystemEntry(entry.path()))?
            .to_owned();
        if name.starts_with(".staging-") {
            if !entry
                .file_type()
                .map_err(|error| {
                    io_error("Could not inspect a staging backup", entry.path(), error)
                })?
                .is_dir()
            {
                return Err(BackupError::UnsafeFilesystemEntry(entry.path()));
            }
            continue;
        }
        let id =
            Uuid::parse_str(&name).map_err(|_| BackupError::UnsafeFilesystemEntry(entry.path()))?;
        if id.to_string() != name {
            return Err(BackupError::UnsafeFilesystemEntry(entry.path()));
        }
        backup_ids.push(id);
    }
    Ok(backup_ids)
}

fn summarize(manifest: &BackupManifest) -> Result<BackupSummary, BackupError> {
    let mut file_count = 0_u64;
    let mut total_bytes = 0_u64;
    for entry in &manifest.entries {
        if entry.kind == BackupEntryKind::File {
            file_count = file_count
                .checked_add(1)
                .ok_or_else(|| BackupError::InvalidManifest("file count overflow".to_owned()))?;
            total_bytes = total_bytes
                .checked_add(entry.size_bytes)
                .ok_or_else(|| BackupError::InvalidManifest("backup size overflow".to_owned()))?;
        }
    }
    Ok(BackupSummary {
        id: manifest.id,
        created_at: manifest.created_at,
        consistency: manifest.consistency,
        file_count,
        total_bytes,
    })
}
