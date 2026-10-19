use std::sync::Arc;

use async_trait::async_trait;
use mc_client::DaemonClient;
use mc_core::{ApiRequest, ApiResponse, AppError, BackupCursor, ErrorCode, ServerSnapshot};

use crate::{
    app::DashboardData,
    instance_detail::{InstanceDetailQuery, InstanceDetailUpdate},
    task_detail::TaskDetailUpdate,
    wizard::{CreateInstanceInput, CreateOptions},
};

const BACKUP_PREVIEW_PAGE_LIMIT: u16 = 3;

#[async_trait]
pub(crate) trait ManagementSource: Send + Sync {
    async fn manage_instance(
        &self,
        _instance_id: uuid::Uuid,
        _action: mc_core::InstanceOperation,
    ) -> Result<ApiResponse, AppError> {
        Err(AppError::new(
            ErrorCode::InvalidInput,
            "此数据源不支持实例管理。",
        ))
    }
    async fn load(&self) -> Result<DashboardData, AppError>;
    async fn load_tasks(&self) -> Result<Vec<mc_core::TaskSnapshot>, AppError> {
        Ok(self.load().await?.tasks)
    }
    async fn load_create_options(&self) -> Result<CreateOptions, AppError>;
    async fn create_instance(
        &self,
        input: CreateInstanceInput,
    ) -> Result<mc_core::TaskSnapshot, AppError>;
    async fn load_task_detail(
        &self,
        task_id: uuid::Uuid,
        after_sequence: i64,
    ) -> Result<TaskDetailUpdate, AppError>;
    async fn load_instance_detail(
        &self,
        query: InstanceDetailQuery,
    ) -> Result<InstanceDetailUpdate, AppError>;
    async fn start_server(&self, instance_id: uuid::Uuid) -> Result<ServerSnapshot, AppError>;
    async fn stop_server(&self, instance_id: uuid::Uuid) -> Result<ServerSnapshot, AppError>;
    async fn create_backup(
        &self,
        instance_id: uuid::Uuid,
    ) -> Result<mc_core::TaskSnapshot, AppError>;
    async fn list_backups(
        &self,
        instance_id: uuid::Uuid,
        cursor: Option<BackupCursor>,
    ) -> Result<mc_core::TaskSnapshot, AppError>;
    async fn verify_backup(
        &self,
        instance_id: uuid::Uuid,
        backup_id: uuid::Uuid,
    ) -> Result<mc_core::TaskSnapshot, AppError>;
    async fn plan_restore(
        &self,
        instance_id: uuid::Uuid,
        backup_id: uuid::Uuid,
    ) -> Result<mc_core::TaskSnapshot, AppError>;
    async fn restore_backup(
        &self,
        instance_id: uuid::Uuid,
        backup_id: uuid::Uuid,
        confirmation_token: String,
    ) -> Result<mc_core::TaskSnapshot, AppError>;
    async fn load_task(&self, task_id: uuid::Uuid) -> Result<mc_core::TaskSnapshot, AppError>;
    async fn cancel_task(&self, task_id: uuid::Uuid) -> Result<mc_core::TaskSnapshot, AppError>;
    async fn analyze_modpack(
        &self,
        path: std::path::PathBuf,
    ) -> Result<mc_core::ModpackAnalysisReport, AppError>;
    async fn send_console_command(
        &self,
        instance_id: uuid::Uuid,
        command: String,
    ) -> Result<(), AppError>;
}

pub(crate) struct DaemonDashboardSource {
    client: Arc<DaemonClient>,
}

impl DaemonDashboardSource {
    pub fn new(client: DaemonClient) -> Self {
        Self {
            client: Arc::new(client),
        }
    }
}

