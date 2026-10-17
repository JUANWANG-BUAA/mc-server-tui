use std::{
    io,
    os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt},
    path::Path,
    sync::Arc,
    time::Duration,
};

use mc_core::{AppError, ErrorCode};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
    process::ChildStdin,
    sync::Mutex,
};
use uuid::Uuid;

use crate::MAX_CONSOLE_COMMAND_BYTES;

const CONTROL_IO_TIMEOUT: Duration = Duration::from_secs(3);
const CONTROL_FRAME_LIMIT: usize = 2048;

#[derive(Debug, Serialize, Deserialize)]
struct ConsoleRequest {
    command: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ConsoleResponse {
    accepted: bool,
    message: Option<String>,
}

pub(crate) fn control_socket_path(instance_id: Uuid) -> std::path::PathBuf {
    let user_id = unsafe { libc::geteuid() };
    std::env::temp_dir()
        .join(format!("mcst-{user_id}"))
        .join(format!("{}.sock", instance_id.simple()))
}

pub(crate) async fn bind_control_socket(path: &Path) -> Result<UnixListener, AppError> {
    let Some(parent) = path.parent() else {
        return Err(AppError::new(
            ErrorCode::Internal,
            "Runner control socket has no parent directory.",
        ));
    };
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|error| io_error("Could not create runner control directory.", error))?;
    let metadata = tokio::fs::symlink_metadata(parent)
        .await
        .map_err(|error| io_error("Could not inspect runner control directory.", error))?;
    if !metadata.file_type().is_dir() || metadata.uid() != unsafe { libc::geteuid() } {
        return Err(AppError::new(
            ErrorCode::Internal,
            "Runner control directory is not a private directory owned by this user.",
        ));
    }
    tokio::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))
        .await
        .map_err(|error| io_error("Could not protect runner control directory.", error))?;

    match tokio::fs::symlink_metadata(path).await {
        Ok(metadata) if metadata.file_type().is_socket() => {
            match tokio::time::timeout(Duration::from_millis(250), UnixStream::connect(path)).await
            {
                Ok(Ok(_)) => {
                    return Err(AppError::new(
                        ErrorCode::InstanceBusy,
                        "A runner already owns the instance control socket.",
                    ));
                }
                Ok(Err(error)) if error.kind() == io::ErrorKind::ConnectionRefused => {
                    tokio::fs::remove_file(path).await.map_err(|remove_error| {
                        io_error("Could not remove a stale runner socket.", remove_error)
                    })?;
                }
                Err(_) => {
                    return Err(AppError::new(
                        ErrorCode::RequestTimeout,
                        "Could not determine whether the runner control socket is stale.",
                    ));
                }
                Ok(Err(error)) if error.kind() == io::ErrorKind::NotFound => {}
                Ok(Err(error)) => return Err(io_error("Could not inspect runner socket.", error)),
            }
        }
        Ok(_) => {
            return Err(AppError::new(
                ErrorCode::Internal,
                "Runner control socket path is not a socket.",
            ));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(io_error("Could not inspect runner control socket.", error)),
    }

    let listener = UnixListener::bind(path)
        .map_err(|error| io_error("Could not bind runner control socket.", error))?;
    tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .await
        .map_err(|error| io_error("Could not protect runner control socket.", error))?;
    Ok(listener)
}

