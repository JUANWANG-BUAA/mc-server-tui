use std::{io, os::unix::process::CommandExt, sync::Arc, time::Duration};

use mc_core::{AppError, ErrorCode, InstanceSummary};
use mc_instance::InstanceService;
use tokio::{process::Command, sync::Mutex, task::JoinSet};
use uuid::Uuid;

use crate::storage::{self, RuntimeRecord};

mod control;
mod logs;

pub(crate) use control::send_console_command;

const RUNNER_STARTUP_TIMEOUT: Duration = Duration::from_secs(8);

/// Resident owner for one Java server. It survives management daemon restarts,
/// owns Java's pipes, and exposes only a private local command socket.
pub async fn run_instance_runner(
    instances: InstanceService,
    instance_id: Uuid,
) -> Result<(), AppError> {
    let instance = instances.get_async(instance_id).await?;
    let runner_pid = std::process::id();
    let marker = current_runner_marker(instance_id);
    wait_for_runtime_record(&instance, runner_pid, &marker).await?;
    let socket_path = control::control_socket_path(instance_id);
    let listener = control::bind_control_socket(&socket_path).await?;

    let java_marker = format!("-Dmc-server-tui-instance={instance_id}");
    let mut command = Command::new(&instance.java_path);
    command
        .arg(format!("-Xms{}M", instance.min_memory_mb))
        .arg(format!("-Xmx{}M", instance.max_memory_mb))
        .arg(&java_marker)
        .arg("-jar")
        .arg("server.jar")
        .arg("nogui")
        .arg("--port")
        .arg(instance.port.to_string())
        .current_dir(instance.directory.join("server"))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            let failure = AppError::new(
                ErrorCode::ServerStartFailed,
                "Could not start the Java server.",
            )
            .with_action("Check the selected Java executable and instance log.")
            .with_evidence(error.to_string());
            write_runner_error(&instance, runner_pid, &failure.reason).await;
            return Err(failure);
        }
    };
    let Some(java_pid) = child.id() else {
        let _ = child.start_kill();
        let error = AppError::new(
            ErrorCode::ServerStartFailed,
            "The operating system did not return a Java process ID.",
        );
        write_runner_error(&instance, runner_pid, &error.reason).await;
        return Err(error);
    };

    update_runtime_record(&instance, runner_pid, |record| {
        record.java_pid = Some(java_pid);
    })
    .await?;

    let (stdin, stdout, stderr) =
        match (child.stdin.take(), child.stdout.take(), child.stderr.take()) {
            (Some(stdin), Some(stdout), Some(stderr)) => (stdin, stdout, stderr),
            _ => {
                let error = AppError::new(
                    ErrorCode::ServerStartFailed,
                    "The Java process pipes could not be prepared.",
                );
                write_runner_error(&instance, runner_pid, &error.reason).await;
                let _ = child.start_kill();
                return Err(error);
            }
        };

    let stdin = Arc::new(Mutex::new(stdin));
    let capture = logs::OutputCapture::start(instance.clone(), stdout, stderr);
    let mut connections = JoinSet::new();

    let status = loop {
        tokio::select! {
            result = child.wait() => {
                break result.map_err(|error| io_error("Could not wait for the Java server.", error))?;
            }
            accepted = listener.accept() => match accepted {
                Ok((stream, _)) => {
                    let stdin = Arc::clone(&stdin);
                    connections.spawn(async move {
                        control::serve_console_request(stream, stdin).await
                    });
                }
                Err(error) => {
                    tracing::warn!(%error, "runner control socket accept failed");
                }
            },
            Some(_) = connections.join_next(), if !connections.is_empty() => {}
        }
    };

    connections.abort_all();
    while connections.join_next().await.is_some() {}
    let log_result = capture.wait().await;
    let code = status.code();
    update_runtime_record(&instance, runner_pid, |record| {
        record.java_pid = Some(java_pid);
        record.last_exit_code = code;
    })
    .await?;
    drop(listener);
    if let Err(error) = tokio::fs::remove_file(&socket_path).await {
        if error.kind() != io::ErrorKind::NotFound {
            tracing::warn!(path = %socket_path.display(), %error, "could not remove runner control socket");
        }
    }

    log_result?;
    Ok(())
}

