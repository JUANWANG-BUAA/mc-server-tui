use std::sync::Arc;

use async_trait::async_trait;
use mc_client::DaemonClient;
use mc_core::{ApiRequest, ApiResponse, AppError, ErrorCode, ServerSnapshot};

use crate::{
    app::DashboardData,
    instance_detail::{InstanceDetailQuery, InstanceDetailUpdate},
    task_detail::TaskDetailUpdate,
    wizard::CreateInstanceInput,
};

#[async_trait]
pub(crate) trait ManagementSource: Send + Sync {
    async fn load(&self) -> Result<DashboardData, AppError>;
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
    async fn load(&self) -> Result<DashboardData, AppError> {
        let (instances, tasks) = tokio::join!(
            self.client.request(ApiRequest::ListInstances),
            self.client.request(ApiRequest::ListTasks { limit: 8 }),
        );
        Ok(DashboardData {
            instances: expect_instances(instances?)?,
            tasks: expect_tasks(tasks?)?,
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
        Ok(InstanceDetailUpdate { detail, logs })
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
