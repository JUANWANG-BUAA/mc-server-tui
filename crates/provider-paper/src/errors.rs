//! Structured errors for Paper HTTP and upstream status failures.

use mc_core::{AppError, ErrorCode};
use reqwest::StatusCode;
use url::Url;

pub(super) fn status_error(status: StatusCode, retry_after: Option<&str>) -> AppError {
    if status == StatusCode::TOO_MANY_REQUESTS {
        let evidence = retry_after
            .map(|value| format!("HTTP 429; Retry-After: {value}"))
            .unwrap_or_else(|| "HTTP 429".to_owned());
        return AppError::new(
            ErrorCode::SourceRateLimited,
            "Paper rate limited this request.",
        )
        .with_action("Wait for the upstream retry window, then try again.")
        .with_evidence(evidence)
        .retryable();
    }
    let retryable = status.is_server_error();
    let mut error = AppError::new(
        ErrorCode::SourceUnavailable,
        format!("Paper metadata request failed with HTTP {status}."),
    )
    .with_evidence(format!("HTTP {status}"));
    if retryable {
        error = error.with_action("Retry after a short delay.").retryable();
    } else {
        error = error.with_action("Check the selected Minecraft version and Paper service status.");
    }
    error
}

pub(super) fn network_error(error: reqwest::Error, url: &Url) -> AppError {
    let error = error.without_url();
    let mut result = AppError::new(
        ErrorCode::SourceUnavailable,
        "Could not reach the Paper download service.",
    )
    .with_evidence(format!(
        "{}: {error}",
        url.host_str().unwrap_or("unknown host")
    ))
    .with_action("Check the network connection and retry.");
    if error.is_timeout() || error.is_connect() {
        result = result.retryable();
    }
    result
}