fn current_runner_marker(instance_id: Uuid) -> String {
    #[cfg(not(test))]
    {
        format!("--internal-runner={instance_id}")
    }
    #[cfg(test)]
    {
        let _ = instance_id;
        "tests::runner_entry".to_owned()
    }
}

async fn wait_for_runtime_record(
    instance: &InstanceSummary,
    runner_pid: u32,
    marker: &str,
) -> Result<RuntimeRecord, AppError> {
    let deadline = tokio::time::Instant::now() + RUNNER_STARTUP_TIMEOUT;
    loop {
        if let Some(record) = storage::read_record(instance).await? {
            if record.pid == runner_pid && record.marker == marker {
                return Ok(record);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(AppError::new(
                ErrorCode::ServerStatusUnknown,
                "The runner did not find its matching runtime record.",
            ));
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

async fn update_runtime_record<F>(
    instance: &InstanceSummary,
    runner_pid: u32,
    update: F,
) -> Result<(), AppError>
where
    F: FnOnce(&mut RuntimeRecord),
{
    let deadline = tokio::time::Instant::now() + RUNNER_STARTUP_TIMEOUT;
    let mut update = Some(update);
    loop {
        match storage::acquire_lifecycle_lock(instance).await {
            Ok(_lock) => {
                let Some(mut record) = storage::read_record(instance).await? else {
                    return Err(AppError::new(
                        ErrorCode::ServerStatusUnknown,
                        "Runner runtime record disappeared.",
                    ));
                };
                if record.pid != runner_pid {
                    return Err(AppError::new(
                        ErrorCode::ServerStatusUnknown,
                        "Runner no longer owns the runtime record.",
                    ));
                }
                update.take().expect("single runtime update")(&mut record);
                storage::write_record(instance, &record).await?;
                return Ok(());
            }
            Err(error) if error.code == ErrorCode::InstanceBusy => {
                if tokio::time::Instant::now() >= deadline {
                    return Err(error);
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            Err(error) => return Err(error),
        }
    }
}

async fn write_runner_error(instance: &InstanceSummary, runner_pid: u32, message: &str) {
    let message = message.to_owned();
    let _ = update_runtime_record(instance, runner_pid, move |record| {
        record.last_error = Some(message);
        record.last_exit_code = Some(1);
    })
    .await;
}

pub(crate) fn runner_process_command(instance: &InstanceSummary) -> Result<Command, AppError> {
    let executable = std::env::current_exe()
        .map_err(|error| io_error("Could not resolve the daemon executable.", error))?;
    let state_dir = instance
        .directory
        .parent()
        .and_then(|instances| instances.parent())
        .ok_or_else(|| {
            AppError::new(
                ErrorCode::Internal,
                "Could not resolve the instance state directory.",
            )
        })?;
    let mut command = Command::new(executable);
    #[cfg(not(test))]
    command
        .arg(format!("--internal-runner={}", instance.id))
        .arg(format!("--state-dir={}", state_dir.display()));
    #[cfg(test)]
    command
        .args(["--exact", "tests::runner_entry", "--nocapture"])
        .env("MC_SUPERVISOR_RUNNER_STATE_DIR", state_dir)
        .env("MC_SUPERVISOR_RUNNER_INSTANCE_ID", instance.id.to_string());
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    command.as_std_mut().process_group(0);
    Ok(command)
}

pub(crate) fn runner_marker(instance_id: Uuid) -> String {
    current_runner_marker(instance_id)
}

fn io_error(reason: &str, error: io::Error) -> AppError {
    AppError::new(ErrorCode::Internal, reason).with_evidence(error.to_string())
}
