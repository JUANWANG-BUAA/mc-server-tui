use std::{path::PathBuf, sync::Arc};

use mc_core::{
    ApiRequest, ApiResponse, AppError, ErrorCode, TaskKind, TaskPhase, TaskProgress, TaskResult,
    TaskSnapshot,
};
use mc_instance::InstanceSpec;
use mc_task::TaskReporter;

use crate::backup::{create_world_backup_task, list_world_backups_task, verify_world_backup_task};
use crate::backup_restore::{plan_world_backup_restore_task, restore_world_backup_task};
use crate::state::DaemonState;

struct PaperCreateCommand {
    name: String,
    game_version: String,
    build_id: Option<u64>,
    java_path: PathBuf,
    min_memory_mb: u32,
    max_memory_mb: u32,
    port: u16,
    accept_eula: bool,
}

pub(crate) async fn dispatch(
    request: ApiRequest,
    state: &DaemonState,
    protocol_version: u16,
) -> ApiResponse {
    let response = match request {
        ApiRequest::ReadServerProperties { instance_id } => {
            crate::management::read_server_properties(state, instance_id)
                .await
                .map(ApiResponse::ServerProperties)
                .unwrap_or_else(ApiResponse::Error)
        }
        ApiRequest::SaveServerProperties {
            instance_id,
            expected_revision,
            values,
        } => {
            crate::management::save_server_properties(state, instance_id, expected_revision, values)
                .await
                .map(ApiResponse::ServerProperties)
                .unwrap_or_else(ApiResponse::Error)
        }
        ApiRequest::ManageInstance {
            instance_id,
            action,
        } => crate::management::dispatch(state, instance_id, action).await,
        ApiRequest::Ping => ApiResponse::Pong,
        ApiRequest::ServerInfo => ApiResponse::ServerInfo(mc_core::ServerInfo::current()),
        ApiRequest::Doctor => ApiResponse::Doctor(mc_system::run_doctor().await),
        ApiRequest::PaperVersions => state
            .provider
            .list_versions()
            .await
            .map(ApiResponse::PaperVersions)
            .unwrap_or_else(ApiResponse::Error),
        ApiRequest::PaperBuilds { game_version } => state
            .provider
            .list_builds(&game_version)
            .await
            .map(ApiResponse::PaperBuilds)
            .unwrap_or_else(ApiResponse::Error),
        ApiRequest::PaperPlan {
            game_version,
            build_id,
        } => state
            .provider
            .resolve_build(&game_version, build_id)
            .await
            .map(|plan| ApiResponse::ArtifactPlan(Box::new(plan)))
            .unwrap_or_else(ApiResponse::Error),
        ApiRequest::ListInstances => state
            .instances
            .list_async()
            .await
            .map(ApiResponse::Instances)
            .unwrap_or_else(ApiResponse::Error),
        ApiRequest::GetInstanceDetail { instance_id } => state
            .supervisor
            .detail(instance_id)
            .await
            .map(ApiResponse::InstanceDetail)
            .unwrap_or_else(ApiResponse::Error),
        ApiRequest::CreatePaperInstance {
            name,
            game_version,
            build_id,
            java_path,
            min_memory_mb,
            max_memory_mb,
            port,
            accept_eula,
        } => match create_paper_task(
            state,
            PaperCreateCommand {
                name,
                game_version,
                build_id,
                java_path,
                min_memory_mb,
                max_memory_mb,
                port,
                accept_eula,
            },
        )
        .await
        {
            Ok(task) => ApiResponse::TaskQueued(task),
            Err(error) => ApiResponse::Error(error),
        },
        ApiRequest::CreateWorldBackup { instance_id } => {
            match create_world_backup_task(
                &state.tasks,
                &state.supervisor,
                state.backups,
                instance_id,
            )
            .await
            {
                Ok(task) => ApiResponse::TaskQueued(task),
                Err(error) => ApiResponse::Error(error),
            }
        }
        ApiRequest::ListWorldBackups {
            instance_id,
            limit,
            cursor,
        } => match list_world_backups_task(
            &state.tasks,
            state.instances.clone(),
            state.backups,
            instance_id,
            limit,
            cursor,
        )
        .await
        {
            Ok(task) => ApiResponse::TaskQueued(task),
            Err(error) => ApiResponse::Error(error),
        },
        ApiRequest::VerifyWorldBackup {
            instance_id,
            backup_id,
        } => match verify_world_backup_task(
            &state.tasks,
            state.instances.clone(),
            state.backups,
            instance_id,
            backup_id,
        )
        .await
        {
            Ok(task) => ApiResponse::TaskQueued(task),
            Err(error) => ApiResponse::Error(error),
        },
        ApiRequest::PlanWorldBackupRestore {
            instance_id,
            backup_id,
        } => match plan_world_backup_restore_task(
            &state.tasks,
            &state.supervisor,
            state.backups,
            instance_id,
            backup_id,
        )
        .await
        {
            Ok(task) => ApiResponse::TaskQueued(task),
            Err(error) => ApiResponse::Error(error),
        },
        ApiRequest::RestoreWorldBackup {
            instance_id,
            backup_id,
            confirmation_token,
        } => match restore_world_backup_task(
            &state.tasks,
            &state.supervisor,
            state.backups,
            instance_id,
            backup_id,
            confirmation_token,
        )
        .await
        {
            Ok(task) => ApiResponse::TaskQueued(task),
            Err(error) => ApiResponse::Error(error),
        },
        ApiRequest::StartServer { instance_id } => match start_server(state, instance_id).await {
            Ok(snapshot) => ApiResponse::ServerStatus(snapshot),
            Err(error) => ApiResponse::Error(error),
        },
        ApiRequest::StopServer {
            instance_id,
            timeout_seconds,
        } => state
            .supervisor
            .stop(
                instance_id,
                (timeout_seconds != 0)
                    .then(|| std::time::Duration::from_secs(u64::from(timeout_seconds.min(60)))),
            )
            .await
            .map(ApiResponse::ServerStatus)
            .unwrap_or_else(ApiResponse::Error),
        ApiRequest::SendConsoleCommand {
            instance_id,
            command,
        } => state
            .supervisor
            .send_console_command(instance_id, &command)
            .await
            .map(|()| ApiResponse::ConsoleCommandAccepted)
            .unwrap_or_else(ApiResponse::Error),
        ApiRequest::ReadServerLogs {
            instance_id,
            after_offset,
            max_bytes,
        } => state
            .supervisor
            .read_logs(instance_id, after_offset, max_bytes)
            .await
            .map(ApiResponse::ServerLogs)
            .unwrap_or_else(ApiResponse::Error),
        ApiRequest::ListTasks { limit } => state
            .tasks
            .list(limit)
            .await
            .map(ApiResponse::Tasks)
            .unwrap_or_else(ApiResponse::Error),
        ApiRequest::GetTask { task_id } => state
            .tasks
            .get(task_id)
            .await
            .map(ApiResponse::Task)
            .unwrap_or_else(ApiResponse::Error),
        ApiRequest::ListTaskEvents {
            task_id,
            after_sequence,
        } => state
            .tasks
            .events(task_id, after_sequence, 200)
            .await
            .map(ApiResponse::TaskEvents)
            .unwrap_or_else(ApiResponse::Error),
        ApiRequest::CancelTask { task_id } => state
            .tasks
            .cancel(task_id)
            .await
            .map(ApiResponse::Task)
            .unwrap_or_else(ApiResponse::Error),
        ApiRequest::AnalyzeModpack { path } => {
            match tokio::task::spawn_blocking(move || mc_modpack::analyze(path)).await {
                Ok(Ok(report)) => ApiResponse::ModpackAnalysis(report),
                Ok(Err(error)) => ApiResponse::Error(error),
                Err(error) => ApiResponse::Error(
                    AppError::new(
                        ErrorCode::Internal,
                        "The modpack analysis worker stopped unexpectedly.",
                    )
                    .with_evidence(error.to_string()),
                ),
            }
        }
    };
    downgrade_response(response, protocol_version)
}

