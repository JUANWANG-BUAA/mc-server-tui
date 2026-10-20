//! Per-instance Java process lifecycle supervision.
//!
//! The daemon owns lifecycle requests; server stdout/stderr and stdin are
//! redirected to instance files so the Java process does not depend on the
//! TUI, daemon socket, or terminal remaining open.

mod process;
mod runner;
mod storage;

pub use runner::run_instance_runner;

use std::{collections::HashMap, sync::Arc, time::Duration};

use chrono::Utc;
use mc_core::{
    AppError, ErrorCode, InstanceDetail, InstanceSummary, ServerLogChunk, ServerSnapshot,
    ServerState, StoppedInstanceLease,
};
use mc_instance::InstanceService;
use tokio::sync::{Mutex, OwnedMutexGuard};
use uuid::Uuid;

use crate::{process::ProcessIdentity, storage::RuntimeRecord};

const DEFAULT_STOP_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_STOP_TIMEOUT: Duration = Duration::from_secs(25);
const STOP_POLL_INTERVAL: Duration = Duration::from_millis(200);
const LIFECYCLE_LOCK_WAIT: Duration = Duration::from_secs(3);
const MAX_LOG_CHUNK_BYTES: usize = 64 * 1024;
const MAX_CONSOLE_COMMAND_BYTES: usize = 512;
const MAX_CONSOLE_LOG_BYTES: u64 = 32 * 1024 * 1024;
const SERVER_READY_MARKER: &str = "Done (";

/// Coordinates lifecycle operations while preserving per-instance isolation.
#[derive(Clone)]
pub struct ProcessSupervisor {
    instances: InstanceService,
    instance_locks: Arc<Mutex<HashMap<Uuid, Arc<Mutex<()>>>>>,
    /// Held for one complete user-visible lifecycle command. Status and log
    /// reads deliberately do not acquire this lock, so a restart can wait for
    /// Java to exit without making the instance uninspectable.
    coordination_locks: Arc<Mutex<HashMap<Uuid, Arc<Mutex<()>>>>>,
}

/// Holds the per-instance lifecycle locks while a stopped-world operation runs.
///
/// Keep this guard alive for the entire filesystem operation. It prevents a
/// concurrent start/stop request from changing server state while the caller
/// reads or replaces world files.
#[must_use = "dropping this guard releases the stopped-instance reservation"]
pub struct StoppedInstanceGuard {
    instance: InstanceSummary,
    _coordination_guard: OwnedMutexGuard<()>,
    _operation_guard: OwnedMutexGuard<()>,
    _lifecycle_lock: std::fs::File,
}

impl StoppedInstanceGuard {
    pub fn instance(&self) -> &InstanceSummary {
        &self.instance
    }
}

impl StoppedInstanceLease for StoppedInstanceGuard {
    fn instance(&self) -> &InstanceSummary {
        &self.instance
    }
}

impl ProcessSupervisor {
    pub fn new(instances: InstanceService) -> Self {
        Self {
            instances,
            instance_locks: Arc::default(),
            coordination_locks: Arc::default(),
        }
    }

    pub async fn detail(&self, instance_id: Uuid) -> Result<InstanceDetail, AppError> {
        let (instance, _guard, _file_lock) = self.lock_instance(instance_id).await?;
        let server = self.snapshot_locked(&instance).await?;
        Ok(InstanceDetail { instance, server })
    }

    /// Reserve an instance for an offline world operation.
    ///
    /// The returned guard keeps both in-process and cross-process lifecycle
    /// locks held until it is dropped. Callers should move blocking filesystem
    /// work to `spawn_blocking` while retaining the guard.
    pub async fn lock_stopped_instance(
        &self,
        instance_id: Uuid,
    ) -> Result<StoppedInstanceGuard, AppError> {
        let coordination_guard = self.lock_coordination(instance_id).await;
        let (instance, operation_guard, lifecycle_lock) = self.lock_instance(instance_id).await?;
        if let Some(record) = storage::read_record(&instance).await? {
            match process::inspect(&record).await? {
                ProcessIdentity::Matches => {
                    return Err(AppError::new(
                        ErrorCode::InstanceBusy,
                        "Stop the server before changing world files.",
                    )
                    .with_action("Request a graceful stop, wait until it exits, then retry.")
                    .retryable());
                }
                ProcessIdentity::Unverified => return Err(unknown_process_error(record.pid)),
                ProcessIdentity::NotRunning => {}
            }
        }
        if !port_available(instance.port).await {
            return Err(AppError::new(
                ErrorCode::ServerStatusUnknown,
                "The configured server port is occupied by an unverified process.",
            )
            .with_action("Inspect the process using the port before changing world files.")
            .retryable());
        }
        Ok(StoppedInstanceGuard {
            instance,
            _coordination_guard: coordination_guard,
            _operation_guard: operation_guard,
            _lifecycle_lock: lifecycle_lock,
        })
    }

