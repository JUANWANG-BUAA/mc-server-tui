use std::time::Duration;
use std::{io, os::unix::fs::FileTypeExt, path::Path, sync::Arc};

use mc_core::{
    ApiRequestEnvelope, ApiResponse, ApiResponseEnvelope, AppError, ErrorCode, API_PROTOCOL_MAX,
    API_PROTOCOL_MIN,
};
use tokio::{
    fs,
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
};

use crate::{dispatch, state::DaemonState};

const MAX_FRAME_BYTES: u64 = 1024 * 1024;
const REQUEST_READ_TIMEOUT: Duration = Duration::from_secs(10);
const RESPONSE_WRITE_TIMEOUT: Duration = Duration::from_secs(10);

pub(crate) async fn prepare_parent(
    socket_path: &Path,
    private_parent: bool,
) -> Result<(), io::Error> {
    use std::os::unix::fs::PermissionsExt;

    let parent = socket_path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "socket path has no parent directory",
        )
    })?;
    let existed = fs::try_exists(parent).await?;
    fs::create_dir_all(parent).await?;
    let metadata = fs::symlink_metadata(parent).await?;
    if !metadata.file_type().is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "the daemon socket parent must be a real directory",
        ));
    }
    if private_parent || !existed {
        fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700)).await?;
    } else if metadata.permissions().mode() & 0o077 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "the custom socket parent is accessible to other users; use a private directory with mode 0700",
        ));
    }
    Ok(())
}

pub(crate) async fn remove_stale_socket(
    socket_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let metadata = match fs::symlink_metadata(socket_path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if !metadata.file_type().is_socket() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "refusing to replace a non-socket file at the daemon path",
        )
        .into());
    }
    match UnixStream::connect(socket_path).await {
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::AddrInUse,
            "another management daemon is already listening",
        )
        .into()),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::ConnectionRefused | io::ErrorKind::NotFound
            ) =>
        {
            fs::remove_file(socket_path).await?;
            Ok(())
        }
        Err(error) => Err(error.into()),
    }
}

/// Validate the protocol window and the operation's minimum version before
/// dispatch. Returns the protocol to echo in the response and the payload.
fn classify_envelope(
    envelope: ApiRequestEnvelope,
) -> Result<(u16, mc_core::ApiRequest), (u16, Box<ApiResponse>)> {
    let protocol = envelope.protocol_version;
    // A client outside the window cannot be satisfied, but clamping keeps the
    // diagnostic envelope in a shape a nearby version can decode.
    let reply_protocol = protocol.clamp(API_PROTOCOL_MIN, API_PROTOCOL_MAX);
    if !(API_PROTOCOL_MIN..=API_PROTOCOL_MAX).contains(&protocol) {
        return Err((
            reply_protocol,
            Box::new(ApiResponse::Error(
                AppError::new(
                    ErrorCode::ProtocolInvalid,
                    "The client and daemon use incompatible protocol versions.",
                )
                .with_action("Upgrade mc-server-tui so both sides share a protocol version.")
                .with_evidence(format!(
                    "client protocol version: {protocol}; daemon protocol range: {API_PROTOCOL_MIN}..={API_PROTOCOL_MAX}"
                )),
            )),
        ));
    }
    let required = envelope.payload.minimum_protocol_version();
    if required > protocol {
        return Err((
            reply_protocol,
            Box::new(ApiResponse::Error(
                AppError::new(
                    ErrorCode::ProtocolInvalid,
                    "The requested operation is newer than the request protocol.",
                )
                .with_action("Upgrade the client so it negotiates the required protocol version.")
                .with_evidence(format!(
                    "operation requires API protocol {required}; request protocol: {protocol}"
                )),
            )),
        ));
    }
    Ok((protocol, envelope.payload))
}

