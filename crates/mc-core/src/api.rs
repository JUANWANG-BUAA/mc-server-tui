use serde::{Deserialize, Serialize};

use crate::{
    AppError, ArtifactPlan, BuildInfo, DoctorReport, InstanceSummary, ServerVersion, TaskEvent,
    TaskSnapshot,
};

pub const API_PROTOCOL_VERSION: u16 = 3;

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
}
