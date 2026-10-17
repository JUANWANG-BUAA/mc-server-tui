use chrono::{DateTime, Utc};
use mc_core::{AppError, ArtifactPlan, ErrorCode, InstanceSummary};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

pub const INSTANCE_SCHEMA_VERSION: u32 = 1;
pub const INSTANCE_LOCK_SCHEMA_VERSION: u32 = 1;
const MIN_MEMORY_MB: u32 = 512;
const MAX_MEMORY_MB: u32 = 65_536;

/// User choices captured at instance creation time.
#[derive(Debug, Clone)]
pub struct InstanceSpec {
    pub name: String,
    pub java_path: PathBuf,
    pub min_memory_mb: u32,
    pub max_memory_mb: u32,
    pub port: u16,
    pub accept_eula: bool,
}

impl InstanceSpec {
    pub fn validate_for_creation(&self) -> Result<(), AppError> {
        self.validate_details().map(|_| ())
    }

    pub async fn validate_for_creation_async(&self) -> Result<(), AppError> {
        let spec = self.clone();
        tokio::task::spawn_blocking(move || spec.validate_for_creation())
            .await
            .map_err(|error| {
                AppError::new(ErrorCode::Internal, "Java validation worker failed.")
                    .with_evidence(error.to_string())
            })?
    }

    pub(crate) async fn validate_details_async(&self) -> Result<(String, PathBuf), AppError> {
        let spec = self.clone();
        tokio::task::spawn_blocking(move || spec.validate_details())
            .await
            .map_err(|error| {
                AppError::new(ErrorCode::Internal, "Java validation worker failed.")
                    .with_evidence(error.to_string())
            })?
    }

    pub(crate) fn validate_details(&self) -> Result<(String, PathBuf), AppError> {
        let name = self.name.trim();
        if name.is_empty() || name.len() > 128 || name.chars().any(char::is_control) {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Instance name must contain 1 to 128 visible characters.",
            ));
        }
        if !self.accept_eula {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Minecraft EULA acceptance is required before creating an instance.",
            )
            .with_action("Review the Minecraft EULA and pass the explicit acceptance option."));
        }
        if self.min_memory_mb < MIN_MEMORY_MB
            || self.max_memory_mb < self.min_memory_mb
            || self.max_memory_mb > MAX_MEMORY_MB
        {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Memory must be between 512 and 65536 MiB, with minimum no greater than maximum.",
            ));
        }
        if self.port == 0 || self.port < 1024 {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Instance port must be between 1024 and 65535.",
            ));
        }
        if !self.java_path.is_absolute() {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Java executable path must be absolute.",
            ));
        }
        let java_path = std::fs::canonicalize(&self.java_path).map_err(|error| {
            AppError::new(
                ErrorCode::InvalidInput,
                "Java executable could not be resolved.",
            )
            .with_evidence(error.kind().to_string())
        })?;
        let metadata = std::fs::metadata(&java_path).map_err(|error| {
            AppError::new(ErrorCode::InvalidInput, "Java executable is unavailable.")
                .with_evidence(error.kind().to_string())
        })?;
        if !metadata.is_file() || !is_executable(&metadata) {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Selected Java path is not an executable file.",
            ));
        }
        Ok((name.to_owned(), java_path))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstanceConfig {
    pub schema_version: u32,
    pub id: Uuid,
    pub name: String,
    pub provider: String,
    pub project: String,
    pub game_version: String,
    pub build_id: u64,
    pub java_path: PathBuf,
    pub min_memory_mb: u32,
    pub max_memory_mb: u32,
    pub port: u16,
    pub eula_accepted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl InstanceConfig {
    pub fn summary(&self, directory: PathBuf) -> InstanceSummary {
        InstanceSummary {
            id: self.id,
            name: self.name.clone(),
            provider: self.provider.clone(),
            game_version: self.game_version.clone(),
            build_id: self.build_id,
            java_path: self.java_path.clone(),
            min_memory_mb: self.min_memory_mb,
            max_memory_mb: self.max_memory_mb,
            port: self.port,
            directory,
            created_at: self.created_at,
        }
    }

    pub(crate) fn validate(&self) -> Result<(), AppError> {
        if self.schema_version != INSTANCE_SCHEMA_VERSION
            || self.name.trim().is_empty()
            || self.provider.trim().is_empty()
            || self.project.trim().is_empty()
            || self.game_version.trim().is_empty()
            || self.build_id == 0
            || !self.eula_accepted
            || !self.java_path.is_absolute()
            || self.min_memory_mb < MIN_MEMORY_MB
            || self.max_memory_mb < self.min_memory_mb
            || self.max_memory_mb > MAX_MEMORY_MB
            || self.port < 1024
        {
            return Err(AppError::new(
                ErrorCode::SourceSchemaChanged,
                "Instance configuration is incomplete or uses an unsupported schema.",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstanceLock {
    pub schema_version: u32,
    pub provider: String,
    pub project: String,
    pub game_version: String,
    pub build_id: u64,
    pub channel: String,
    pub metadata_url: String,
    pub artifact_name: String,
    pub artifact_size: u64,
    pub upstream_sha256: String,
    pub local_sha256: String,
    pub resolved_at: DateTime<Utc>,
    pub installed_at: DateTime<Utc>,
}

impl InstanceLock {
    pub(crate) fn from_plan(plan: &ArtifactPlan, local_sha256: String) -> Self {
        Self {
            schema_version: INSTANCE_LOCK_SCHEMA_VERSION,
            provider: plan.provider.clone(),
            project: plan.project.clone(),
            game_version: plan.game_version.clone(),
            build_id: plan.build_id,
            channel: plan.channel.0.clone(),
            metadata_url: plan.metadata_url.to_string(),
            artifact_name: plan.artifact_name.clone(),
            artifact_size: plan.expected_size,
            upstream_sha256: plan.checksum.value.clone(),
            local_sha256,
            resolved_at: plan.resolved_at,
            installed_at: Utc::now(),
        }
    }

    pub(crate) fn validate(&self, config: &InstanceConfig) -> Result<(), AppError> {
        if self.schema_version != INSTANCE_LOCK_SCHEMA_VERSION
            || self.provider != config.provider
            || self.project.trim().is_empty()
            || self.game_version != config.game_version
            || self.build_id != config.build_id
            || !self.metadata_url.starts_with("https://")
            || self.artifact_name.trim().is_empty()
            || self.artifact_size == 0
            || !is_sha256(&self.upstream_sha256)
            || !is_sha256(&self.local_sha256)
        {
            return Err(AppError::new(
                ErrorCode::SourceSchemaChanged,
                "Instance lock file is incomplete or does not match its configuration.",
            ));
        }
        Ok(())
    }
}

pub(crate) fn validate_plan(plan: &ArtifactPlan) -> Result<(), AppError> {
    if plan.provider.trim().is_empty()
        || plan.project.trim().is_empty()
        || plan.game_version.trim().is_empty()
        || plan.build_id == 0
        || plan.expected_size == 0
        || plan.artifact_name.trim().is_empty()
        || plan.download_url.scheme() != "https"
        || plan.metadata_url.scheme() != "https"
        || !is_sha256(&plan.checksum.value)
    {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Artifact plan is incomplete or invalid for installation.",
        ));
    }
    Ok(())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(unix)]
fn is_executable(metadata: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_metadata: &std::fs::Metadata) -> bool {
    true
}
