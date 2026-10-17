use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Stable machine-readable error categories shared by every front end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    InvalidInput,
    UserAgentRequired,
    SourceUnavailable,
    SourceRateLimited,
    SourceSchemaChanged,
    NoStableBuild,
    ServerArtifactMissing,
    DownloadInterrupted,
    DownloadTooLarge,
    HashMismatch,
    DiskSpaceLow,
    RedirectBlocked,
    RequestTooLarge,
    RequestTimeout,
    ProtocolInvalid,
    TaskQueueFull,
    TaskNotFound,
    TaskInterrupted,
    InstanceBusy,
    InstanceNotFound,
    ServerStartFailed,
    ServerStopTimeout,
    ServerStatusUnknown,
    ServerNotRunning,
    InvalidConsoleCommand,
    Internal,
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = match self {
            Self::InvalidInput => "INVALID_INPUT",
            Self::UserAgentRequired => "USER_AGENT_REQUIRED",
            Self::SourceUnavailable => "SOURCE_UNAVAILABLE",
            Self::SourceRateLimited => "SOURCE_RATE_LIMITED",
            Self::SourceSchemaChanged => "SOURCE_SCHEMA_CHANGED",
            Self::NoStableBuild => "NO_STABLE_BUILD",
            Self::ServerArtifactMissing => "SERVER_ARTIFACT_MISSING",
            Self::DownloadInterrupted => "DOWNLOAD_INTERRUPTED",
            Self::DownloadTooLarge => "DOWNLOAD_TOO_LARGE",
            Self::HashMismatch => "HASH_MISMATCH",
            Self::DiskSpaceLow => "DISK_SPACE_LOW",
            Self::RedirectBlocked => "REDIRECT_BLOCKED",
            Self::RequestTooLarge => "REQUEST_TOO_LARGE",
            Self::RequestTimeout => "REQUEST_TIMEOUT",
            Self::ProtocolInvalid => "PROTOCOL_INVALID",
            Self::TaskQueueFull => "TASK_QUEUE_FULL",
            Self::TaskNotFound => "TASK_NOT_FOUND",
            Self::TaskInterrupted => "TASK_INTERRUPTED",
            Self::InstanceBusy => "INSTANCE_BUSY",
            Self::InstanceNotFound => "INSTANCE_NOT_FOUND",
            Self::ServerStartFailed => "SERVER_START_FAILED",
            Self::ServerStopTimeout => "SERVER_STOP_TIMEOUT",
            Self::ServerStatusUnknown => "SERVER_STATUS_UNKNOWN",
            Self::ServerNotRunning => "SERVER_NOT_RUNNING",
            Self::InvalidConsoleCommand => "INVALID_CONSOLE_COMMAND",
            Self::Internal => "INTERNAL",
        };
        formatter.write_str(code)
    }
}

/// An operation error safe to return over the local management API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
#[error("{reason}")]
pub struct AppError {
    pub code: ErrorCode,
    pub reason: String,
    pub next_action: Option<String>,
    pub evidence: Option<String>,
    pub retryable: bool,
}

impl AppError {
    pub fn new(code: ErrorCode, reason: impl Into<String>) -> Self {
        Self {
            code,
            reason: reason.into(),
            next_action: None,
            evidence: None,
            retryable: false,
        }
    }

    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.next_action = Some(action.into());
        self
    }

    pub fn with_evidence(mut self, evidence: impl Into<String>) -> Self {
        self.evidence = Some(evidence.into());
        self
    }

    pub fn retryable(mut self) -> Self {
        self.retryable = true;
        self
    }
}