/// A v8 client cannot decode the task kinds, phases, and results introduced in
/// v9. Hide those tasks from v8 list responses and reject direct reads with an
/// actionable error instead of sending an envelope it cannot deserialize.
fn downgrade_response(response: ApiResponse, protocol_version: u16) -> ApiResponse {
    if protocol_version >= 9 {
        return response;
    }
    match response {
        ApiResponse::Tasks(tasks) => {
            ApiResponse::Tasks(tasks.into_iter().filter(task_is_v8_compatible).collect())
        }
        ApiResponse::Task(task) if !task_is_v8_compatible(&task) => newer_task_error(),
        ApiResponse::TaskEvents(events)
            if events
                .iter()
                .any(|event| event.phase == TaskPhase::ManagingInstance) =>
        {
            newer_task_error()
        }
        other => other,
    }
}

fn task_is_v8_compatible(task: &TaskSnapshot) -> bool {
    task.kind != TaskKind::ManageInstance
        && task.phase != TaskPhase::ManagingInstance
        && !matches!(task.result, Some(TaskResult::ManagementCompleted { .. }))
}

fn newer_task_error() -> ApiResponse {
    ApiResponse::Error(
        AppError::new(
            ErrorCode::ProtocolInvalid,
            "This task belongs to a newer daemon feature than the client supports.",
        )
        .with_action("Update the client to inspect this task."),
    )
}

