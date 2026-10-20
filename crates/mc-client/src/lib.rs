//! Async local daemon client shared by the CLI and TUI.

use std::{
    io,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use mc_core::{
    ApiRequest, ApiRequestEnvelope, ApiResponse, ApiResponseEnvelope, AppError, ErrorCode,
    API_PROTOCOL_MAX, API_PROTOCOL_MIN,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
    sync::OnceCell,
};

const MAX_FRAME_BYTES: u64 = 1024 * 1024;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const WRITE_TIMEOUT: Duration = Duration::from_secs(10);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(40);

#[derive(Debug, Clone)]
pub struct DaemonClient {
    socket: PathBuf,
    /// Highest protocol version the daemon accepted during the one-time
    /// handshake. Clones share the negotiated value.
    protocol: Arc<OnceCell<u16>>,
}

impl DaemonClient {
    pub fn new(socket: impl Into<PathBuf>) -> Self {
        Self {
            socket: socket.into(),
            protocol: Arc::new(OnceCell::new()),
        }
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket
    }

    /// Send a request at the highest protocol version both sides support.
    ///
    /// The first call performs a `ServerInfo` probe at `API_PROTOCOL_MIN`, so
    /// both an older daemon (N-1) and a newer daemon can answer. An operation
    /// introduced after the daemon's version is rejected locally with an
    /// actionable error rather than being sent as an unknown operation.
    pub async fn request(&self, request: ApiRequest) -> Result<ApiResponse, AppError> {
        let protocol = *self.protocol.get_or_try_init(|| self.negotiate()).await?;
        let required = request.minimum_protocol_version();
        if required > protocol {
            return Err(AppError::new(
                ErrorCode::ProtocolInvalid,
                "This operation requires a newer management daemon.",
            )
            .with_action("Upgrade mc-server-tui so the client and daemon use compatible versions, then retry.")
            .with_evidence(format!(
                "operation requires API protocol {required}; daemon supports up to {protocol}"
            )));
        }
        self.exchange(protocol, request).await
    }

    async fn negotiate(&self) -> Result<u16, AppError> {
        match self
            .exchange(API_PROTOCOL_MIN, ApiRequest::ServerInfo)
            .await?
        {
            ApiResponse::ServerInfo(info) => info
                .negotiated_version(API_PROTOCOL_MIN, API_PROTOCOL_MAX)
                .ok_or_else(|| {
                    AppError::new(
                        ErrorCode::ProtocolInvalid,
                        "The client and daemon have no common API protocol version.",
                    )
                    .with_action("Upgrade mc-server-tui so both sides share a protocol version.")
                    .with_evidence(format!(
                        "client protocol {}..={API_PROTOCOL_MAX}; daemon protocol {}..={}",
                        API_PROTOCOL_MIN, info.api_protocol_min, info.api_protocol_max
                    ))
                }),
            ApiResponse::Error(error) => Err(error),
            _ => Err(AppError::new(
                ErrorCode::ProtocolInvalid,
                "The daemon returned an unexpected response to a version probe.",
            )),
        }
    }

    async fn exchange(
        &self,
        protocol_version: u16,
        request: ApiRequest,
    ) -> Result<ApiResponse, AppError> {
        let connect = tokio::time::timeout(CONNECT_TIMEOUT, UnixStream::connect(&self.socket))
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
            .with_evidence(format!("{}: {error}", self.socket.display()))
        })?;
        let envelope = ApiRequestEnvelope::with_protocol(protocol_version, request);
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
        if envelope.protocol_version != protocol_version {
            return Err(AppError::new(
                ErrorCode::ProtocolInvalid,
                "The daemon replied with a different protocol version than requested.",
            )
            .with_action("Restart the daemon and retry; a version mismatch usually means the daemon was replaced during the request.")
            .with_evidence(format!(
                "daemon protocol version: {}; requested protocol version: {protocol_version}",
                envelope.protocol_version
            )));
        }
        Ok(envelope.payload)
    }
}

fn protocol_io_error(error: io::Error) -> AppError {
    AppError::new(
        ErrorCode::SourceUnavailable,
        "The client connection to the daemon failed.",
    )
    .with_action("Check that the daemon is still running, then retry.")
    .with_evidence(error.to_string())
}

#[cfg(test)]
mod tests {
    use std::{io, path::PathBuf};

    use mc_core::{
        ApiRequest, ApiRequestEnvelope, ApiResponse, ApiResponseEnvelope, ErrorCode,
        InstanceOperation, ServerInfo, API_PROTOCOL_MAX, API_PROTOCOL_MIN,
    };
    use tokio::{
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
        net::{UnixListener, UnixStream},
    };
    use uuid::Uuid;

    use super::DaemonClient;

