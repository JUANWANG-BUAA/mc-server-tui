//! Domain types and transport-neutral management contracts.

mod api;
mod doctor;
mod domain;
mod error;
mod provider;
mod task;

pub use api::{
    ApiRequest, ApiRequestEnvelope, ApiResponse, ApiResponseEnvelope, API_PROTOCOL_VERSION,
};
pub use doctor::{DoctorCheck, DoctorCheckStatus, DoctorReport, JavaRuntime};
pub use domain::{
    ArtifactPlan, BuildInfo, Checksum, ChecksumAlgorithm, InstanceDetail, InstanceSummary,
    ReleaseChannel, ServerLogChunk, ServerSnapshot, ServerState, ServerVersion,
};
pub use error::{AppError, ErrorCode};
pub use provider::ServerProvider;
pub use task::{
    TaskEvent, TaskKind, TaskPhase, TaskProgress, TaskResult, TaskSnapshot, TaskStatus,
};