async fn start_server(
    state: &DaemonState,
    instance_id: uuid::Uuid,
) -> Result<mc_core::ServerSnapshot, AppError> {
    let instance = state.instances.get_async(instance_id).await?;
    if state
        .backups
        .has_pending_restore(&instance)
        .await
        .map_err(AppError::from)?
    {
        let guard = state.supervisor.lock_stopped_instance(instance_id).await?;
        state
            .backups
            .recover_pending(guard)
            .await
            .map_err(AppError::from)?;
    }
    state.supervisor.start(instance_id).await
}

async fn create_paper_task(
    state: &DaemonState,
    command: PaperCreateCommand,
) -> Result<TaskSnapshot, AppError> {
    let spec = InstanceSpec {
        name: command.name,
        java_path: command.java_path,
        min_memory_mb: command.min_memory_mb,
        max_memory_mb: command.max_memory_mb,
        port: command.port,
        accept_eula: command.accept_eula,
    };
    spec.validate_for_creation_async().await?;

    let provider = Arc::clone(&state.provider);
    let downloader = state.downloader.clone();
    let instances = state.instances.clone();
    let game_version = command.game_version;
    let build_id = command.build_id;
    state
        .tasks
        .submit(
            TaskKind::CreatePaperInstance,
            None,
            move |reporter| async move {
                create_paper_instance(
                    provider,
                    downloader,
                    instances,
                    spec,
                    game_version,
                    build_id,
                    reporter,
                )
                .await
            },
        )
        .await
}

async fn create_paper_instance(
    provider: Arc<dyn mc_core::ServerProvider>,
    downloader: mc_download::DownloadManager,
    instances: mc_instance::InstanceService,
    spec: InstanceSpec,
    game_version: String,
    build_id: Option<u64>,
    reporter: TaskReporter,
) -> Result<TaskResult, AppError> {
    reporter
        .set_phase(
            TaskPhase::ResolvingArtifact,
            "Resolving an exact stable Paper build.",
            Some(TaskProgress {
                completed_steps: 0,
                total_steps: 3,
                transfer: None,
            }),
        )
        .await?;
    let plan = provider.resolve_build(&game_version, build_id).await?;
    if reporter.is_cancelled() {
        return Err(cancelled_task_error());
    }

    reporter
        .set_phase(
            TaskPhase::DownloadingArtifact,
            format!("Downloading and verifying {}.", plan.artifact_name),
            Some(TaskProgress {
                completed_steps: 1,
                total_steps: 3,
                transfer: None,
            }),
        )
        .await?;
    let blob = download_with_progress(&downloader, &plan, &reporter).await?;
    if reporter.is_cancelled() {
        return Err(cancelled_task_error());
    }

    reporter
        .set_phase(
            TaskPhase::InstallingInstance,
            "Staging and committing the verified instance.",
            Some(TaskProgress {
                completed_steps: 2,
                total_steps: 3,
                transfer: None,
            }),
        )
        .await?;
    let summary = instances.create(&plan, &blob, &spec).await?;
    Ok(TaskResult::InstanceCreated(summary))
}

