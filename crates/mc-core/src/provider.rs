use async_trait::async_trait;

use crate::{AppError, ArtifactPlan, BuildInfo, ServerVersion};

/// Provider boundary: adapters resolve authoritative metadata into common
/// domain objects; callers never depend on a provider's JSON response shape.
#[async_trait]
pub trait ServerProvider: Send + Sync {
    fn id(&self) -> &'static str;

    async fn list_versions(&self) -> Result<Vec<ServerVersion>, AppError>;

    async fn list_builds(&self, game_version: &str) -> Result<Vec<BuildInfo>, AppError>;

    async fn resolve_build(
        &self,
        game_version: &str,
        build_id: Option<u64>,
    ) -> Result<ArtifactPlan, AppError>;
}
