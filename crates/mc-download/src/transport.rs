use crate::cache::discard_partial;
use crate::digest::hex_digest;
use crate::errors::{http_status_error, io_error, is_redirect};
use crate::resume::{
    hash_existing_file, read_resume_state, strong_etag, url_fingerprint, valid_content_range,
    write_resume_metadata, ResumeMetadata, ResumeState,
};
use crate::security::{resolve_public_address, validate_url};
use crate::DownloadManager;
use mc_core::{AppError, ArtifactPlan, ErrorCode};
use reqwest::header::{CONTENT_LENGTH, ETAG, IF_RANGE, LOCATION, RANGE};
use reqwest::{Client, StatusCode};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use url::Url;

const MAX_REDIRECTS: usize = 5;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const READ_TIMEOUT: Duration = Duration::from_secs(30);
const TRANSFER_TIMEOUT: Duration = Duration::from_secs(2 * 60 * 60);
impl DownloadManager {
    pub(crate) async fn download_to_part(
        &self,
        plan: &ArtifactPlan,
        part_path: &Path,
        metadata_path: &Path,
    ) -> Result<(), AppError> {
        let plan_fingerprint = url_fingerprint(&plan.download_url);
        let mut resume = read_resume_state(part_path, metadata_path, plan).await?;
        let mut restarted_after_range_rejection = false;

        loop {
            let response = self
                .send_with_redirects(&plan.download_url, resume.as_ref())
                .await?;
            let status = response.status();
            if status == StatusCode::TOO_MANY_REQUESTS {
                return Err(AppError::new(
                    ErrorCode::SourceRateLimited,
                    "The artifact source is rate limiting downloads.",
                )
                .with_action("Wait briefly and retry the same artifact.")
                .retryable());
            }

            let (append, offset, etag) = if resume.is_some() {
                match status {
                    StatusCode::PARTIAL_CONTENT => {
                        let state = resume.as_ref().expect("checked above");
                        let content_range = response
                            .headers()
                            .get(reqwest::header::CONTENT_RANGE)
                            .and_then(|value| value.to_str().ok());
                        let response_etag = strong_etag(response.headers().get(ETAG));
                        if !content_range.is_some_and(|value| {
                            valid_content_range(value, state.offset, plan.expected_size)
                        }) || response_etag.as_deref() != state.strong_etag.as_deref()
                        {
                            drop(response);
                            discard_partial(part_path, metadata_path).await?;
                            resume = None;
                            if restarted_after_range_rejection {
                                return Err(AppError::new(
                                    ErrorCode::DownloadInterrupted,
                                    "The source returned an invalid range response twice.",
                                ));
                            }
                            restarted_after_range_rejection = true;
                            continue;
                        }
                        (true, state.offset, response_etag)
                    }
                    StatusCode::OK => {
                        // The source declined Range; safely start over with this full response.
                        (false, 0, strong_etag(response.headers().get(ETAG)))
                    }
                    StatusCode::RANGE_NOT_SATISFIABLE => {
                        drop(response);
                        discard_partial(part_path, metadata_path).await?;
                        resume = None;
                        if restarted_after_range_rejection {
                            return Err(AppError::new(
                                ErrorCode::DownloadInterrupted,
                                "The source rejected a clean range restart.",
                            ));
                        }
                        restarted_after_range_rejection = true;
                        continue;
                    }
                    _ => return Err(http_status_error(status)),
                }
            } else if status == StatusCode::OK {
                (false, 0, strong_etag(response.headers().get(ETAG)))
            } else {
                return Err(http_status_error(status));
            };

            let expected_response_size = plan.expected_size.saturating_sub(offset);
            if let Some(content_length) = response
                .headers()
                .get(CONTENT_LENGTH)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok())
            {
                if content_length > expected_response_size {
                    discard_partial(part_path, metadata_path).await?;
                    return Err(AppError::new(
                        ErrorCode::DownloadTooLarge,
                        "The source response exceeds the artifact plan size.",
                    ));
                }
            }

            let metadata = ResumeMetadata {
                url_fingerprint: plan_fingerprint.clone(),
                expected_size: plan.expected_size,
                expected_sha256: plan.checksum.value.clone(),
                strong_etag: etag.clone(),
            };
            write_resume_metadata(metadata_path, &metadata).await?;

            let mut options = tokio::fs::OpenOptions::new();
            options.create(true).write(true);
            if append {
                options.append(true);
            } else {
                options.truncate(true);
            }
            #[cfg(unix)]
            {
                options.custom_flags(libc::O_NOFOLLOW);
                options.mode(0o600);
            }
            let mut file = options
                .open(part_path)
                .await
                .map_err(|error| io_error("Could not open the temporary download file.", error))?;

            let result = tokio::time::timeout(TRANSFER_TIMEOUT, async {
                let mut hasher = Sha256::new();
                if append {
                    hash_existing_file(part_path, &mut hasher).await?;
                }
                let mut written = offset;
                let mut response = response;
                while let Some(chunk) = response.chunk().await.map_err(|_| {
                    AppError::new(
                        ErrorCode::DownloadInterrupted,
                        "The artifact transfer was interrupted.",
                    )
                    .with_action("Retry the same artifact; a verified range may be resumed.")
                    .retryable()
                })? {
                    let chunk_len = u64::try_from(chunk.len()).unwrap_or(u64::MAX);
                    written = written.checked_add(chunk_len).ok_or_else(|| {
                        AppError::new(
                            ErrorCode::DownloadTooLarge,
                            "The source response exceeded the artifact size limit.",
                        )
                    })?;
                    if written > plan.expected_size {
                        return Err(AppError::new(
                            ErrorCode::DownloadTooLarge,
                            "The source response exceeded the artifact plan size.",
                        ));
                    }
                    hasher.update(&chunk);
                    file.write_all(&chunk).await.map_err(|error| {
                        io_error("Could not write the temporary download file.", error)
                    })?;
                }
                if written != plan.expected_size {
                    return Err(AppError::new(
                        ErrorCode::DownloadInterrupted,
                        "The downloaded artifact length does not match its metadata.",
                    )
                    .with_action("Retry the same artifact.")
                    .retryable());
                }
                file.flush()
                    .await
                    .map_err(|error| io_error("Could not flush the downloaded artifact.", error))?;
                file.sync_all()
                    .await
                    .map_err(|error| io_error("Could not sync the downloaded artifact.", error))?;
                let actual_hash = hex_digest(hasher.finalize().as_slice());
                if !actual_hash.eq_ignore_ascii_case(&plan.checksum.value) {
                    return Err(AppError::new(
                        ErrorCode::HashMismatch,
                        "The downloaded artifact SHA-256 does not match the source metadata.",
                    ));
                }
                Ok::<(), AppError>(())
            })
            .await;

            match result {
                Ok(Ok(())) => {
                    return Ok(());
                }
                Ok(Err(error)) => {
                    let preserve = error.code == ErrorCode::DownloadInterrupted && etag.is_some();
                    if !preserve {
                        discard_partial(part_path, metadata_path).await?;
                    }
                    return Err(error);
                }
                Err(_) => {
                    if etag.is_none() {
                        discard_partial(part_path, metadata_path).await?;
                    }
                    return Err(AppError::new(
                        ErrorCode::DownloadInterrupted,
                        "The artifact transfer exceeded its time limit.",
                    )
                    .with_action("Retry the same artifact; a verified range may be resumed.")
                    .retryable());
                }
            }
        }
    }

    async fn send_with_redirects(
        &self,
        initial_url: &Url,
        resume: Option<&ResumeState>,
    ) -> Result<reqwest::Response, AppError> {
        let mut current_url = initial_url.clone();
        for redirect_count in 0..=MAX_REDIRECTS {
            let address = resolve_public_address(&current_url, &self.config.allowed_hosts).await?;
            let host = current_url.host_str().expect("validated URL host");
            let client = Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .connect_timeout(CONNECT_TIMEOUT)
                .read_timeout(READ_TIMEOUT)
                .no_proxy()
                .resolve(host, address)
                .build()
                .map_err(|_| {
                    AppError::new(ErrorCode::Internal, "Could not configure HTTP client.")
                })?;
            let mut request = client
                .get(current_url.clone())
                .header(reqwest::header::USER_AGENT, &self.config.user_agent);
            if let Some(state) = resume {
                request = request
                    .header(RANGE, format!("bytes={}-", state.offset))
                    .header(
                        IF_RANGE,
                        state.strong_etag.as_deref().expect("resume requires ETag"),
                    );
            }
            let response = request.send().await.map_err(|_| {
                AppError::new(
                    ErrorCode::SourceUnavailable,
                    "Could not connect to the artifact source.",
                )
                .with_action("Check the network and retry the same artifact.")
                .retryable()
            })?;

            if !is_redirect(response.status()) {
                return Ok(response);
            }
            if redirect_count == MAX_REDIRECTS {
                return Err(AppError::new(
                    ErrorCode::RedirectBlocked,
                    "The artifact source exceeded the redirect limit.",
                ));
            }
            let location = response
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| {
                    AppError::new(
                        ErrorCode::RedirectBlocked,
                        "The artifact source returned an invalid redirect.",
                    )
                })?;
            let next_url = current_url.join(location).map_err(|_| {
                AppError::new(
                    ErrorCode::RedirectBlocked,
                    "The artifact source returned an invalid redirect target.",
                )
            })?;
            validate_url(&next_url, &self.config.allowed_hosts)?;
            current_url = next_url;
        }
        unreachable!("redirect loop always returns or errors")
    }
}
