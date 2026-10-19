use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{AppError, BackupPage, BackupRestorePlan, BackupSummary, InstanceSummary};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    ManageInstance,
    CreatePaperInstance,
    CreateWorldBackup,
    ListWorldBackups,
    VerifyWorldBackup,
    PlanWorldBackupRestore,
    RestoreWorldBackup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Interrupted,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPhase {
    ManagingInstance,
    Queued,
    WaitingForInstance,
    ResolvingArtifact,
    DownloadingArtifact,
    InstallingInstance,
    BackingUpWorld,
    ListingBackups,
    VerifyingBackup,
    PlanningBackupRestore,
    RestoringWorldBackup,
    Completed,
    Failed,
    Interrupted,
    Cancelled,
}

/// Byte-level transfer progress for long downloads. Older persisted tasks
/// deserialize with `None`, so the field is optional and backward-compatible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub bytes_per_second: Option<u64>,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskProgress {
    pub completed_steps: u16,
    pub total_steps: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transfer: Option<TransferProgress>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum TaskResult {
    ManagementCompleted {
        instance_id: Uuid,
        report: String,
    },
    InstanceCreated(InstanceSummary),
    WorldBackupCreated {
        instance_id: Uuid,
        backup_id: Uuid,
    },
    WorldBackupsListed {
        instance_id: Uuid,
        page: BackupPage,
    },
    WorldBackupVerified {
        instance_id: Uuid,
        backup: BackupSummary,
    },
    WorldBackupRestorePlanned {
        instance_id: Uuid,
        plan: BackupRestorePlan,
    },
    WorldBackupRestored {
        instance_id: Uuid,
        backup_id: Uuid,
    },
}

impl TaskResult {
    pub fn instance_id(&self) -> Uuid {
        match self {
            Self::ManagementCompleted { instance_id, .. } => *instance_id,
            Self::InstanceCreated(instance) => instance.id,
            Self::WorldBackupCreated { instance_id, .. } => *instance_id,
            Self::WorldBackupsListed { instance_id, .. }
            | Self::WorldBackupVerified { instance_id, .. }
            | Self::WorldBackupRestorePlanned { instance_id, .. }
            | Self::WorldBackupRestored { instance_id, .. } => *instance_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskSnapshot {
    pub id: Uuid,
    pub kind: TaskKind,
    pub instance_id: Option<Uuid>,
    pub status: TaskStatus,
    pub phase: TaskPhase,
    pub message: String,
    pub progress: Option<TaskProgress>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub result: Option<TaskResult>,
    pub error: Option<AppError>,
    pub last_event_sequence: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskEvent {
    pub sequence: i64,
    pub task_id: Uuid,
    pub occurred_at: DateTime<Utc>,
    pub status: TaskStatus,
    pub phase: TaskPhase,
    pub message: String,
    pub progress: Option<TaskProgress>,
    pub error: Option<AppError>,
}
