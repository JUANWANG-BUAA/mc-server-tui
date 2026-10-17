use mc_core::{AppError, ErrorCode};
use reqwest::StatusCode;

pub(crate) fn is_redirect(status: StatusCode) -> bool {
    matches!(status.as_u16(), 301 | 302 | 303 | 307 | 308)
}

pub(crate) fn http_status_error(status: StatusCode) -> AppError {
    let mut error = AppError::new(
        ErrorCode::SourceUnavailable,
        "The artifact source returned an unsuccessful HTTP response.",
    )
    .with_evidence(format!("http_status={}", status.as_u16()));
    if status.is_server_error() {
        error = error
            .with_action("Retry the same artifact after a short wait.")
            .retryable();
    }
    error
}

pub(crate) fn redirect_error(evidence: &str) -> AppError {
    AppError::new(
        ErrorCode::RedirectBlocked,
        "The artifact URL or redirect failed source validation.",
    )
    .with_evidence(evidence)
}

pub(crate) fn io_error(reason: &str, error: std::io::Error) -> AppError {
    AppError::new(ErrorCode::Internal, reason).with_evidence(error.kind().to_string())
}
