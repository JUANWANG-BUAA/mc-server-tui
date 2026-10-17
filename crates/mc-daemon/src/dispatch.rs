use std::{path::PathBuf, sync::Arc};

use mc_core::{
    ApiRequest, ApiResponse, AppError, TaskKind, TaskPhase, TaskProgress, TaskResult, TaskSnapshot,
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

pub(crate) async fn dispatch(request: ApiRequest, state: &DaemonState) -> ApiResponse {
    match request {
        ApiRequest::Ping => ApiResponse::Pong,
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
    }
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
            }),
        )
        .await?;
    let plan = provider.resolve_build(&game_version, build_id).await?;

    reporter
        .set_phase(
            TaskPhase::DownloadingArtifact,
            format!("Downloading and verifying {}.", plan.artifact_name),
            Some(TaskProgress {
                completed_steps: 1,
                total_steps: 3,
            }),
        )
        .await?;
    let blob = downloader.fetch(&plan).await?;

    reporter
        .set_phase(
            TaskPhase::InstallingInstance,
            "Staging and committing the verified instance.",
            Some(TaskProgress {
                completed_steps: 2,
                total_steps: 3,
            }),
        )
        .await?;
    let summary = instances.create(&plan, &blob, &spec).await?;
    Ok(TaskResult::InstanceCreated(summary))
}