pub(crate) async fn serve_connection(
    stream: UnixStream,
    state: Arc<DaemonState>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half).take(MAX_FRAME_BYTES + 1);
    let mut line = String::new();
    let (response, response_protocol) =
        match tokio::time::timeout(REQUEST_READ_TIMEOUT, reader.read_line(&mut line)).await {
            Err(_) => (
                ApiResponse::Error(
                    AppError::new(
                        ErrorCode::RequestTimeout,
                        "The daemon timed out while reading the request frame.",
                    )
                    .with_action("Send a complete request frame and retry.")
                    .retryable(),
                ),
                API_PROTOCOL_MIN,
            ),
            Ok(read_result) => match read_result {
                Ok(0) => return Ok(()),
                Ok(size) if size as u64 <= MAX_FRAME_BYTES => {
                    match serde_json::from_str::<ApiRequestEnvelope>(&line) {
                        Ok(envelope) => match classify_envelope(envelope) {
                            Ok((protocol, payload)) => (
                                dispatch::dispatch(payload, &state, protocol).await,
                                protocol,
                            ),
                            Err((reply_protocol, response)) => (*response, reply_protocol),
                        },
                        Err(error) => (
                            ApiResponse::Error(
                                AppError::new(
                                    ErrorCode::ProtocolInvalid,
                                    "The daemon received an invalid request.",
                                )
                                .with_evidence(error.to_string()),
                            ),
                            API_PROTOCOL_MIN,
                        ),
                    }
                }
                Ok(_) => (
                    ApiResponse::Error(AppError::new(
                        ErrorCode::RequestTooLarge,
                        "The request exceeded the 1 MiB protocol limit.",
                    )),
                    API_PROTOCOL_MIN,
                ),
                Err(error) => (
                    ApiResponse::Error(
                        AppError::new(
                            ErrorCode::ProtocolInvalid,
                            "The daemon could not read the request.",
                        )
                        .with_evidence(error.to_string()),
                    ),
                    API_PROTOCOL_MIN,
                ),
            },
        };
    let encoded = serde_json::to_vec(&ApiResponseEnvelope::with_protocol(
        response_protocol,
        response,
    ))?;
    tokio::time::timeout(RESPONSE_WRITE_TIMEOUT, async {
        write_half.write_all(&encoded).await?;
        write_half.write_all(b"\n").await?;
        write_half.shutdown().await
    })
    .await
    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "response write timed out"))??;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mc_core::{ApiRequest, InstanceOperation};

    fn envelope(protocol: u16, payload: ApiRequest) -> ApiRequestEnvelope {
        ApiRequestEnvelope::with_protocol(protocol, payload)
    }

    fn management() -> ApiRequest {
        ApiRequest::ManageInstance {
            instance_id: uuid::Uuid::nil(),
            action: InstanceOperation::ListAddons,
        }
    }

    #[test]
    fn accepts_both_sides_of_the_protocol_window() {
        assert!(matches!(
            classify_envelope(envelope(8, ApiRequest::Ping)),
            Ok((8, ApiRequest::Ping))
        ));
        assert!(matches!(
            classify_envelope(envelope(9, ApiRequest::Ping)),
            Ok((9, ApiRequest::Ping))
        ));
    }

    #[test]
    fn rejects_management_operations_in_a_v8_envelope() {
        match classify_envelope(envelope(8, management())) {
            Err((8, response)) => match *response {
                ApiResponse::Error(error) => {
                    assert_eq!(error.code, ErrorCode::ProtocolInvalid);
                    assert!(error
                        .evidence
                        .as_deref()
                        .is_some_and(|evidence| evidence.contains("requires API protocol 9")));
                }
                _ => panic!("expected a protocol error"),
            },
            _ => panic!("v8 management request must be rejected"),
        }
    }

    #[test]
    fn rejects_versions_outside_the_window_and_clamps_the_reply() {
        for (protocol, reply_protocol) in [(7, 8), (10, 9)] {
            match classify_envelope(envelope(protocol, ApiRequest::Ping)) {
                Err((reply, response)) => {
                    assert_eq!(reply, reply_protocol);
                    match *response {
                        ApiResponse::Error(error) => {
                            assert_eq!(error.code, ErrorCode::ProtocolInvalid);
                            assert!(
                                error.evidence.as_deref().is_some_and(
                                    |evidence| evidence.contains("daemon protocol range")
                                )
                            );
                        }
                        _ => panic!("expected a protocol error"),
                    }
                }
                _ => panic!("out-of-window request must be rejected"),
            }
        }
    }
}
