use crate::error_message::{ErrorMessage, ErrorTemplates, Language};
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
    TaskCancelled,
    BackupFailed,
    BackupIntegrityFailed,
    BackupRestorePlanChanged,
    BackupRestorePrimaryMismatch,
    BackupRestoreRecoveryRequired,
    BackupRestoreRollbackFailed,
    InstanceBusy,
    InstanceNotFound,
    ConcurrentModification,
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
            Self::TaskCancelled => "TASK_CANCELLED",
            Self::BackupFailed => "BACKUP_FAILED",
            Self::BackupIntegrityFailed => "BACKUP_INTEGRITY_FAILED",
            Self::BackupRestorePlanChanged => "BACKUP_RESTORE_PLAN_CHANGED",
            Self::BackupRestorePrimaryMismatch => "BACKUP_RESTORE_PRIMARY_MISMATCH",
            Self::BackupRestoreRecoveryRequired => "BACKUP_RESTORE_RECOVERY_REQUIRED",
            Self::BackupRestoreRollbackFailed => "BACKUP_RESTORE_ROLLBACK_FAILED",
            Self::InstanceBusy => "INSTANCE_BUSY",
            Self::InstanceNotFound => "INSTANCE_NOT_FOUND",
            Self::ConcurrentModification => "CONCURRENT_MODIFICATION",
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

    /// Convert this AppError to a user-friendly ErrorMessage.
    pub fn to_error_message(&self, lang: Language) -> ErrorMessage {
        // Try to match known error patterns and use templates
        let template_msg = self.try_template_message(lang);
        if let Some(msg) = template_msg {
            return msg;
        }

        // Fall back to generic conversion
        let mut msg = ErrorMessage::new(
            self.format_summary(lang),
            self.reason.clone(),
        );

        if let Some(evidence) = &self.evidence {
            msg = msg.with_evidence(evidence.clone());
        }

        if let Some(action) = &self.next_action {
            msg = msg.with_action(action.clone());
        }

        if self.retryable {
            msg = msg.retryable();
        }

        msg
    }

    fn format_summary(&self, lang: Language) -> String {
        match lang {
            Language::En => match self.code {
                ErrorCode::DiskSpaceLow => "Insufficient disk space".to_string(),
                ErrorCode::SourceUnavailable => "Download source unavailable".to_string(),
                ErrorCode::SourceRateLimited => "Rate limited by download source".to_string(),
                ErrorCode::DownloadInterrupted => "Download interrupted".to_string(),
                ErrorCode::HashMismatch => "File integrity check failed".to_string(),
                ErrorCode::ServerStartFailed => "Server failed to start".to_string(),
                ErrorCode::BackupFailed => "Backup operation failed".to_string(),
                ErrorCode::BackupIntegrityFailed => "Backup integrity check failed".to_string(),
                ErrorCode::ServerStopTimeout => "Server stop operation timed out".to_string(),
                ErrorCode::InvalidInput => "Invalid input or configuration".to_string(),
                _ => "An error occurred".to_string(),
            },
            Language::ZhCN => match self.code {
                ErrorCode::DiskSpaceLow => "磁盘空间不足".to_string(),
                ErrorCode::SourceUnavailable => "下载源不可用".to_string(),
                ErrorCode::SourceRateLimited => "下载源限流".to_string(),
                ErrorCode::DownloadInterrupted => "下载中断".to_string(),
                ErrorCode::HashMismatch => "文件完整性检查失败".to_string(),
                ErrorCode::ServerStartFailed => "服务器启动失败".to_string(),
                ErrorCode::BackupFailed => "备份操作失败".to_string(),
                ErrorCode::BackupIntegrityFailed => "备份完整性检查失败".to_string(),
                ErrorCode::ServerStopTimeout => "服务器停止超时".to_string(),
                ErrorCode::InvalidInput => "输入或配置无效".to_string(),
                _ => "发生错误".to_string(),
            },
        }
    }

    fn try_template_message(&self, lang: Language) -> Option<ErrorMessage> {
        // Try to extract specific parameters and use templates
        match self.code {
            ErrorCode::DiskSpaceLow => {
                // Try to parse "need X bytes, have Y" from reason
                if let Some((needed, available)) = self.parse_disk_space() {
                    return Some(ErrorTemplates::disk_space_low(needed, available, lang));
                }
            }
            ErrorCode::SourceUnavailable | ErrorCode::DownloadInterrupted => {
                if let Some(url) = self.extract_url() {
                    return Some(ErrorTemplates::download_failed(&url, &self.reason, lang));
                }
            }
            ErrorCode::HashMismatch => {
                if let Some((expected, actual)) = self.parse_hashes() {
                    return Some(ErrorTemplates::hash_mismatch(&expected, &actual, lang));
                }
            }
            ErrorCode::ServerStartFailed => {
                return Some(ErrorTemplates::server_start_failed(&self.reason, lang));
            }
            ErrorCode::BackupFailed | ErrorCode::BackupIntegrityFailed => {
                return Some(ErrorTemplates::backup_failed(&self.reason, lang));
            }
            _ => {}
        }
        None
    }

    fn parse_disk_space(&self) -> Option<(u64, u64)> {
        // Try to parse "need X bytes, have Y" pattern
        let evidence = self.evidence.as_ref()?;
        let parts: Vec<&str> = evidence.split(',').collect();
        if parts.len() != 2 {
            return None;
        }

        let needed = parts[0].trim().strip_prefix("need ")?.strip_suffix(" bytes")?;
        let available = parts[1].trim().strip_prefix("have ")?.strip_suffix(" bytes")?;

        let needed: u64 = needed.parse().ok()?;
        let available: u64 = available.parse().ok()?;

        Some((needed, available))
    }

    fn extract_url(&self) -> Option<String> {
        self.evidence
            .as_ref()
            .and_then(|e| e.strip_prefix("url="))
            .map(|url| url.split(',').next().unwrap_or(url).to_string())
    }

    fn parse_hashes(&self) -> Option<(String, String)> {
        let evidence = self.evidence.as_ref()?;
        let parts: Vec<&str> = evidence.split(", ").collect();
        if parts.len() != 2 {
            return None;
        }

        let expected = parts[0].strip_prefix("expected_hash=")?.to_string();
        let actual = parts[1].strip_prefix("actual_hash=")?.to_string();

        Some((expected, actual))
    }
}