pub(crate) async fn download_with_progress(
    downloader: &mc_download::DownloadManager,
    plan: &mc_core::ArtifactPlan,
    reporter: &TaskReporter,
) -> Result<mc_download::VerifiedBlob, AppError> {
    let control = mc_download::DownloadControl::new(reporter.cancellation_flag());
    let source = format!(
        "校验下载 · {}",
        plan.download_url.host_str().unwrap_or("host unavailable")
    );
    let fetch = downloader.fetch_with_control(plan, &control);
    tokio::pin!(fetch);
    let mut progress_timer = tokio::time::interval_at(
        tokio::time::Instant::now() + std::time::Duration::from_millis(500),
        std::time::Duration::from_millis(500),
    );
    progress_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut last_bytes = 0u64;
    let mut last_instant = tokio::time::Instant::now();
    loop {
        tokio::select! {
            result = &mut fetch => return result,
            _ = progress_timer.tick() => {
                if reporter.is_cancelled() {
                    control.request_cancel();
                }
                let downloaded = control.downloaded_bytes();
                let now = tokio::time::Instant::now();
                let elapsed = now.duration_since(last_instant).as_secs_f64();
                let speed = if elapsed > 0.0 && downloaded >= last_bytes {
                    Some(((downloaded - last_bytes) as f64 / elapsed) as u64)
                } else {
                    None
                };
                last_bytes = downloaded;
                last_instant = now;
                reporter
                    .set_phase(
                        TaskPhase::DownloadingArtifact,
                        format!(
                            "Downloading {} ({:.1} / {:.1} MiB).",
                            plan.artifact_name,
                            downloaded as f64 / 1_048_576.0,
                            plan.expected_size as f64 / 1_048_576.0,
                        ),
                        Some(TaskProgress {
                            completed_steps: 1,
                            total_steps: 3,
                            transfer: Some(mc_core::TransferProgress {
                                downloaded_bytes: downloaded,
                                total_bytes: plan.expected_size,
                                bytes_per_second: speed,
                                source: source.clone(),
                            }),
                        }),
                    )
                    .await?;
            }
        }
    }
}

fn cancelled_task_error() -> AppError {
    AppError::new(
        ErrorCode::TaskCancelled,
        "The task was cancelled by user request.",
    )
    .with_action("Submit the same operation again to resume from its verified partial file.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn snapshot(kind: TaskKind, phase: TaskPhase, result: Option<TaskResult>) -> TaskSnapshot {
        let now = Utc::now();
        TaskSnapshot {
            id: uuid::Uuid::new_v4(),
            kind,
            instance_id: None,
            status: mc_core::TaskStatus::Succeeded,
            phase,
            message: String::new(),
            progress: None,
            created_at: now,
            updated_at: now,
            finished_at: None,
            result,
            error: None,
            last_event_sequence: 0,
        }
    }

    #[test]
    fn v8_clients_do_not_see_management_tasks() {
        let legacy = snapshot(TaskKind::CreatePaperInstance, TaskPhase::Queued, None);
        let management = snapshot(TaskKind::ManageInstance, TaskPhase::Queued, None);
        let response = downgrade_response(ApiResponse::Tasks(vec![legacy.clone(), management]), 8);
        match response {
            ApiResponse::Tasks(tasks) => {
                assert_eq!(tasks.len(), 1);
                assert_eq!(tasks[0].id, legacy.id);
            }
            _ => panic!("expected a task list"),
        }
    }

    #[test]
    fn v9_clients_keep_management_tasks() {
        let management = snapshot(
            TaskKind::ManageInstance,
            TaskPhase::ManagingInstance,
            Some(TaskResult::ManagementCompleted {
                instance_id: uuid::Uuid::new_v4(),
                report: "ok".to_owned(),
            }),
        );
        let response = downgrade_response(ApiResponse::Tasks(vec![management]), 9);
        match response {
            ApiResponse::Tasks(tasks) => assert_eq!(tasks.len(), 1),
            _ => panic!("expected a task list"),
        }
    }

    #[test]
    fn v8_task_detail_rejects_a_management_task() {
        let management = snapshot(
            TaskKind::ManageInstance,
            TaskPhase::ManagingInstance,
            Some(TaskResult::ManagementCompleted {
                instance_id: uuid::Uuid::new_v4(),
                report: "ok".to_owned(),
            }),
        );
        match downgrade_response(ApiResponse::Task(management), 8) {
            ApiResponse::Error(error) => assert_eq!(error.code, ErrorCode::ProtocolInvalid),
            _ => panic!("expected a protocol error"),
        }
    }
}
