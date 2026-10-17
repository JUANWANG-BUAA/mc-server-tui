use std::path::{Component, Path};

use chrono::{DateTime, Utc};
use mc_core::BackupConsistency;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::BackupError;

pub const BACKUP_SCHEMA_VERSION: u32 = 1;
pub const BACKUP_MANIFEST_FILE: &str = "manifest.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupEntryKind {
    Directory,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupEntry {
    /// UTF-8 path relative to the server directory, including its world root.
    pub path: String,
    pub kind: BackupEntryKind,
    pub size_bytes: u64,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupManifest {
    pub schema_version: u32,
    pub id: Uuid,
    pub instance_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub consistency: BackupConsistency,
    pub worlds: Vec<String>,
    pub entries: Vec<BackupEntry>,
}

impl BackupManifest {
    pub(crate) fn validate(
        &self,
        expected_instance_id: Uuid,
        expected_backup_id: Uuid,
    ) -> Result<(), BackupError> {
        if self.schema_version != BACKUP_SCHEMA_VERSION
            || self.id != expected_backup_id
            || self.instance_id != expected_instance_id
            || self.worlds.is_empty()
            || !strictly_sorted(&self.worlds)
            || self.worlds.iter().any(|world| !is_safe_world_name(world))
        {
            return Err(BackupError::InvalidManifest(
                "schema, identity, or world list is invalid".to_owned(),
            ));
        }

        let world_roots = self
            .worlds
            .iter()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        let mut previous_path: Option<&str> = None;
        let mut root_entries = std::collections::BTreeSet::new();
        for entry in &self.entries {
            if previous_path.is_some_and(|previous| previous >= entry.path.as_str())
                || !is_safe_relative_path(&entry.path)
            {
                return Err(BackupError::InvalidManifest(
                    "entry paths are unsafe, duplicated, or unsorted".to_owned(),
                ));
            }
            previous_path = Some(&entry.path);

            let root = entry
                .path
                .split('/')
                .next()
                .ok_or_else(|| BackupError::InvalidManifest("empty entry path".to_owned()))?;
            if !world_roots.contains(root) {
                return Err(BackupError::InvalidManifest(format!(
                    "entry is outside a listed world: {}",
                    entry.path
                )));
            }
            match entry.kind {
                BackupEntryKind::Directory => {
                    if entry.size_bytes != 0 || entry.sha256.is_some() {
                        return Err(BackupError::InvalidManifest(format!(
                            "directory metadata is invalid: {}",
                            entry.path
                        )));
                    }
                    if entry.path == root {
                        root_entries.insert(root.to_owned());
                    }
                }
                BackupEntryKind::File => {
                    if !entry.sha256.as_deref().is_some_and(is_sha256) {
                        return Err(BackupError::InvalidManifest(format!(
                            "file checksum is invalid: {}",
                            entry.path
                        )));
                    }
                }
            }
        }
        if root_entries.len() != self.worlds.len() {
            return Err(BackupError::InvalidManifest(
                "each world must have a root directory entry".to_owned(),
            ));
        }
        Ok(())
    }
}

pub(crate) fn is_safe_world_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains(['/', '\\', ':'])
        && name != BACKUP_MANIFEST_FILE
        && !name.chars().any(char::is_control)
        && Path::new(name).components().collect::<Vec<_>>().as_slice()
            == [Component::Normal(std::ffi::OsStr::new(name))]
}

fn is_safe_relative_path(path: &str) -> bool {
    !path.is_empty()
        && path != BACKUP_MANIFEST_FILE
        && !path.contains('\\')
        && path.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && !part.contains(':')
                && !part.chars().any(char::is_control)
        })
        && Path::new(path)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
        && Path::new(path).to_str() == Some(path)
}

fn strictly_sorted(values: &[String]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
