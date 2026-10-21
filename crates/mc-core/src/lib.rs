//! Domain types and transport-neutral management contracts.

mod api;
mod backup;
mod doctor;
mod domain;
mod error;
mod error_message;
mod lifecycle;
mod management;
pub use management::InstanceOperation;
mod modpack;
mod provider;
mod server_properties;
mod task;

pub use api::{
    ApiRequest, ApiRequestEnvelope, ApiResponse, ApiResponseEnvelope, ServerInfo, API_PROTOCOL_MAX,
    API_PROTOCOL_MIN, API_PROTOCOL_VERSION, RUNNER_PROTOCOL_VERSION, STATE_SCHEMA_VERSION,
};
pub use backup::{
    BackupConsistency, BackupCursor, BackupPage, BackupRestorePlan, BackupSummary,
    MAX_BACKUP_PAGE_LIMIT,
};
pub use doctor::{DoctorCheck, DoctorCheckStatus, DoctorReport, JavaRuntime};
pub use domain::{
    minecraft_java_requirement, minecraft_version_key, ArtifactPlan, BuildInfo, Checksum,
    ChecksumAlgorithm, InstanceDetail, InstanceSummary, ReleaseChannel, ServerLogChunk,
    ServerSnapshot, ServerState, ServerVersion,
};
pub use error::{AppError, ErrorCode};
pub use error_message::{ErrorMessage, ErrorTemplates, Language};
pub use lifecycle::StoppedInstanceLease;
pub use modpack::{
    ModpackAnalysisReport, ModpackFileDisposition, ModpackFilePlan, ModpackIssue, ModpackLoader,
    ModpackSeverity, ModpackStatus,
};
pub use provider::ServerProvider;
pub use server_properties::{ServerDifficulty, ServerPropertiesSnapshot, ServerPropertiesValues};
pub use task::{
    TaskEvent, TaskKind, TaskPhase, TaskProgress, TaskResult, TaskSnapshot, TaskStatus,
    TransferProgress,
};
