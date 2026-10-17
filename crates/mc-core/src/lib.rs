//! Domain types and transport-neutral management contracts.

mod api;
mod backup;
mod doctor;
mod domain;
mod error;
mod lifecycle;
mod provider;
mod task;

pub use api::{
    ApiRequest, ApiRequestEnvelope, ApiResponse, ApiResponseEnvelope, API_PROTOCOL_VERSION,
};
pub use backup::{
    BackupConsistency, BackupCursor, BackupPage, BackupSummary, MAX_BACKUP_PAGE_LIMIT,
};
pub use doctor::{DoctorCheck, DoctorCheckStatus, DoctorReport, JavaRuntime};
pub use domain::{
    ArtifactPlan, BuildInfo, Checksum, ChecksumAlgorithm, InstanceDetail, InstanceSummary,
    ReleaseChannel, ServerLogChunk, ServerSnapshot, ServerState, ServerVersion,
};
pub use error::{AppError, ErrorCode};
pub use lifecycle::StoppedInstanceLease;
pub use provider::ServerProvider;
pub use task::{
    TaskEvent, TaskKind, TaskPhase, TaskProgress, TaskResult, TaskSnapshot, TaskStatus,
};