#[async_trait]
impl ManagementSource for DaemonDashboardSource {
    async fn load_tasks(&self) -> Result<Vec<mc_core::TaskSnapshot>, AppError> {
        match self
            .client
            .request(ApiRequest::ListTasks { limit: 32 })
            .await?
        {
            ApiResponse::Tasks(tasks) => Ok(tasks),
            ApiResponse::Error(error) => Err(error),
            _ => Err(unexpected_response("tasks")),
        }
    }
    async fn manage_instance(
        &self,
        instance_id: uuid::Uuid,
        action: mc_core::InstanceOperation,
    ) -> Result<ApiResponse, AppError> {
        match self
            .client
            .request(ApiRequest::ManageInstance {
                instance_id,
                action,
            })
            .await?
        {
            ApiResponse::Error(error) => Err(error),
            response => Ok(response),
        }
    }
    async fn load(&self) -> Result<DashboardData, AppError> {
        let (instances, tasks) = tokio::join!(
            self.client.request(ApiRequest::ListInstances),
            self.client.request(ApiRequest::ListTasks { limit: 32 }),
        );
        let instances = expect_instances(instances?)?;
        let details = futures_util::future::join_all(instances.iter().map(|instance| {
            self.client.request(ApiRequest::GetInstanceDetail {
                instance_id: instance.id,
            })
        }))
        .await;
        let servers = details
            .into_iter()
            .filter_map(|result| match result {
                Ok(ApiResponse::InstanceDetail(detail)) => Some(detail.server),
                _ => None,
            })
            .collect();
        Ok(DashboardData {
            servers,
            instances,
            tasks: expect_tasks(tasks?)?,
        })
    }

    async fn load_create_options(&self) -> Result<CreateOptions, AppError> {
        let doctor = expect_doctor(self.client.request(ApiRequest::Doctor).await?)?;
        let versions = expect_versions(self.client.request(ApiRequest::PaperVersions).await?)?;
        let reserved = expect_instances(self.client.request(ApiRequest::ListInstances).await?)?
            .into_iter()
            .map(|v| v.port)
            .collect::<Vec<_>>();
        let suggested_port = tokio::task::spawn_blocking(move || find_available_port(&reserved))
            .await
            .map_err(|error| {
                AppError::new(
                    ErrorCode::Internal,
                    "Could not inspect local port availability.",
                )
                .with_evidence(error.to_string())
            })?;
        Ok(CreateOptions {
            versions,
            java_runtimes: doctor.java_runtimes,
            suggested_port,
        })
    }

    async fn create_instance(
        &self,
        input: CreateInstanceInput,
    ) -> Result<mc_core::TaskSnapshot, AppError> {
        match self
            .client
            .request(ApiRequest::CreatePaperInstance {
                name: input.name,
                game_version: input.game_version,
                build_id: None,
                java_path: input.java_path,
                min_memory_mb: input.min_memory_mb,
                max_memory_mb: input.max_memory_mb,
                port: input.port,
                accept_eula: input.accept_eula,
            })
            .await?
        {
            ApiResponse::TaskQueued(task) => Ok(task),
            ApiResponse::Error(error) => Err(error),
            _ => Err(unexpected_response("instance creation")),
        }
    }

    async fn load_task_detail(
        &self,
        task_id: uuid::Uuid,
        after_sequence: i64,
    ) -> Result<TaskDetailUpdate, AppError> {
        let (task, events) = tokio::join!(
            self.client.request(ApiRequest::GetTask { task_id }),
            self.client.request(ApiRequest::ListTaskEvents {
                task_id,
                after_sequence,
            }),
        );
        let task = expect_task(task?)?;
        let events = expect_task_events(events?)?;
        if task.id != task_id || events.iter().any(|event| event.task_id != task_id) {
            return Err(unexpected_response("task detail"));
        }
        Ok(TaskDetailUpdate { task, events })
    }

    async fn load_instance_detail(
        &self,
        query: InstanceDetailQuery,
    ) -> Result<InstanceDetailUpdate, AppError> {
        let (detail, logs) = tokio::join!(
            self.client.request(ApiRequest::GetInstanceDetail {
                instance_id: query.instance_id,
            }),
            self.client.request(ApiRequest::ReadServerLogs {
                instance_id: query.instance_id,
                after_offset: query.after_offset,
                max_bytes: 16 * 1024,
            }),
        );
        let detail = expect_instance_detail(detail?)?;
        let logs = expect_server_logs(logs?)?;
        if detail.instance.id != query.instance_id
            || detail.server.instance_id != query.instance_id
            || logs.instance_id != query.instance_id
        {
            return Err(unexpected_response("instance detail"));
        }
        let stats = process_stats(detail.server.pid).await;
        Ok(InstanceDetailUpdate {
            detail,
            logs,
            stats,
        })
    }