    /// Start is idempotent. A live PID is reused only when its unique instance
    /// marker can be verified; otherwise the state is reported as unknown.
    pub async fn start(&self, instance_id: Uuid) -> Result<ServerSnapshot, AppError> {
        let _coordination_guard = self.lock_coordination(instance_id).await;
        self.start_inner(instance_id).await
    }

    async fn start_inner(&self, instance_id: Uuid) -> Result<ServerSnapshot, AppError> {
        let (instance, operation_guard, lifecycle_lock) = self.lock_instance(instance_id).await?;
        let prior = storage::read_record(&instance).await?;
        if let Some(record) = prior.as_ref() {
            match process::inspect(record).await? {
                ProcessIdentity::Matches => {
                    return self.snapshot_with_record(&instance, record).await
                }
                ProcessIdentity::Unverified => {
                    return Ok(snapshot(
                        &instance,
                        ServerState::Unknown,
                        Some(record.pid),
                        Some(record),
                        "A process is using the recorded PID but its instance identity could not be verified.",
                    ));
                }
                ProcessIdentity::NotRunning => {}
            }
        }

        if !port_available(instance.port).await {
            return Ok(snapshot(
                &instance,
                ServerState::Unknown,
                prior.as_ref().map(|record| record.pid),
                prior.as_ref(),
                "The configured port is occupied; no server process was started because ownership is unknown.",
            ));
        }

        self.validate_installation(&instance).await?;
        storage::rotate_console_log(&instance).await?;
        let started_at = Utc::now();
        let marker = runner::runner_marker(instance_id);
        let mut command = runner::runner_process_command(&instance)?;
        let child = command.spawn().map_err(|error| {
            AppError::new(
                ErrorCode::ServerStartFailed,
                "Could not start the instance runner.",
            )
            .with_action("Check the daemon executable and instance state directory.")
            .with_evidence(error.to_string())
        })?;
        let Some(pid) = child.id() else {
            return Err(AppError::new(
                ErrorCode::ServerStartFailed,
                "The operating system did not return a runner process ID.",
            ));
        };
        let record = RuntimeRecord {
            schema_version: storage::RUNTIME_SCHEMA_VERSION,
            instance_id,
            pid,
            java_pid: None,
            marker,
            started_at,
            stopping: false,
            last_exit_code: None,
            last_error: None,
        };
        storage::write_record(&instance, &record).await?;

        // The runner must acquire the lifecycle lock before it publishes the
        // Java PID. Release both locks, then wait briefly for that update so
        // `ServerSnapshot.pid` consistently means the Java process ID.
        drop(lifecycle_lock);
        drop(operation_guard);
        let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
        let mut current = record;
        while current.java_pid.is_none() && tokio::time::Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(25)).await;
            if let Some(updated) = storage::read_record(&instance).await? {
                if updated.pid != pid {
                    return self.snapshot_with_record(&instance, &updated).await;
                }
                current = updated;
            }
        }
        self.snapshot_with_record(&instance, &current).await
    }

    /// Send the Minecraft `stop` command and wait for the verified process to exit.
    /// A timeout never escalates to a signal; callers can inspect and retry safely.
    pub async fn stop(
        &self,
        instance_id: Uuid,
        timeout: Option<Duration>,
    ) -> Result<ServerSnapshot, AppError> {
        let _coordination_guard = self.lock_coordination(instance_id).await;
        self.stop_inner(instance_id, timeout).await
    }

    async fn stop_inner(
        &self,
        instance_id: Uuid,
        timeout: Option<Duration>,
    ) -> Result<ServerSnapshot, AppError> {
        let (instance, operation_guard, lifecycle_lock) = self.lock_instance(instance_id).await?;
        let Some(mut record) = storage::read_record(&instance).await? else {
            return Err(AppError::new(
                ErrorCode::ServerNotRunning,
                "This instance has not been started by the supervisor.",
            ));
        };
        match process::inspect(&record).await? {
            ProcessIdentity::NotRunning => {
                return self.snapshot_with_record(&instance, &record).await;
            }
            ProcessIdentity::Unverified => {
                return Err(unknown_process_error(record.pid));
            }
            ProcessIdentity::Matches => {}
        }

        runner::send_console_command(instance_id, "stop").await?;
        record.stopping = true;
        storage::write_record(&instance, &record).await?;

        // Status and log refreshes must remain available while a server exits.
        drop(lifecycle_lock);
        drop(operation_guard);

        let timeout = timeout
            .unwrap_or(DEFAULT_STOP_TIMEOUT)
            .min(MAX_STOP_TIMEOUT);
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let current = storage::read_record(&instance)
                .await?
                .unwrap_or_else(|| record.clone());
            if current.pid != record.pid {
                return self.snapshot_with_record(&instance, &current).await;
            }
            match process::inspect(&current).await? {
                ProcessIdentity::NotRunning => {
                    return self.snapshot_with_record(&instance, &current).await;
                }
                ProcessIdentity::Unverified => {
                    return Err(unknown_process_error(record.pid));
                }
                ProcessIdentity::Matches if tokio::time::Instant::now() >= deadline => {
                    return Err(AppError::new(
                        ErrorCode::ServerStopTimeout,
                        "The server did not exit before the graceful stop timeout.",
                    )
                    .with_action("Inspect the server status and logs, then retry the stop request.")
                    .with_evidence(format!("instance_id={instance_id}; pid={}", record.pid)));
                }
                ProcessIdentity::Matches => tokio::time::sleep(STOP_POLL_INTERVAL).await,
            }
        }
    }

    /// Gracefully stop a verified running server and start the same instance
    /// again. One coordination lock covers the whole sequence, so another
    /// start/stop/backup/restart command cannot slip into the stopped window.
    pub async fn restart(
        &self,
        instance_id: Uuid,
        timeout: Option<Duration>,
    ) -> Result<ServerSnapshot, AppError> {
        let _coordination_guard = self.lock_coordination(instance_id).await;
        let current = self.detail(instance_id).await?;
        if !matches!(
            current.server.state,
            ServerState::Starting | ServerState::Running
        ) {
            return Err(AppError::new(
                ErrorCode::ServerNotRunning,
                "Restart requires a running server.",
            )
            .with_action("Start the instance first, then retry the restart request."));
        }
        let stopped = self.stop_inner(instance_id, timeout).await?;
        if stopped.state != ServerState::Stopped {
            return Err(AppError::new(
                ErrorCode::ServerStartFailed,
                "The server did not stop cleanly; restart was aborted before starting.",
            )
            .with_action("Inspect the instance status and logs, then start it manually when safe.")
            .with_evidence(format!(
                "instance_id={instance_id}; state={:?}; last_exit_code={:?}",
                stopped.state, stopped.last_exit_code
            )));
        }
        self.start_inner(instance_id).await
    }

    pub async fn send_console_command(
        &self,
        instance_id: Uuid,
        command: &str,
    ) -> Result<(), AppError> {
        validate_console_command(command)?;
        let (instance, _guard, _file_lock) = self.lock_instance(instance_id).await?;
        let Some(record) = storage::read_record(&instance).await? else {
            return Err(AppError::new(
                ErrorCode::ServerNotRunning,
                "The server is not running.",
            ));
        };
        if process::inspect(&record).await? != ProcessIdentity::Matches {
            return Err(unknown_process_error(record.pid));
        }
        let snapshot = self.snapshot_with_record(&instance, &record).await?;
        if snapshot.state != ServerState::Running {
            return Err(AppError::new(
                ErrorCode::ServerNotRunning,
                "Console commands are available after the server reports readiness.",
            ));
        }
        runner::send_console_command(instance_id, command).await
    }

    pub async fn read_logs(
        &self,
        instance_id: Uuid,
        after_offset: u64,
        max_bytes: u32,
    ) -> Result<ServerLogChunk, AppError> {
        let instance = self.instances.get_async(instance_id).await?;
        storage::read_logs(
            &instance,
            after_offset,
            max_bytes.min(MAX_LOG_CHUNK_BYTES as u32),
        )
        .await
    }

    async fn validate_installation(&self, instance: &InstanceSummary) -> Result<(), AppError> {
        let java_metadata = tokio::fs::symlink_metadata(&instance.java_path)
            .await
            .map_err(|error| {
                AppError::new(
                    ErrorCode::ServerStartFailed,
                    "The selected Java executable is unavailable.",
                )
                .with_evidence(error.to_string())
            })?;
        use std::os::unix::fs::PermissionsExt;
        if !java_metadata.file_type().is_file() || java_metadata.permissions().mode() & 0o111 == 0 {
            return Err(AppError::new(
                ErrorCode::ServerStartFailed,
                "The selected Java path is not an executable regular file.",
            ));
        }
        let server_dir = instance.directory.join("server");
        let directory_metadata = tokio::fs::symlink_metadata(&server_dir)
            .await
            .map_err(|error| io_error("Could not inspect the server directory.", error))?;
        if !directory_metadata.file_type().is_dir() {
            return Err(AppError::new(
                ErrorCode::ServerStartFailed,
                "The server working directory is not a real directory.",
            ));
        }
        let jar = server_dir.join("server.jar");
        let jar_metadata = tokio::fs::symlink_metadata(&jar)
            .await
            .map_err(|error| io_error("The installed server JAR is unavailable.", error))?;
        if !jar_metadata.file_type().is_file() || jar_metadata.len() == 0 {
            return Err(AppError::new(
                ErrorCode::ServerStartFailed,
                "The installed server JAR is not a non-empty regular file.",
            ));
        }
        let logs_dir = instance.directory.join("logs");
        let logs_metadata = tokio::fs::symlink_metadata(&logs_dir)
            .await
            .map_err(|error| io_error("Could not inspect the instance log directory.", error))?;
        if !logs_metadata.file_type().is_dir() {
            return Err(AppError::new(
                ErrorCode::ServerStartFailed,
                "The instance log path is not a real directory.",
            ));
        }
        Ok(())
    }

    async fn snapshot_locked(
        &self,
        instance: &InstanceSummary,
    ) -> Result<ServerSnapshot, AppError> {
        let Some(record) = storage::read_record(instance).await? else {
            let state = if port_available(instance.port).await {
                ServerState::Installed
            } else {
                ServerState::Unknown
            };
            return Ok(snapshot(
                instance,
                state,
                None,
                None,
                if state == ServerState::Installed {
                    "Installed and stopped."
                } else {
                    "The configured port is occupied; process ownership is unknown."
                },
            ));
        };
        self.snapshot_with_record(instance, &record).await
    }

    async fn snapshot_with_record(
        &self,
        instance: &InstanceSummary,
        record: &RuntimeRecord,
    ) -> Result<ServerSnapshot, AppError> {
        let identity = process::inspect(record).await?;
        let failure_diagnosis = if identity == ProcessIdentity::NotRunning
            && (record.last_error.is_some() || record.last_exit_code.is_some_and(|code| code != 0))
        {
            self.diagnose_failed_start(instance, record).await?
        } else {
            None
        };
        let configured_port_available = if identity == ProcessIdentity::NotRunning {
            port_available(instance.port).await
        } else {
            true
        };
        let (state, pid, message) = match identity {
            ProcessIdentity::Matches => {
                let ready = storage::log_has_ready_marker(instance, SERVER_READY_MARKER).await?;
                if record.stopping {
                    (
                        ServerState::Stopping,
                        record.java_pid,
                        "Graceful stop requested.",
                    )
                } else if ready {
                    (
                        ServerState::Running,
                        record.java_pid,
                        "Server is ready.",
                    )
                } else {
                    (
                        ServerState::Starting,
                        record.java_pid,
                        "Waiting for the server readiness message.",
                    )
                }
            }
            ProcessIdentity::NotRunning if !configured_port_available => (
                ServerState::Unknown,
                None,
                "The recorded process is gone but the configured port is occupied by an unknown process.",
            ),
            ProcessIdentity::NotRunning if record.last_error.is_some() => (
                ServerState::Failed,
                None,
                failure_diagnosis
                    .unwrap_or("Server failed to start; inspect the instance logs."),
            ),
            ProcessIdentity::NotRunning => match record.last_exit_code {
                Some(0) => (ServerState::Stopped, None, "Server stopped successfully."),
                Some(_code) => (
                    ServerState::Failed,
                    None,
                    failure_diagnosis.unwrap_or("Server exited with an error."),
                ),
                None if record.stopping => (ServerState::Stopped, None, "Server stopped."),
                None => (
                    ServerState::Stopped,
                    None,
                    "Server process is no longer running.",
                ),
            },
            ProcessIdentity::Unverified => (
                ServerState::Unknown,
                Some(record.pid),
                "The recorded PID could not be verified; no process signal will be sent.",
            ),
        };
        Ok(snapshot(instance, state, pid, Some(record), message))
    }

    async fn diagnose_failed_start(
        &self,
        instance: &InstanceSummary,
        record: &RuntimeRecord,
    ) -> Result<Option<&'static str>, AppError> {
        let log = storage::read_logs(instance, u64::MAX, 64 * 1024).await?;
        let mut evidence = record.last_error.clone().unwrap_or_default();
        if !evidence.is_empty() && !log.content.is_empty() {
            evidence.push('\n');
        }
        evidence.push_str(&log.content);
        Ok(diagnose_failure_output(&evidence))
    }

    async fn lock_instance(
        &self,
        instance_id: Uuid,
    ) -> Result<(InstanceSummary, OwnedMutexGuard<()>, std::fs::File), AppError> {
        let instance = self.instances.get_async(instance_id).await?;
        let server_dir = instance.directory.join("server");
        let server_metadata = tokio::fs::symlink_metadata(&server_dir)
            .await
            .map_err(|error| {
                io_error(
                    "Could not inspect server directory for a lifecycle lock.",
                    error,
                )
            })?;
        if !server_metadata.file_type().is_dir() {
            return Err(AppError::new(
                ErrorCode::Internal,
                "Server working directory is not a real directory.",
            ));
        }
        let lock = {
            let mut locks = self.instance_locks.lock().await;
            Arc::clone(
                locks
                    .entry(instance_id)
                    .or_insert_with(|| Arc::new(Mutex::new(()))),
            )
        };
        let memory_guard = lock.lock_owned().await;
        let deadline = tokio::time::Instant::now() + LIFECYCLE_LOCK_WAIT;
        let file_lock = loop {
            match storage::acquire_lifecycle_lock(&instance).await {
                Ok(lock) => break lock,
                Err(error)
                    if error.code == ErrorCode::InstanceBusy
                        && tokio::time::Instant::now() < deadline =>
                {
                    tokio::time::sleep(Duration::from_millis(25)).await;
                }
                Err(error) => return Err(error),
            }
        };
        Ok((instance, memory_guard, file_lock))
    }

    async fn lock_coordination(&self, instance_id: Uuid) -> OwnedMutexGuard<()> {
        let lock = {
            let mut locks = self.coordination_locks.lock().await;
            Arc::clone(
                locks
                    .entry(instance_id)
                    .or_insert_with(|| Arc::new(Mutex::new(()))),
            )
        };
        lock.lock_owned().await
    }
}

