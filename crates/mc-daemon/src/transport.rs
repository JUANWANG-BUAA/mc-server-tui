use std::time::Duration;
use std::{io, os::unix::fs::FileTypeExt, path::Path, sync::Arc};

use mc_core::{
    ApiRequestEnvelope, ApiResponse, ApiResponseEnvelope, AppError, ErrorCode, API_PROTOCOL_VERSION,
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

pub(crate) async fn serve_connection(
    stream: UnixStream,
    state: Arc<DaemonState>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half).take(MAX_FRAME_BYTES + 1);
    let mut line = String::new();
    let response = match tokio::time::timeout(REQUEST_READ_TIMEOUT, reader.read_line(&mut line))
        .await
    {
        Err(_) => ApiResponse::Error(
            AppError::new(
                ErrorCode::RequestTimeout,
                "The daemon timed out while reading the request frame.",
            )
            .with_action("Send a complete request frame and retry.")
            .retryable(),
        ),
        Ok(read_result) => match read_result {
            Ok(0) => return Ok(()),
            Ok(size) if size as u64 <= MAX_FRAME_BYTES => {
                match serde_json::from_str::<ApiRequestEnvelope>(&line) {
                Ok(envelope) if envelope.protocol_version == API_PROTOCOL_VERSION => {
                    dispatch::dispatch(envelope.payload, &state).await
                }
                Ok(envelope) => ApiResponse::Error(
                    AppError::new(
                        ErrorCode::ProtocolInvalid,
                        "The CLI and daemon use incompatible protocol versions.",
                    )
                    .with_evidence(format!(
                        "client protocol version: {}; daemon protocol version: {API_PROTOCOL_VERSION}",
                        envelope.protocol_version
                    )),
                ),
                Err(error) => ApiResponse::Error(
                    AppError::new(
                        ErrorCode::ProtocolInvalid,
                        "The daemon received an invalid request.",
                    )
                    .with_evidence(error.to_string()),
                ),
            }
            }
            Ok(_) => ApiResponse::Error(AppError::new(
                ErrorCode::RequestTooLarge,
                "The request exceeded the 1 MiB protocol limit.",
            )),
            Err(error) => ApiResponse::Error(
                AppError::new(
                    ErrorCode::ProtocolInvalid,
                    "The daemon could not read the request.",
                )
                .with_evidence(error.to_string()),
            ),
        },
    };
    let encoded = serde_json::to_vec(&ApiResponseEnvelope::current(response))?;
    tokio::time::timeout(RESPONSE_WRITE_TIMEOUT, async {
        write_half.write_all(&encoded).await?;
        write_half.write_all(b"\n").await?;
        write_half.shutdown().await
    })
    .await
    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "response write timed out"))??;
    Ok(())
}