    async fn start_server(&self, instance_id: uuid::Uuid) -> Result<ServerSnapshot, AppError> {
        expect_server_status(
            self.client
                .request(ApiRequest::StartServer { instance_id })
                .await?,
            "start server",
        )
    }

    async fn stop_server(&self, instance_id: uuid::Uuid) -> Result<ServerSnapshot, AppError> {
        expect_server_status(
            self.client
                .request(ApiRequest::StopServer {
                    instance_id,
                    timeout_seconds: 20,
                })
                .await?,
            "stop server",
        )
    }

    async fn create_backup(
        &self,
        instance_id: uuid::Uuid,
    ) -> Result<mc_core::TaskSnapshot, AppError> {
        match self
            .client
            .request(ApiRequest::CreateWorldBackup { instance_id })
            .await?
        {
            ApiResponse::TaskQueued(task) => Ok(task),
            ApiResponse::Error(error) => Err(error),
            _ => Err(unexpected_response("world backup")),
        }
    }

    async fn list_backups(
        &self,
        instance_id: uuid::Uuid,
        cursor: Option<BackupCursor>,
    ) -> Result<mc_core::TaskSnapshot, AppError> {
        match self
            .client
            .request(ApiRequest::ListWorldBackups {
                instance_id,
                limit: BACKUP_PREVIEW_PAGE_LIMIT,
                cursor,
            })
            .await?
        {
            ApiResponse::TaskQueued(task) => Ok(task),
            ApiResponse::Error(error) => Err(error),
            _ => Err(unexpected_response("world backup list")),
        }
    }

    async fn verify_backup(
        &self,
        instance_id: uuid::Uuid,
        backup_id: uuid::Uuid,
    ) -> Result<mc_core::TaskSnapshot, AppError> {
        match self
            .client
            .request(ApiRequest::VerifyWorldBackup {
                instance_id,
                backup_id,
            })
            .await?
        {
            ApiResponse::TaskQueued(task) => Ok(task),
            ApiResponse::Error(error) => Err(error),
            _ => Err(unexpected_response("world backup verification")),
        }
    }

    async fn plan_restore(
        &self,
        instance_id: uuid::Uuid,
        backup_id: uuid::Uuid,
    ) -> Result<mc_core::TaskSnapshot, AppError> {
        match self
            .client
            .request(ApiRequest::PlanWorldBackupRestore {
                instance_id,
                backup_id,
            })
            .await?
        {
            ApiResponse::TaskQueued(task) => Ok(task),
            ApiResponse::Error(error) => Err(error),
            _ => Err(unexpected_response("world backup restore plan")),
        }
    }

    async fn restore_backup(
        &self,
        instance_id: uuid::Uuid,
        backup_id: uuid::Uuid,
        confirmation_token: String,
    ) -> Result<mc_core::TaskSnapshot, AppError> {
        match self
            .client
            .request(ApiRequest::RestoreWorldBackup {
                instance_id,
                backup_id,
                confirmation_token,
            })
            .await?
        {
            ApiResponse::TaskQueued(task) => Ok(task),
            ApiResponse::Error(error) => Err(error),
            _ => Err(unexpected_response("world backup restore")),
        }
    }

    async fn load_task(&self, task_id: uuid::Uuid) -> Result<mc_core::TaskSnapshot, AppError> {
        expect_task(self.client.request(ApiRequest::GetTask { task_id }).await?)
    }

    async fn cancel_task(&self, task_id: uuid::Uuid) -> Result<mc_core::TaskSnapshot, AppError> {
        expect_task(
            self.client
                .request(ApiRequest::CancelTask { task_id })
                .await?,
        )
    }

    async fn analyze_modpack(
        &self,
        path: std::path::PathBuf,
    ) -> Result<mc_core::ModpackAnalysisReport, AppError> {
        match self
            .client
            .request(ApiRequest::AnalyzeModpack { path })
            .await?
        {
            ApiResponse::ModpackAnalysis(report) => Ok(report),
            ApiResponse::Error(error) => Err(error),
            _ => Err(unexpected_response("modpack analysis")),
        }
    }

