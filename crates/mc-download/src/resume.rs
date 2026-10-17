use crate::cache::{discard_partial, remove_if_regular_file};
use crate::digest::hex_digest;
use crate::errors::io_error;
use mc_core::{AppError, ArtifactPlan, ErrorCode};
use reqwest::header::HeaderValue;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use url::Url;

const COPY_BUFFER_SIZE: usize = 64 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ResumeMetadata {
    pub(crate) url_fingerprint: String,
    pub(crate) expected_size: u64,
    pub(crate) expected_sha256: String,
    pub(crate) strong_etag: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ResumeState {
    pub(crate) offset: u64,
    pub(crate) strong_etag: Option<String>,
}

pub(crate) async fn read_resume_state(
    part_path: &Path,
    metadata_path: &Path,
    plan: &ArtifactPlan,
) -> Result<Option<ResumeState>, AppError> {
    let part_info = match tokio::fs::symlink_metadata(part_path).await {
        Ok(metadata) if metadata.file_type().is_file() => metadata,
        Ok(_) => {
            return Err(AppError::new(
                ErrorCode::Internal,
                "Temporary download path is not a regular file.",
            ))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(io_error(
                "Could not inspect temporary download file.",
                error,
            ))
        }
    };
    match tokio::fs::symlink_metadata(metadata_path).await {
        Ok(metadata) if metadata.file_type().is_file() => {}
        Ok(_) => {
            return Err(AppError::new(
                ErrorCode::Internal,
                "Resume metadata path is not a regular file.",
            ))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            discard_partial(part_path, metadata_path).await?;
            return Ok(None);
        }
        Err(error) => return Err(io_error("Could not inspect resume metadata.", error)),
    }
    let metadata_bytes = match tokio::fs::read(metadata_path).await {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            discard_partial(part_path, metadata_path).await?;
            return Ok(None);
        }
        Err(error) => return Err(io_error("Could not read download resume metadata.", error)),
    };
    let saved: ResumeMetadata = match serde_json::from_slice(&metadata_bytes) {
        Ok(saved) => saved,
        Err(_) => {
            discard_partial(part_path, metadata_path).await?;
            return Ok(None);
        }
    };
    let offset = part_info.len();
    let identity_matches = saved.url_fingerprint == url_fingerprint(&plan.download_url)
        && saved.expected_size == plan.expected_size
        && saved
            .expected_sha256
            .eq_ignore_ascii_case(&plan.checksum.value);
    if !identity_matches
        || !saved.strong_etag.as_deref().is_some_and(is_strong_etag)
        || offset == 0
        || offset >= plan.expected_size
    {
        discard_partial(part_path, metadata_path).await?;
        return Ok(None);
    }
    Ok(Some(ResumeState {
        offset,
        strong_etag: saved.strong_etag,
    }))
}

pub(crate) async fn write_resume_metadata(
    path: &Path,
    metadata: &ResumeMetadata,
) -> Result<(), AppError> {
    let temp_path = path.with_extension("json.tmp");
    remove_if_regular_file(&temp_path).await?;
    let bytes = serde_json::to_vec(metadata)
        .map_err(|_| AppError::new(ErrorCode::Internal, "Could not encode resume metadata."))?;
    let mut options = tokio::fs::OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        options.mode(0o600);
    }
    let mut file = options
        .open(&temp_path)
        .await
        .map_err(|error| io_error("Could not create resume metadata.", error))?;
    file.write_all(&bytes)
        .await
        .map_err(|error| io_error("Could not write resume metadata.", error))?;
    file.sync_all()
        .await
        .map_err(|error| io_error("Could not sync resume metadata.", error))?;
    drop(file);
    tokio::fs::rename(temp_path, path)
        .await
        .map_err(|error| io_error("Could not commit resume metadata.", error))
}

pub(crate) async fn hash_existing_file(path: &Path, hasher: &mut Sha256) -> Result<(), AppError> {
    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|error| io_error("Could not read partial artifact for resume.", error))?;
    let mut buffer = vec![0; COPY_BUFFER_SIZE];
    loop {
        let count = file
            .read(&mut buffer)
            .await
            .map_err(|error| io_error("Could not read partial artifact for resume.", error))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(())
}

pub(crate) fn valid_content_range(value: &str, expected_start: u64, expected_total: u64) -> bool {
    let Some(range) = value.strip_prefix("bytes ") else {
        return false;
    };
    let Some((numbers, total)) = range.split_once('/') else {
        return false;
    };
    let Some((start, end)) = numbers.split_once('-') else {
        return false;
    };
    let parsed = (
        start.parse::<u64>(),
        end.parse::<u64>(),
        total.parse::<u64>(),
    );
    matches!(parsed, (Ok(start), Ok(end), Ok(total)) if total > 0 && start == expected_start && end == total - 1 && total == expected_total)
}

pub(crate) fn strong_etag(value: Option<&HeaderValue>) -> Option<String> {
    let value = value?.to_str().ok()?.trim();
    is_strong_etag(value).then(|| value.to_owned())
}

pub(crate) fn is_strong_etag(value: &str) -> bool {
    !value.starts_with("W/")
        && value.len() >= 2
        && value.starts_with('"')
        && value.ends_with('"')
        && !value[1..value.len() - 1].contains('"')
}

pub(crate) fn url_fingerprint(url: &Url) -> String {
    hex_digest(Sha256::digest(url.as_str().as_bytes()).as_slice())
}
