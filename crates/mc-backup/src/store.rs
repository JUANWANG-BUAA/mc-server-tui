use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use chrono::Utc;
use mc_core::{BackupConsistency, InstanceSummary, StoppedInstanceLease};
use uuid::Uuid;

use crate::{
    error::{io_error, BackupError},
    manifest::{BackupManifest, BACKUP_MANIFEST_FILE, BACKUP_SCHEMA_VERSION},
    tree::{
        copy_worlds, create_private_directory, measure_worlds, scan_backup_tree, sync_directory,
    },
    worlds::{discover_worlds, ensure_real_directory},
};

const MANIFEST_MAX_BYTES: u64 = 16 * 1024 * 1024;
const SPACE_SAFETY_MARGIN_BYTES: u64 = 1024 * 1024;

/// Creates and verifies local world snapshots.
///
/// File operations run on Tokio's blocking pool. Backup creation owns a
/// [`StoppedInstanceLease`] for the full copy, so cancellation of the async
/// caller cannot release the lifecycle reservation while filesystem work is
/// still running.
#[derive(Debug, Default, Clone, Copy)]
pub struct BackupService;

impl BackupService {
    /// Copies all recognized world roots into an immutable, checksummed backup.
    pub async fn create<G>(&self, guard: G, backup_id: Uuid) -> Result<BackupManifest, BackupError>
    where
        G: StoppedInstanceLease,
    {
        tokio::task::spawn_blocking(move || {
            let instance = guard.instance();
            Self::create_blocking(instance.id, &instance.directory, backup_id)
        })
        .await
        .map_err(|error| BackupError::WorkerFailed(error.to_string()))?
    }

    pub(super) fn create_blocking(
        instance_id: Uuid,
        instance_dir: &Path,
        backup_id: Uuid,
    ) -> Result<BackupManifest, BackupError> {
        let server_dir = instance_dir.join("server");
        ensure_real_directory(&server_dir)?;
        ensure_real_directory(instance_dir)?;
        let worlds = discover_worlds(&server_dir)?;
        let payload_bytes = measure_worlds(&server_dir, &worlds)?;
        let backup_root = instance_dir.join("backups");
        ensure_private_directory(&backup_root)?;
        sync_directory(instance_dir)?;
        let required_bytes = payload_bytes
            .checked_add(SPACE_SAFETY_MARGIN_BYTES)
            .ok_or_else(|| BackupError::InvalidManifest("required space overflow".to_owned()))?;
        let available_bytes = fs2::available_space(&backup_root).map_err(|error| {
            io_error("Could not inspect free backup space", &backup_root, error)
        })?;
        if required_bytes > available_bytes {
            return Err(BackupError::InsufficientSpace {
                needed: required_bytes,
                available: available_bytes,
            });
        }

        let destination = backup_root.join(backup_id.to_string());
        match fs::symlink_metadata(&destination) {
            Ok(_) => return Err(BackupError::AlreadyExists(backup_id)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(io_error(
                    "Could not inspect the backup destination",
                    &destination,
                    error,
                ));
            }
        }
        let staging_path = backup_root.join(format!(".staging-{backup_id}"));
        create_private_directory(&staging_path)?;
        let mut staging = StagingDirectory::new(staging_path.clone());

        let entries = copy_worlds(&server_dir, &staging_path, &worlds)?;
        let copied_bytes = entries
            .iter()
            .filter(|entry| entry.kind == crate::manifest::BackupEntryKind::File)
            .try_fold(0_u64, |total, entry| total.checked_add(entry.size_bytes))
            .ok_or_else(|| BackupError::InvalidManifest("copied size overflow".to_owned()))?;
        if copied_bytes != payload_bytes {
            return Err(BackupError::SourceChanged(server_dir));
        }

        let manifest = BackupManifest {
            schema_version: BACKUP_SCHEMA_VERSION,
            id: backup_id,
            instance_id,
            created_at: Utc::now(),
            consistency: BackupConsistency::ServerStopped,
            worlds,
            entries,
        };
        manifest.validate(instance_id, backup_id)?;
        write_manifest(&staging_path.join(BACKUP_MANIFEST_FILE), &manifest)?;
        if scan_backup_tree(&staging_path)? != manifest.entries {
            return Err(BackupError::IntegrityMismatch(staging_path));
        }
        sync_directory(&staging_path)?;
        fs::rename(&staging_path, &destination).map_err(|error| {
            io_error("Could not commit the completed backup", &destination, error)
        })?;
        staging.committed = true;
        if let Err(error) = sync_directory(&backup_root) {
            let sync_detail = error.to_string();
            let cleanup_error = match fs::remove_dir_all(&destination) {
                Ok(()) => sync_directory(&backup_root)
                    .err()
                    .map(|error| error.to_string()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    sync_directory(&backup_root)
                        .err()
                        .map(|error| error.to_string())
                }
                Err(error) => Some(error.to_string()),
            };
            if let Some(cleanup_error) = cleanup_error {
                return Err(BackupError::CommitUncertain {
                    backup_id,
                    path: destination,
                    detail: format!(
                        "{sync_detail}; cleanup could not be confirmed: {cleanup_error}"
                    ),
                });
            }
            return Err(BackupError::CommitRolledBack {
                backup_id,
                detail: sync_detail,
            });
        }
        Ok(manifest)
    }

