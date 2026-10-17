mod journal;
mod plan;
mod recovery;
mod transaction;

use mc_core::{BackupRestorePlan, InstanceSummary, StoppedInstanceLease};
use uuid::Uuid;

use crate::{error::BackupError, manifest::BackupManifest, store::BackupService};

impl BackupService {
    /// Verifies a backup and returns the stopped world's complete replacement
    /// impact. The returned token must be supplied unchanged to restore.
    pub async fn plan_restore<G>(
        &self,
        guard: G,
        backup_id: Uuid,
    ) -> Result<BackupRestorePlan, BackupError>
    where
        G: StoppedInstanceLease,
    {
        tokio::task::spawn_blocking(move || {
            let instance = guard.instance().clone();
            plan_restore_blocking(&instance, backup_id)
        })
        .await
        .map_err(|error| BackupError::WorkerFailed(error.to_string()))?
    }

    /// Restores a verified snapshot only when the caller confirms the exact
    /// preview produced by plan_restore. Filesystem work retains the stopped
    /// instance lease inside the blocking worker, including after cancellation.
    pub async fn restore<G>(
        &self,
        guard: G,
        backup_id: Uuid,
        confirmation_token: String,
    ) -> Result<BackupManifest, BackupError>
    where
        G: StoppedInstanceLease,
    {
        tokio::task::spawn_blocking(move || {
            let instance = guard.instance().clone();
            restore_blocking(&instance, backup_id, &confirmation_token)
        })
        .await
        .map_err(|error| BackupError::WorkerFailed(error.to_string()))?
    }

    /// Checks for durable restore work before the daemon permits a server
    /// start. This scan is bounded to UUID-named restore operation directories.
    pub async fn has_pending_restore(
        &self,
        instance: &InstanceSummary,
    ) -> Result<bool, BackupError> {
        let instance = instance.clone();
        tokio::task::spawn_blocking(move || recovery::has_pending_restore(&instance))
            .await
            .map_err(|error| BackupError::WorkerFailed(error.to_string()))?
    }

    /// Reconciles any interrupted local restore transaction while retaining
    /// the caller's stopped-instance reservation.
    pub async fn recover_pending<G>(&self, guard: G) -> Result<(), BackupError>
    where
        G: StoppedInstanceLease,
    {
        tokio::task::spawn_blocking(move || {
            let instance = guard.instance().clone();
            recovery::recover_pending(&instance)
        })
        .await
        .map_err(|error| BackupError::WorkerFailed(error.to_string()))?
    }
}

fn plan_restore_blocking(
    instance: &InstanceSummary,
    backup_id: Uuid,
) -> Result<BackupRestorePlan, BackupError> {
    recovery::recover_pending(instance)?;
    plan::build_restore_plan(instance, backup_id)
}

fn restore_blocking(
    instance: &InstanceSummary,
    backup_id: Uuid,
    confirmation_token: &str,
) -> Result<BackupManifest, BackupError> {
    restore_blocking_with_hook(instance, backup_id, confirmation_token, &mut |_| Ok(()))
}

fn restore_blocking_with_hook<F>(
    instance: &InstanceSummary,
    backup_id: Uuid,
    confirmation_token: &str,
    after_step: &mut F,
) -> Result<BackupManifest, BackupError>
where
    F: FnMut(transaction::RestoreStep) -> Result<(), BackupError>,
{
    recovery::recover_pending(instance)?;
    let plan = plan::build_restore_plan(instance, backup_id)?;
    if plan.confirmation_token != confirmation_token {
        return Err(BackupError::RestorePlanChanged);
    }
    let manifest = BackupService::read_manifest_blocking(instance, backup_id)?;
    transaction::execute_restore(instance, &manifest, confirmation_token, after_step)?;
    Ok(manifest)
}

#[cfg(test)]
mod tests;
