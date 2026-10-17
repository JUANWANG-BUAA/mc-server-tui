use std::sync::Arc;

use async_trait::async_trait;
use mc_client::DaemonClient;
use mc_core::{ApiRequest, ApiResponse, AppError, ErrorCode};

use crate::app::DashboardData;

#[async_trait]
pub(crate) trait DashboardSource: Send + Sync {
    async fn load(&self) -> Result<DashboardData, AppError>;
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
impl DashboardSource for DaemonDashboardSource {
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

fn unexpected_response(request: &str) -> AppError {
    AppError::new(
        ErrorCode::ProtocolInvalid,
        format!("The daemon returned an unexpected response for the {request}."),
    )
}
