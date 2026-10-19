//! Paper Fill v3 adapter.

mod client;
mod errors;
mod parse;

#[cfg(test)]
mod tests;

use chrono::Utc;
pub use client::PaperProvider;
use mc_core::{AppError, ArtifactPlan, BuildInfo, ErrorCode, ServerProvider, ServerVersion};
use serde_json::Value;

use parse::{schema_error, select_stable_build};

const API_ROOT: &str = "https://fill.papermc.io/v3/";
const API_HOST: &str = "fill.papermc.io";
const DATA_HOST: &str = "fill-data.papermc.io";
const MAX_METADATA_BYTES: usize = 2 * 1024 * 1024;

#[async_trait::async_trait]
impl ServerProvider for PaperProvider {
    fn id(&self) -> &'static str {
        "paper"
    }

    async fn list_versions(&self) -> Result<Vec<ServerVersion>, AppError> {
        let value = self.get_json(self.endpoint("projects/paper")).await?;
        let groups = value
            .get("versions")
            .and_then(Value::as_object)
            .ok_or_else(|| schema_error("Paper project response is missing the versions map."))?;
        let mut versions = Vec::new();
        for (group, entries) in groups {
            let entries = entries
                .as_array()
                .ok_or_else(|| schema_error("A Paper version group was not an array."))?;
            for entry in entries {
                let id = entry
                    .as_str()
                    .ok_or_else(|| schema_error("Paper version identifier was not a string."))?;
                versions.push(ServerVersion {
                    id: id.to_owned(),
                    group: group.clone(),
                });
            }
        }
        if versions.is_empty() {
            return Err(schema_error("Paper returned no versions."));
        }
        versions.sort_by_key(|v| std::cmp::Reverse(mc_core::minecraft_version_key(&v.id)));
        versions.dedup_by(|a, b| a.id == b.id);
        Ok(versions)
    }

    async fn list_builds(&self, game_version: &str) -> Result<Vec<BuildInfo>, AppError> {
        Ok(self
            .parse_builds(game_version)
            .await?
            .into_iter()
            .map(|build| build.info)
            .collect())
    }

    async fn resolve_build(
        &self,
        game_version: &str,
        build_id: Option<u64>,
    ) -> Result<ArtifactPlan, AppError> {
        let endpoint = self.endpoint(&format!("projects/paper/versions/{game_version}/builds"));
        let builds = self.parse_builds(game_version).await?;
        let selected = select_stable_build(builds, game_version, build_id)?;
        let artifact = selected.artifact.ok_or_else(|| {
            AppError::new(
                ErrorCode::ServerArtifactMissing,
                "The selected Paper build has no server:default artifact.",
            )
        })?;
        ArtifactPlan::new(
            self.id(),
            "paper",
            game_version,
            selected.info.id,
            selected.info.channel,
            artifact.name,
            endpoint,
            artifact.url,
            artifact.size,
            artifact.sha256,
            Utc::now(),
        )
    }
}