pub(crate) async fn send_console_command(instance_id: Uuid, command: &str) -> Result<(), AppError> {
    let path = control_socket_path(instance_id);
    let result = tokio::time::timeout(CONTROL_IO_TIMEOUT, async {
        let mut stream = UnixStream::connect(path)
            .await
            .map_err(|error| io_error("Could not connect to the instance runner.", error))?;
        let request = serde_json::to_vec(&ConsoleRequest {
            command: command.to_owned(),
        })
        .map_err(|error| {
            AppError::new(ErrorCode::Internal, "Could not encode console request.")
                .with_evidence(error.to_string())
        })?;
        stream
            .write_all(&request)
            .await
            .map_err(|error| io_error("Could not send console request.", error))?;
        stream
            .write_all(b"\n")
            .await
            .map_err(|error| io_error("Could not finish console request.", error))?;
        let mut line = Vec::new();
        let limited = (&mut stream).take(CONTROL_FRAME_LIMIT as u64);
        BufReader::new(limited)
            .read_until(b'\n', &mut line)
            .await
            .map_err(|error| io_error("Could not read console response.", error))?;
        let response: ConsoleResponse = serde_json::from_slice(&line).map_err(|error| {
            AppError::new(
                ErrorCode::SourceSchemaChanged,
                "Runner response was invalid.",
            )
            .with_evidence(error.to_string())
        })?;
        if response.accepted {
            Ok(())
        } else {
            Err(AppError::new(
                ErrorCode::ServerNotRunning,
                response
                    .message
                    .unwrap_or_else(|| "The runner did not accept console input.".to_owned()),
            ))
        }
    })
    .await;
    result.map_err(|_| {
        AppError::new(
            ErrorCode::RequestTimeout,
            "The runner control request timed out.",
        )
        .retryable()
    })?
}

pub(crate) async fn serve_console_request(
    mut stream: UnixStream,
    stdin: Arc<Mutex<ChildStdin>>,
) -> Result<(), AppError> {
    let result = tokio::time::timeout(CONTROL_IO_TIMEOUT, async {
        let mut line = Vec::new();
        let limited = (&mut stream).take(CONTROL_FRAME_LIMIT as u64);
        let read = BufReader::new(limited)
            .read_until(b'\n', &mut line)
            .await
            .map_err(|error| io_error("Could not read console request.", error))?;
        if read == 0 || line.len() >= CONTROL_FRAME_LIMIT || !line.ends_with(b"\n") {
            return Err(AppError::new(
                ErrorCode::RequestTooLarge,
                "Console request was empty or exceeded the frame limit.",
            ));
        }
        let request: ConsoleRequest = serde_json::from_slice(&line).map_err(|error| {
            AppError::new(ErrorCode::InvalidInput, "Console request was invalid.")
                .with_evidence(error.to_string())
        })?;
        validate_command(&request.command)?;
        let mut stdin = stdin.lock().await;
        stdin
            .write_all(request.command.as_bytes())
            .await
            .map_err(|error| io_error("Could not write to Java standard input.", error))?;
        stdin
            .write_all(b"\n")
            .await
            .map_err(|error| io_error("Could not finish Java console command.", error))?;
        stdin
            .flush()
            .await
            .map_err(|error| io_error("Could not flush Java console command.", error))
    })
    .await;
    let response = match result {
        Ok(Ok(())) => ConsoleResponse {
            accepted: true,
            message: None,
        },
        Ok(Err(error)) => ConsoleResponse {
            accepted: false,
            message: Some(error.reason),
        },
        Err(_) => ConsoleResponse {
            accepted: false,
            message: Some("Console input timed out.".to_owned()),
        },
    };
    let mut encoded = serde_json::to_vec(&response).map_err(|error| {
        AppError::new(ErrorCode::Internal, "Could not encode console response.")
            .with_evidence(error.to_string())
    })?;
    encoded.push(b'\n');
    stream
        .write_all(&encoded)
        .await
        .map_err(|error| io_error("Could not send console response.", error))
}

fn validate_command(command: &str) -> Result<(), AppError> {
    if command.trim().is_empty()
        || command.len() > MAX_CONSOLE_COMMAND_BYTES
        || command.chars().any(char::is_control)
    {
        return Err(AppError::new(
            ErrorCode::InvalidConsoleCommand,
            "Console command must contain 1 to 512 printable bytes and no line breaks.",
        ));
    }
    Ok(())
}

fn io_error(reason: &str, error: io::Error) -> AppError {
    AppError::new(ErrorCode::Internal, reason).with_evidence(error.to_string())
}