    /// Rehashes a backup and rejects missing, extra, or changed files.
    pub async fn verify(
        &self,
        instance: &InstanceSummary,
        backup_id: Uuid,
    ) -> Result<BackupManifest, BackupError> {
        let instance = instance.clone();
        tokio::task::spawn_blocking(move || Self::verify_blocking(&instance, backup_id))
            .await
            .map_err(|error| BackupError::WorkerFailed(error.to_string()))?
    }

    pub(super) fn verify_blocking(
        instance: &InstanceSummary,
        backup_id: Uuid,
    ) -> Result<BackupManifest, BackupError> {
        let manifest = Self::read_manifest_blocking(instance, backup_id)?;
        let backup_dir = instance
            .directory
            .join("backups")
            .join(backup_id.to_string());
        let actual_entries = scan_backup_tree(&backup_dir)?;
        if actual_entries != manifest.entries {
            return Err(BackupError::IntegrityMismatch(backup_dir));
        }
        Ok(manifest)
    }

    pub(super) fn read_manifest_blocking(
        instance: &InstanceSummary,
        backup_id: Uuid,
    ) -> Result<BackupManifest, BackupError> {
        ensure_real_directory(&instance.directory)?;
        let backup_root = instance.directory.join("backups");
        match fs::symlink_metadata(&backup_root) {
            Ok(metadata) if metadata.file_type().is_dir() => {}
            Ok(_) => return Err(BackupError::UnsafeFilesystemEntry(backup_root)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(BackupError::NotFound(backup_id));
            }
            Err(error) => {
                return Err(io_error(
                    "Could not inspect backup storage",
                    &backup_root,
                    error,
                ));
            }
        }
        let backup_dir = backup_root.join(backup_id.to_string());
        let metadata = match fs::symlink_metadata(&backup_dir) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(BackupError::NotFound(backup_id));
            }
            Err(error) => {
                return Err(io_error("Could not inspect a backup", &backup_dir, error));
            }
        };
        if !metadata.file_type().is_dir() {
            return Err(BackupError::UnsafeFilesystemEntry(backup_dir));
        }
        let manifest_path = backup_dir.join(BACKUP_MANIFEST_FILE);
        let manifest_metadata = fs::symlink_metadata(&manifest_path).map_err(|error| {
            io_error("Could not inspect a backup manifest", &manifest_path, error)
        })?;
        if !manifest_metadata.file_type().is_file() || manifest_metadata.len() > MANIFEST_MAX_BYTES
        {
            return Err(BackupError::InvalidManifest(
                "manifest is not a bounded regular file".to_owned(),
            ));
        }
        let bytes = fs::read(&manifest_path)
            .map_err(|error| io_error("Could not read a backup manifest", &manifest_path, error))?;
        let manifest: BackupManifest = serde_json::from_slice(&bytes)?;
        manifest.validate(instance.id, backup_id)?;
        Ok(manifest)
    }
}

fn ensure_private_directory(path: &Path) -> Result<(), BackupError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() => {}
        Ok(_) => return Err(BackupError::UnsafeFilesystemEntry(path.to_path_buf())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            create_private_directory(path)?;
            return Ok(());
        }
        Err(error) => return Err(io_error("Could not inspect backup storage", path, error)),
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|error| io_error("Could not protect backup storage", path, error))?;
    }
    Ok(())
}

fn write_manifest(path: &Path, manifest: &BackupManifest) -> Result<(), BackupError> {
    let contents = serde_json::to_vec_pretty(manifest)?;
    if contents.len() as u64 > MANIFEST_MAX_BYTES {
        return Err(BackupError::InvalidManifest(
            "manifest exceeds the supported size limit".to_owned(),
        ));
    }
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|error| io_error("Could not create a backup manifest", path, error))?;
    file.write_all(&contents)
        .map_err(|error| io_error("Could not write a backup manifest", path, error))?;
    file.sync_all()
        .map_err(|error| io_error("Could not sync a backup manifest", path, error))
}

struct StagingDirectory {
    path: PathBuf,
    committed: bool,
}

impl StagingDirectory {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            committed: false,
        }
    }
}

impl Drop for StagingDirectory {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
