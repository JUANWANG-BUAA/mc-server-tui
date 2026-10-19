use std::{path::PathBuf, sync::Arc};

use mc_backup::BackupService;
use mc_core::ServerProvider;
use mc_download::DownloadManager;
use mc_instance::InstanceService;
use mc_supervisor::ProcessSupervisor;
use mc_task::TaskManager;

pub(crate) struct DaemonState {
    pub provider: Arc<dyn ServerProvider>,
    pub downloader: DownloadManager,
    pub instances: InstanceService,
    pub supervisor: ProcessSupervisor,
    pub backups: BackupService,
    pub tasks: TaskManager,
    /// Configured identity sent to every upstream (Paper, Fabric, Modrinth).
    pub user_agent: String,
    /// Content-addressed artifact cache shared by all providers.
    pub cache_dir: PathBuf,
}
