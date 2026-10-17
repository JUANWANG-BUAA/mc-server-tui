use serde::{Deserialize, Serialize};

use crate::{
    AppError, ArtifactPlan, BackupCursor, BuildInfo, DoctorReport, InstanceSummary, ServerVersion,
    TaskEvent, TaskSnapshot,
};

pub const API_PROTOCOL_VERSION: u16 = 7;

/// Versioned request envelope shared by CLI, TUI, and the local daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRequestEnvelope {
    pub protocol_version: u16,
    pub payload: ApiRequest,
}

impl ApiRequestEnvelope {
    pub fn current(payload: ApiRequest) -> Self {
        Self {
            protocol_version: API_PROTOCOL_VERSION,
            payload,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponseEnvelope {
    pub protocol_version: u16,
    pub payload: ApiResponse,
}

impl ApiResponseEnvelope {
    pub fn current(payload: ApiResponse) -> Self {
        Self {
            protocol_version: API_PROTOCOL_VERSION,
            payload,
        }
    }
}

/// Transport-neutral operation payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation", content = "arguments", rename_all = "snake_case")]
pub enum ApiRequest {
    Ping,
    Doctor,
    PaperVersions,
    PaperBuilds {
        game_version: String,
    },
    PaperPlan {
        game_version: String,
        build_id: Option<u64>,
    },
    ListInstances,
    GetInstanceDetail {
        instance_id: uuid::Uuid,
    },
    CreatePaperInstance {
        name: String,
        game_version: String,
        build_id: Option<u64>,
        java_path: std::path::PathBuf,
        min_memory_mb: u32,
        max_memory_mb: u32,
        port: u16,
        accept_eula: bool,
    },
    CreateWorldBackup {
        instance_id: uuid::Uuid,
    },
    ListWorldBackups {
        instance_id: uuid::Uuid,
        limit: u16,
        cursor: Option<BackupCursor>,
    },
    VerifyWorldBackup {
        instance_id: uuid::Uuid,
        backup_id: uuid::Uuid,
    },
    PlanWorldBackupRestore {
        instance_id: uuid::Uuid,
        backup_id: uuid::Uuid,
    },
    RestoreWorldBackup {
        instance_id: uuid::Uuid,
        backup_id: uuid::Uuid,
        confirmation_token: String,
    },
    StartServer {
        instance_id: uuid::Uuid,
    },
    StopServer {
        instance_id: uuid::Uuid,
        timeout_seconds: u16,
    },
    SendConsoleCommand {
        instance_id: uuid::Uuid,
        command: String,
    },
    ReadServerLogs {
        instance_id: uuid::Uuid,
        after_offset: u64,
        max_bytes: u32,
    },
    ListTasks {
        limit: u16,
    },
    GetTask {
        task_id: uuid::Uuid,
    },
    ListTaskEvents {
        task_id: uuid::Uuid,
        after_sequence: i64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "result", content = "data", rename_all = "snake_case")]
pub enum ApiResponse {
    Pong,
    Doctor(DoctorReport),
    PaperVersions(Vec<ServerVersion>),
    PaperBuilds(Vec<BuildInfo>),
    ArtifactPlan(Box<ArtifactPlan>),
    Instances(Vec<InstanceSummary>),
    InstanceDetail(crate::InstanceDetail),
    ServerStatus(crate::ServerSnapshot),
    ServerLogs(crate::ServerLogChunk),
    ConsoleCommandAccepted,
    TaskQueued(TaskSnapshot),
    Tasks(Vec<TaskSnapshot>),
    Task(TaskSnapshot),
    TaskEvents(Vec<TaskEvent>),
    Error(AppError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_envelope_carries_protocol_version() {
        let envelope = ApiRequestEnvelope::current(ApiRequest::Ping);
        let encoded = serde_json::to_string(&envelope).expect("serialize request");
        let decoded: ApiRequestEnvelope =
            serde_json::from_str(&encoded).expect("deserialize request");

        assert_eq!(decoded.protocol_version, API_PROTOCOL_VERSION);
        assert!(matches!(decoded.payload, ApiRequest::Ping));
    }

    #[test]
    fn world_backup_request_round_trips_through_the_versioned_protocol() {
        let instance_id = uuid::Uuid::new_v4();
        let envelope = ApiRequestEnvelope::current(ApiRequest::CreateWorldBackup { instance_id });
        let encoded = serde_json::to_string(&envelope).expect("serialize backup request");
        let decoded: ApiRequestEnvelope =
            serde_json::from_str(&encoded).expect("deserialize backup request");

        assert_eq!(decoded.protocol_version, API_PROTOCOL_VERSION);
        assert!(matches!(
            decoded.payload,
            ApiRequest::CreateWorldBackup { instance_id: decoded_id } if decoded_id == instance_id
        ));
    }

    #[test]
    fn backup_listing_and_verification_requests_round_trip() {
        let instance_id = uuid::Uuid::new_v4();
        let backup_id = uuid::Uuid::new_v4();
        let cursor = BackupCursor {
            created_at: chrono::Utc::now(),
            id: backup_id,
        };
        let requests = [
            ApiRequest::ListWorldBackups {
                instance_id,
                limit: 7,
                cursor: Some(cursor.clone()),
            },
            ApiRequest::VerifyWorldBackup {
                instance_id,
                backup_id,
            },
        ];
        for request in requests {
            let envelope = ApiRequestEnvelope::current(request);
            let encoded = serde_json::to_string(&envelope).expect("serialize request");
            let decoded: ApiRequestEnvelope =
                serde_json::from_str(&encoded).expect("deserialize request");
            assert_eq!(decoded.protocol_version, API_PROTOCOL_VERSION);
            match decoded.payload {
                ApiRequest::ListWorldBackups {
                    instance_id: decoded_instance_id,
                    limit,
                    cursor: Some(decoded_cursor),
                } => {
                    assert_eq!(decoded_instance_id, instance_id);
                    assert_eq!(limit, 7);
                    assert_eq!(decoded_cursor, cursor);
                }
                ApiRequest::VerifyWorldBackup {
                    instance_id: decoded_instance_id,
                    backup_id: decoded_backup_id,
                } => {
                    assert_eq!(decoded_instance_id, instance_id);
                    assert_eq!(decoded_backup_id, backup_id);
                }
                _ => panic!("decoded a different backup request"),
            }
        }
    }

    #[test]
    fn restore_plan_and_confirm_requests_round_trip() {
        let instance_id = uuid::Uuid::new_v4();
        let backup_id = uuid::Uuid::new_v4();
        let token = format!("v1:{}", "b".repeat(64));
        for request in [
            ApiRequest::PlanWorldBackupRestore {
                instance_id,
                backup_id,
            },
            ApiRequest::RestoreWorldBackup {
                instance_id,
                backup_id,
                confirmation_token: token.clone(),
            },
        ] {
            let envelope = ApiRequestEnvelope::current(request);
            let encoded = serde_json::to_string(&envelope).expect("serialize request");
            let decoded: ApiRequestEnvelope =
                serde_json::from_str(&encoded).expect("deserialize request");
            assert_eq!(decoded.protocol_version, API_PROTOCOL_VERSION);
            match decoded.payload {
                ApiRequest::PlanWorldBackupRestore {
                    instance_id: decoded_instance,
                    backup_id: decoded_backup,
                } => {
                    assert_eq!(decoded_instance, instance_id);
                    assert_eq!(decoded_backup, backup_id);
                }
                ApiRequest::RestoreWorldBackup {
                    instance_id: decoded_instance,
                    backup_id: decoded_backup,
                    confirmation_token: decoded_token,
                } => {
                    assert_eq!(decoded_instance, instance_id);
                    assert_eq!(decoded_backup, backup_id);
                    assert_eq!(decoded_token, token);
                }
                _ => panic!("decoded a different restore request"),
            }
        }
    }
}
