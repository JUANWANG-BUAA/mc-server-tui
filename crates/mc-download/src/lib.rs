//! Verified, content-addressed artifact downloading.
//!
//! Provider-specific HTTP and cache policies are isolated behind `DownloadManager`.

mod cache;
mod digest;
mod errors;
mod resume;
mod security;
mod transport;

use crate::cache::{
    acquire_lock, check_available_space, ensure_private_dir, prepare_cache_root, sync_directory,
    verify_cached_blob,
};
use crate::security::validate_url;
use mc_core::{AppError, ArtifactPlan, ErrorCode};
use std::collections::HashSet;
use std::path::PathBuf;

/// Configuration for a provider-specific downloader.
#[derive(Debug, Clone)]
pub struct DownloadConfig {
    /// Parent directory under which the downloader creates its private cache.
    pub cache_root: PathBuf,
    pub user_agent: String,
    /// Exact host names trusted for this provider, including redirect targets.
    /// Entries are exact matches; subdomains are never implicitly trusted.
    pub allowed_hosts: HashSet<String>,
    pub max_artifact_size: u64,
}

impl DownloadConfig {
    pub fn paper(cache_root: impl Into<PathBuf>, user_agent: impl Into<String>) -> Self {
        Self {
            cache_root: cache_root.into(),
            user_agent: user_agent.into(),
            allowed_hosts: ["fill-data.papermc.io".to_owned()].into_iter().collect(),
            max_artifact_size: 2 * 1024 * 1024 * 1024,
        }
    }
}

/// A locally cached artifact whose declared length and upstream SHA-256 passed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedBlob {
    path: PathBuf,
    size: u64,
    sha256: String,
}

impl VerifiedBlob {
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn sha256(&self) -> &str {
        &self.sha256
    }
}

#[cfg(feature = "test-support")]
impl VerifiedBlob {
    /// Constructs a fixture artifact for downstream crate tests.
    pub fn test_fixture(path: PathBuf, size: u64, sha256: impl Into<String>) -> Self {
        Self {
            path,
            size,
            sha256: sha256.into(),
        }
    }
}

/// Downloads an exact `ArtifactPlan` into a revalidated SHA-256 cache.
#[derive(Debug, Clone)]
pub struct DownloadManager {
    pub(crate) config: DownloadConfig,
}

impl DownloadManager {
    pub fn new(config: DownloadConfig) -> Result<Self, AppError> {
        if config.user_agent.trim().is_empty() {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "A descriptive downloader User-Agent is required.",
            ));
        }
        if config.max_artifact_size == 0 || config.allowed_hosts.is_empty() {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Downloader size limit and trusted host list must be configured.",
            ));
        }
        let allowed_hosts = config
            .allowed_hosts
            .iter()
            .map(|host| host.to_ascii_lowercase())
            .collect();
        Ok(Self {
            config: DownloadConfig {
                allowed_hosts,
                ..config
            },
        })
    }

    pub async fn fetch(&self, plan: &ArtifactPlan) -> Result<VerifiedBlob, AppError> {
        self.validate_plan(plan)?;
        let cache_root = prepare_cache_root(&self.config.cache_root).await?;
        let key = plan.checksum.value.to_ascii_lowercase();
        let sha_root = cache_root.join("sha256");
        ensure_private_dir(&sha_root).await?;
        let bucket = sha_root.join(&key[..2]);
        ensure_private_dir(&bucket).await?;
        let locks_dir = cache_root.join("locks");
        ensure_private_dir(&locks_dir).await?;

        let blob_path = bucket.join(&key);
        let lock_path = locks_dir.join(format!("{key}.lock"));
        let _lock = acquire_lock(lock_path).await?;

        if let Some(blob) = verify_cached_blob(&blob_path, plan).await? {
            return Ok(blob);
        }
        check_available_space(&cache_root, plan.expected_size).await?;
        self.download_to_part(
            plan,
            &bucket.join(format!("{key}.part")),
            &bucket.join(format!("{key}.part.json")),
        )
        .await?;

        tokio::fs::rename(&bucket.join(format!("{key}.part")), &blob_path)
            .await
            .map_err(|error| {
                errors::io_error("Could not commit the verified artifact to cache.", error)
            })?;
        cache::remove_if_regular_file(&bucket.join(format!("{key}.part.json"))).await?;
        sync_directory(bucket).await?;
        verify_cached_blob(&blob_path, plan)
            .await?
            .ok_or_else(|| AppError::new(ErrorCode::Internal, "Committed artifact is missing."))
    }

    fn validate_plan(&self, plan: &ArtifactPlan) -> Result<(), AppError> {
        validate_url(&plan.download_url, &self.config.allowed_hosts)?;
        if plan.expected_size == 0 || plan.expected_size > self.config.max_artifact_size {
            return Err(AppError::new(
                ErrorCode::DownloadTooLarge,
                "The artifact size is outside the configured download limit.",
            ));
        }
        if plan.checksum.value.len() != 64
            || !plan
                .checksum
                .value
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "The artifact plan contains an invalid SHA-256 checksum.",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