    async fn read_request(reader: &mut BufReader<UnixStream>) -> ApiRequestEnvelope {
        let mut line = String::new();
        reader.read_line(&mut line).await.expect("read request");
        serde_json::from_str(&line).expect("decode request envelope")
    }

    async fn write_response(stream: &mut UnixStream, protocol: u16, payload: ApiResponse) {
        let encoded = serde_json::to_vec(&ApiResponseEnvelope::with_protocol(protocol, payload))
            .expect("encode response");
        stream
            .write_all(&[encoded, b"\n".to_vec()].concat())
            .await
            .expect("write response");
    }

    fn socket_path() -> PathBuf {
        std::env::temp_dir().join(format!("mc-client-{}.sock", Uuid::new_v4()))
    }

    fn cleanup(socket: &PathBuf) {
        let cleanup = std::fs::remove_file(socket);
        assert!(cleanup.is_ok() || cleanup.as_ref().unwrap_err().kind() == io::ErrorKind::NotFound);
    }

    #[tokio::test]
    async fn negotiates_the_highest_common_protocol() {
        let socket = socket_path();
        let listener = UnixListener::bind(&socket).expect("bind test daemon socket");
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept probe");
            let mut reader = BufReader::new(stream);
            let probe = read_request(&mut reader).await;
            assert_eq!(probe.protocol_version, API_PROTOCOL_MIN);
            assert!(matches!(probe.payload, ApiRequest::ServerInfo));
            write_response(
                reader.get_mut(),
                API_PROTOCOL_MIN,
                ApiResponse::ServerInfo(ServerInfo::current()),
            )
            .await;

            let (stream, _) = listener.accept().await.expect("accept request");
            let mut reader = BufReader::new(stream);
            let request = read_request(&mut reader).await;
            assert_eq!(request.protocol_version, API_PROTOCOL_MAX);
            assert!(matches!(request.payload, ApiRequest::Ping));
            write_response(reader.get_mut(), API_PROTOCOL_MAX, ApiResponse::Pong).await;
        });

        let response = DaemonClient::new(PathBuf::from(&socket))
            .request(ApiRequest::Ping)
            .await
            .expect("request response");
        server.await.expect("fake daemon task");
        cleanup(&socket);

        assert!(matches!(response, ApiResponse::Pong));
    }

    #[tokio::test]
    async fn negotiates_down_to_an_older_daemon_and_rejects_new_operations() {
        let socket = socket_path();
        let listener = UnixListener::bind(&socket).expect("bind test daemon socket");
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept probe");
            let mut reader = BufReader::new(stream);
            let probe = read_request(&mut reader).await;
            assert_eq!(probe.protocol_version, API_PROTOCOL_MIN);
            let mut info = ServerInfo::current();
            info.api_protocol_max = API_PROTOCOL_MIN;
            write_response(
                reader.get_mut(),
                API_PROTOCOL_MIN,
                ApiResponse::ServerInfo(info),
            )
            .await;

            let (stream, _) = listener.accept().await.expect("accept ping");
            let mut reader = BufReader::new(stream);
            let request = read_request(&mut reader).await;
            assert_eq!(request.protocol_version, API_PROTOCOL_MIN);
            assert!(matches!(request.payload, ApiRequest::Ping));
            write_response(reader.get_mut(), API_PROTOCOL_MIN, ApiResponse::Pong).await;

            let extra =
                tokio::time::timeout(std::time::Duration::from_millis(200), listener.accept())
                    .await;
            assert!(
                extra.is_err(),
                "a newer operation must not be sent to a v8 daemon"
            );
        });

        let client = DaemonClient::new(PathBuf::from(&socket));
        let response = client
            .request(ApiRequest::Ping)
            .await
            .expect("v8 daemon accepts Ping");
        assert!(matches!(response, ApiResponse::Pong));
        let error = client
            .request(ApiRequest::ManageInstance {
                instance_id: Uuid::nil(),
                action: InstanceOperation::ListAddons,
            })
            .await
            .expect_err("management requires protocol 9");
        assert_eq!(error.code, ErrorCode::ProtocolInvalid);
        assert!(error
            .evidence
            .as_deref()
            .is_some_and(|evidence| evidence.contains("requires API protocol 9")));
        let error = client
            .request(ApiRequest::RestartServer {
                instance_id: Uuid::nil(),
                timeout_seconds: 20,
            })
            .await
            .expect_err("restart requires protocol 11");
        assert_eq!(error.code, ErrorCode::ProtocolInvalid);
        assert!(error
            .evidence
            .as_deref()
            .is_some_and(|evidence| evidence.contains("requires API protocol 11")));

        server.await.expect("fake daemon task");
        cleanup(&socket);
    }
}
