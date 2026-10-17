use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const MAX_BACKUP_PAGE_LIMIT: u16 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupConsistency {
    ServerStopped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupSummary {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub consistency: BackupConsistency,
    pub file_count: u64,
    pub total_bytes: u64,
}

/// Stable pagination position sorted newest-first by creation time and backup ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupCursor {
    pub created_at: DateTime<Utc>,
    pub id: Uuid,
}

impl fmt::Display for BackupCursor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}~{}", self.created_at.to_rfc3339(), self.id)
    }
}

impl FromStr for BackupCursor {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (created_at, id) = value
            .split_once('~')
            .ok_or_else(|| "cursor must be <RFC3339 timestamp>~<backup UUID>".to_owned())?;
        let created_at = DateTime::parse_from_rfc3339(created_at)
            .map_err(|error| format!("invalid cursor timestamp: {error}"))?
            .with_timezone(&Utc);
        let id =
            Uuid::parse_str(id).map_err(|error| format!("invalid cursor backup ID: {error}"))?;
        Ok(Self { created_at, id })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupPage {
    pub backups: Vec<BackupSummary>,
    pub next_cursor: Option<BackupCursor>,
}

/// Preview of a stopped-world restore, including every recognized root it
/// would replace, add, or remove. The confirmation token binds this preview to
/// the verified backup and the current world contents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupRestorePlan {
    pub instance_id: Uuid,
    pub backup_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub current_worlds: Vec<String>,
    pub target_worlds: Vec<String>,
    pub replaced_worlds: Vec<String>,
    pub added_worlds: Vec<String>,
    pub removed_worlds: Vec<String>,
    pub required_space_bytes: u64,
    pub available_space_bytes: u64,
    pub confirmation_token: String,
}
