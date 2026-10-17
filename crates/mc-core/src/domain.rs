use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use url::Url;
use uuid::Uuid;

use crate::{AppError, ErrorCode};

/// An upstream game version identifier. It is intentionally opaque: Minecraft
/// version strings are not assumed to follow semantic-versioning rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerVersion {
    pub id: String,
    pub group: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseChannel(pub String);

impl ReleaseChannel {
    pub fn is_stable(&self) -> bool {
        self.0.eq_ignore_ascii_case("stable")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checksum {
    pub algorithm: ChecksumAlgorithm,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ChecksumAlgorithm {
    Sha256,
}

impl Checksum {
    pub fn sha256(value: impl Into<String>) -> Result<Self, AppError> {
        let value = value.into();
        if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(AppError::new(
                ErrorCode::SourceSchemaChanged,
                "Paper returned an invalid SHA-256 checksum.",
            ));
        }

        Ok(Self {
            algorithm: ChecksumAlgorithm::Sha256,
            value: value.to_ascii_lowercase(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildInfo {
    pub id: u64,
    pub channel: ReleaseChannel,
    pub released_at: Option<DateTime<Utc>>,
    pub artifact_name: Option<String>,
    pub artifact_size: Option<u64>,
}

/// A resolved, immutable description of one exact upstream artifact.
///
/// The provider constructs plans only after validating their upstream fields.
/// Download and installer modules will consume this object without resolving
/// `latest` again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactPlan {
    pub provider: String,
    pub project: String,
    pub game_version: String,
    pub build_id: u64,
    pub channel: ReleaseChannel,
    pub artifact_name: String,
    pub metadata_url: Url,
    pub download_url: Url,
    pub expected_size: u64,
    pub checksum: Checksum,
    pub resolved_at: DateTime<Utc>,
}

/// Transport-neutral instance index entry returned to CLI and TUI clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstanceSummary {
    pub id: Uuid,
    pub name: String,
    pub provider: String,
    pub game_version: String,
    pub build_id: u64,
    pub java_path: PathBuf,
    pub min_memory_mb: u32,
    pub max_memory_mb: u32,
    pub port: u16,
    pub directory: PathBuf,
    pub created_at: DateTime<Utc>,
}

impl ArtifactPlan {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        provider: impl Into<String>,
        project: impl Into<String>,
        game_version: impl Into<String>,
        build_id: u64,
        channel: ReleaseChannel,
        artifact_name: impl Into<String>,
        metadata_url: Url,
        download_url: Url,
        expected_size: u64,
        checksum: Checksum,
        resolved_at: DateTime<Utc>,
    ) -> Result<Self, AppError> {
        let provider = provider.into();
        let project = project.into();
        let game_version = game_version.into();
        let artifact_name = artifact_name.into();
        if provider.trim().is_empty()
            || project.trim().is_empty()
            || game_version.trim().is_empty()
            || artifact_name.trim().is_empty()
            || build_id == 0
            || expected_size == 0
        {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Artifact plan fields must identify a non-empty, exact artifact.",
            ));
        }
        if metadata_url.scheme() != "https" || download_url.scheme() != "https" {
            return Err(AppError::new(
                ErrorCode::RedirectBlocked,
                "Artifact metadata and download URLs must use HTTPS.",
            ));
        }

        Ok(Self {
            provider,
            project,
            game_version,
            build_id,
            channel,
            artifact_name,
            metadata_url,
            download_url,
            expected_size,
            checksum,
            resolved_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_is_normalized_and_validated() {
        let checksum = Checksum::sha256("AB".repeat(32)).expect("valid checksum");
        assert_eq!(checksum.value, "ab".repeat(32));
        assert!(Checksum::sha256("not-a-hash").is_err());
    }

    #[test]
    fn artifact_plan_rejects_http_urls() {
        let result = ArtifactPlan::new(
            "paper",
            "paper",
            "26.1",
            1,
            ReleaseChannel("STABLE".to_owned()),
            "paper.jar",
            Url::parse("https://fill.papermc.io/metadata").expect("valid URL"),
            Url::parse("http://fill-data.papermc.io/paper.jar").expect("valid URL"),
            1024,
            Checksum::sha256("a1".repeat(32)).expect("valid checksum"),
            Utc::now(),
        );

        assert_eq!(
            result.expect_err("HTTP must be rejected").code,
            ErrorCode::RedirectBlocked
        );
    }
}