    async fn send_console_command(
        &self,
        instance_id: uuid::Uuid,
        command: String,
    ) -> Result<(), AppError> {
        match self
            .client
            .request(ApiRequest::SendConsoleCommand {
                instance_id,
                command,
            })
            .await?
        {
            ApiResponse::ConsoleCommandAccepted => Ok(()),
            ApiResponse::Error(error) => Err(error),
            _ => Err(unexpected_response("server console")),
        }
    }
}

fn expect_doctor(response: ApiResponse) -> Result<mc_core::DoctorReport, AppError> {
    match response {
        ApiResponse::Doctor(report) => Ok(report),
        ApiResponse::Error(error) => Err(error),
        _ => Err(unexpected_response("doctor report")),
    }
}

fn expect_versions(response: ApiResponse) -> Result<Vec<mc_core::ServerVersion>, AppError> {
    match response {
        ApiResponse::PaperVersions(versions) => Ok(versions),
        ApiResponse::Error(error) => Err(error),
        _ => Err(unexpected_response("Paper version list")),
    }
}

fn expect_instances(response: ApiResponse) -> Result<Vec<mc_core::InstanceSummary>, AppError> {
    match response {
        ApiResponse::Instances(instances) => Ok(instances),
        ApiResponse::Error(error) => Err(error),
        _ => Err(unexpected_response("instance list")),
    }
}

fn expect_tasks(response: ApiResponse) -> Result<Vec<mc_core::TaskSnapshot>, AppError> {
    match response {
        ApiResponse::Tasks(tasks) => Ok(tasks),
        ApiResponse::Error(error) => Err(error),
        _ => Err(unexpected_response("task list")),
    }
}

fn expect_task(response: ApiResponse) -> Result<mc_core::TaskSnapshot, AppError> {
    match response {
        ApiResponse::Task(task) => Ok(task),
        ApiResponse::Error(error) => Err(error),
        _ => Err(unexpected_response("task detail")),
    }
}

fn expect_task_events(response: ApiResponse) -> Result<Vec<mc_core::TaskEvent>, AppError> {
    match response {
        ApiResponse::TaskEvents(events) => Ok(events),
        ApiResponse::Error(error) => Err(error),
        _ => Err(unexpected_response("task event list")),
    }
}

async fn process_stats(pid: Option<u32>) -> Option<crate::instance_detail::ProcessStats> {
    let pid = pid?;
    let output = tokio::process::Command::new("ps")
        .args(["-o", "%cpu=,rss=", "-p", &pid.to_string()])
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut parts = text.split_whitespace();
    let cpu_percent = parts.next()?.parse::<f64>().ok()?;
    let rss_kib = parts.next()?.parse::<u64>().ok()?;
    Some(crate::instance_detail::ProcessStats {
        cpu_percent,
        rss_bytes: rss_kib.saturating_mul(1024),
    })
}

fn expect_instance_detail(response: ApiResponse) -> Result<mc_core::InstanceDetail, AppError> {
    match response {
        ApiResponse::InstanceDetail(detail) => Ok(detail),
        ApiResponse::Error(error) => Err(error),
        _ => Err(unexpected_response("instance detail")),
    }
}

fn expect_server_status(response: ApiResponse, action: &str) -> Result<ServerSnapshot, AppError> {
    match response {
        ApiResponse::ServerStatus(status) => Ok(status),
        ApiResponse::Error(error) => Err(error),
        _ => Err(unexpected_response(action)),
    }
}

fn expect_server_logs(response: ApiResponse) -> Result<mc_core::ServerLogChunk, AppError> {
    match response {
        ApiResponse::ServerLogs(logs) => Ok(logs),
        ApiResponse::Error(error) => Err(error),
        _ => Err(unexpected_response("server logs")),
    }
}

fn unexpected_response(request: &str) -> AppError {
    AppError::new(
        ErrorCode::ProtocolInvalid,
        format!("The daemon returned an unexpected response for the {request}."),
    )
}

fn find_available_port(reserved: &[u16]) -> u16 {
    (25565..=25585)
        .find(|port| {
            !reserved.contains(port) && std::net::TcpListener::bind(("127.0.0.1", *port)).is_ok()
        })
        .unwrap_or(25565)
}
