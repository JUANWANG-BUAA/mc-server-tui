use crate::catalog::{InstanceCatalog, SqliteCatalog};
use crate::filesystem::{
    create_staging_directory, ensure_managed_directory, io_error, sync_directory,
};
use crate::model::{
    validate_plan, InstanceConfig, InstanceLock, InstanceSpec, INSTANCE_SCHEMA_VERSION,
};
use crate::recovery::recover_instances;
use crate::stage::write_staged_instance;
use chrono::Utc;
use mc_core::{AppError, ArtifactPlan, ErrorCode, InstanceSummary};
use mc_download::VerifiedBlob;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

/// Coordinates per-instance files and a replaceable, rebuildable index.
#[derive(Clone)]
pub struct InstanceService {
    instances_root: PathBuf,
    catalog: Arc<dyn InstanceCatalog>,
}

impl InstanceService {
    pub async fn open(state_dir: impl AsRef<Path>) -> Result<Self, AppError> {
        tokio::fs::create_dir_all(state_dir.as_ref())
            .await
            .map_err(|error| io_error("Could not create application state directory.", error))?;
        let state_dir = tokio::fs::canonicalize(state_dir.as_ref())
            .await
            .map_err(|error| io_error("Could not resolve application state directory.", error))?;
        let instances_root = state_dir.join("instances");
        ensure_managed_directory(&instances_root).await?;
        let catalog = Arc::new(SqliteCatalog::open(state_dir.join("index.sqlite3"))?);
        let service = Self {
            instances_root,
            catalog,
        };
        service.reconcile().await?;
        Ok(service)
    }

    /// Alternate catalog injection keeps file operations independent from SQLite.
    pub async fn with_catalog(
        state_dir: impl AsRef<Path>,
        catalog: Arc<dyn InstanceCatalog>,
    ) -> Result<Self, AppError> {
        tokio::fs::create_dir_all(state_dir.as_ref())
            .await
            .map_err(|error| io_error("Could not create application state directory.", error))?;
        let state_dir = tokio::fs::canonicalize(state_dir.as_ref())
            .await
            .map_err(|error| io_error("Could not resolve application state directory.", error))?;
        let instances_root = state_dir.join("instances");
        ensure_managed_directory(&instances_root).await?;
        let service = Self {
            instances_root,
            catalog,
        };
        service.reconcile().await?;
        Ok(service)
    }

    pub fn list(&self) -> Result<Vec<InstanceSummary>, AppError> {
        self.catalog.list()
    }

    pub async fn list_async(&self) -> Result<Vec<InstanceSummary>, AppError> {
        let catalog = Arc::clone(&self.catalog);
        tokio::task::spawn_blocking(move || catalog.list())
            .await
            .map_err(|error| {
                AppError::new(ErrorCode::Internal, "Instance index worker failed.")
                    .with_evidence(error.to_string())
            })?
    }

    /// Resolve one committed instance from the rebuildable catalog.
    pub async fn get_async(&self, id: Uuid) -> Result<InstanceSummary, AppError> {
        self.list_async()
            .await?
            .into_iter()
            .find(|instance| instance.id == id)
            .ok_or_else(|| {
                AppError::new(ErrorCode::InstanceNotFound, "The instance does not exist.")
                    .with_evidence(format!("instance_id={id}"))
            })
    }

    pub async fn create(
        &self,
        plan: &ArtifactPlan,
        blob: &VerifiedBlob,
        spec: &InstanceSpec,
    ) -> Result<InstanceSummary, AppError> {
        validate_plan(plan)?;
        let (name, java_path) = spec.validate_details_async().await?;
        if blob.size() != plan.expected_size
            || !blob.sha256().eq_ignore_ascii_case(&plan.checksum.value)
        {
            return Err(AppError::new(
                ErrorCode::HashMismatch,
                "Verified download does not match this exact artifact plan.",
            ));
        }

        let id = Uuid::new_v4();
        let created_at = Utc::now();
        let config = InstanceConfig {
            schema_version: INSTANCE_SCHEMA_VERSION,
            id,
            name,
            provider: plan.provider.clone(),
            project: plan.project.clone(),
            game_version: plan.game_version.clone(),
            build_id: plan.build_id,
            java_path,
            min_memory_mb: spec.min_memory_mb,
            max_memory_mb: spec.max_memory_mb,
            port: spec.port,
            eula_accepted: spec.accept_eula,
            created_at,
            updated_at: created_at,
        };
        let lock = InstanceLock::from_plan(plan, blob.sha256().to_owned());
        let staging_path = self.instances_root.join(format!(".staging-{id}"));
        let final_path = self.instances_root.join(id.to_string());

        create_staging_directory(&staging_path).await?;
        if tokio::fs::try_exists(&final_path)
            .await
            .map_err(|error| io_error("Could not check the final instance path.", error))?
        {
            let _ = tokio::fs::remove_dir(&staging_path).await;
            return Err(AppError::new(
                ErrorCode::Internal,
                "A directory already exists for the new instance ID.",
            ));
        }
        let result = write_staged_instance(&staging_path, plan, blob, &config, &lock).await;
        if let Err(error) = result {
            let _ = tokio::fs::remove_dir_all(&staging_path).await;
            return Err(error);
        }
        if let Err(error) = tokio::fs::rename(&staging_path, &final_path).await {
            let _ = tokio::fs::remove_dir_all(&staging_path).await;
            return Err(io_error(
                "Could not atomically commit the new instance.",
                error,
            ));
        }
        sync_directory(&self.instances_root).await?;

        if let Err(error) = self.reconcile().await {
            return Err(AppError::new(
                ErrorCode::Internal,
                "Instance files were committed, but the index update failed.",
            )
            .with_action("Restart the daemon to rebuild the instance index from committed files.")
            .with_evidence(format!("instance_id={id}; cause={}", error.reason)));
        }
        Ok(config.summary(final_path))
    }

    async fn reconcile(&self) -> Result<(), AppError> {
        let entries = recover_instances(&self.instances_root).await?;
        let catalog = Arc::clone(&self.catalog);
        tokio::task::spawn_blocking(move || catalog.replace_snapshot(&entries))
            .await
            .map_err(|error| {
                AppError::new(ErrorCode::Internal, "Instance index worker failed.")
                    .with_evidence(error.to_string())
            })?
    }
}