fn snapshot(
    instance: &InstanceSummary,
    state: ServerState,
    pid: Option<u32>,
    record: Option<&RuntimeRecord>,
    message: &str,
) -> ServerSnapshot {
    ServerSnapshot {
        instance_id: instance.id,
        state,
        pid,
        port: instance.port,
        address: format!("127.0.0.1:{}", instance.port),
        started_at: record.map(|record| record.started_at),
        last_exit_code: record.and_then(|record| record.last_exit_code),
        message: message.to_owned(),
    }
}

fn validate_console_command(command: &str) -> Result<(), AppError> {
    if command.trim().is_empty()
        || command.len() > MAX_CONSOLE_COMMAND_BYTES
        || command.chars().any(|character| character.is_control())
    {
        return Err(AppError::new(
            ErrorCode::InvalidConsoleCommand,
            "Console command must contain 1 to 512 printable bytes and no line breaks.",
        ));
    }
    Ok(())
}

fn diagnose_failure_output(output: &str) -> Option<&'static str> {
    let output = output.to_ascii_lowercase();
    if output.contains("unsupportedclassversionerror")
        || output.contains("class file version")
        || output.contains("requires java") && output.contains("current java")
    {
        return Some("Java is incompatible with this server; select a newer Java runtime.");
    }
    if output.contains("failed to bind to port") || output.contains("address already in use") {
        return Some(
            "The configured port is in use; stop the other process or choose another port.",
        );
    }
    if output.contains("you need to agree to the eula") || output.contains("eula=false") {
        return Some("The Minecraft EULA has not been accepted; accept it before starting.");
    }
    if output.contains("outofmemoryerror")
        || output.contains("java heap space")
        || output.contains("could not reserve enough space for object heap")
    {
        return Some("Java ran out of memory; raise the instance memory limit.");
    }
    if output.contains("invalid or corrupt jarfile")
        || output.contains("unable to access jarfile")
        || output.contains("could not find or load main class")
    {
        return Some(
            "server.jar is missing, corrupt, or incompatible; reinstall the locked build.",
        );
    }
    None
}

async fn port_available(port: u16) -> bool {
    match tokio::net::TcpListener::bind(("0.0.0.0", port)).await {
        Ok(listener) => {
            drop(listener);
            true
        }
        Err(_) => false,
    }
}

fn unknown_process_error(pid: u32) -> AppError {
    AppError::new(
        ErrorCode::ServerStatusUnknown,
        "The server process identity could not be verified.",
    )
    .with_action("Inspect the process and instance logs manually; the supervisor will not signal an unverified PID.")
    .with_evidence(format!("pid={pid}"))
}

fn io_error(reason: &str, error: std::io::Error) -> AppError {
    AppError::new(ErrorCode::Internal, reason).with_evidence(error.to_string())
}

#[cfg(test)]
mod tests;
