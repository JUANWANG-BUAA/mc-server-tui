use std::time::Duration;
use std::{io, path::Path};

use mc_core::{
    ApiRequest, ApiRequestEnvelope, ApiResponse, ApiResponseEnvelope, AppError, ErrorCode,
    API_PROTOCOL_VERSION,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
};

const MAX_FRAME_BYTES: u64 = 1024 * 1024;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const WRITE_TIMEOUT: Duration = Duration::from_secs(10);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(40);

pub(crate) async fn request_daemon(
    socket: &Path,
    request: ApiRequest,
) -> Result<ApiResponse, AppError> {
    let connect = tokio::time::timeout(CONNECT_TIMEOUT, UnixStream::connect(socket))
        .await
        .map_err(|_| {
            AppError::new(
                ErrorCode::RequestTimeout,
                "The daemon socket did not accept a connection in time.",
            )
            .with_action("Check daemon status and retry.")
            .retryable()
        })?;
    let mut stream = connect.map_err(|error| {
        AppError::new(
            ErrorCode::SourceUnavailable,
            "The local management daemon is unavailable.",
        )
        .with_action("Start mc-server-tui-daemon, then retry the command.")
        .with_evidence(format!("{}: {error}", socket.display()))
    })?;
    let envelope = ApiRequestEnvelope::current(request);
    let mut encoded = serde_json::to_vec(&envelope).map_err(|error| {
        AppError::new(ErrorCode::Internal, "Could not encode the daemon request.")
            .with_evidence(error.to_string())
    })?;
    if encoded.len() as u64 > MAX_FRAME_BYTES {
        return Err(AppError::new(
            ErrorCode::RequestTooLarge,
            "The request exceeded the 1 MiB protocol limit.",
        ));
    }
    encoded.push(b'\n');
    tokio::time::timeout(WRITE_TIMEOUT, stream.write_all(&encoded))
        .await
        .map_err(|_| {
            AppError::new(
                ErrorCode::RequestTimeout,
                "The daemon did not accept the request frame in time.",
            )
            .with_action("Check daemon status and retry.")
            .retryable()
        })?
        .map_err(protocol_io_error)?;
    let mut reader = BufReader::new(stream).take(MAX_FRAME_BYTES + 1);
    let mut line = String::new();
    let read = tokio::time::timeout(RESPONSE_TIMEOUT, reader.read_line(&mut line))
        .await
        .map_err(|_| {
            AppError::new(
                ErrorCode::RequestTimeout,
                "The daemon did not respond before the client timeout.",
            )
            .with_action("Check daemon status and task history, then retry if safe.")
            .retryable()
        })?
        .map_err(protocol_io_error)?;
    if read == 0 {
        return Err(AppError::new(
            ErrorCode::SourceUnavailable,
            "The daemon closed the connection before replying.",
        )
        .with_action("The daemon may be busy; check its status and retry.")
        .retryable());
    }
    if read as u64 > MAX_FRAME_BYTES {
        return Err(AppError::new(
            ErrorCode::ProtocolInvalid,
            "The daemon returned an oversized response.",
        ));
    }
    let envelope: ApiResponseEnvelope = serde_json::from_str(&line).map_err(|error| {
        AppError::new(
            ErrorCode::ProtocolInvalid,
            "The daemon returned an invalid response.",
        )
        .with_evidence(error.to_string())
    })?;
    if envelope.protocol_version != API_PROTOCOL_VERSION {
        return Err(AppError::new(
            ErrorCode::ProtocolInvalid,
            "The CLI and daemon use incompatible protocol versions.",
        )
        .with_evidence(format!(
            "daemon protocol version: {}; CLI protocol version: {API_PROTOCOL_VERSION}",
            envelope.protocol_version
        )));
    }
    Ok(envelope.payload)
}

fn protocol_io_error(error: io::Error) -> AppError {
    AppError::new(
        ErrorCode::SourceUnavailable,
        "The CLI connection to the daemon failed.",
    )
    .with_action("Check that the daemon is still running, then retry.")
    .with_evidence(error.to_string())
}
