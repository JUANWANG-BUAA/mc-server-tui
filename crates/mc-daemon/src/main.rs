mod backup;
mod backup_restore;
mod config;
mod dispatch;
mod management;
mod state;
mod transport;

use std::{io, os::unix::fs::PermissionsExt, sync::Arc};

use clap::Parser;
use config::Args;
use mc_backup::BackupService;
use mc_core::ServerProvider;
use mc_download::{DownloadConfig, DownloadManager};
use mc_instance::InstanceService;
use mc_supervisor::ProcessSupervisor;
use mc_task::TaskManager;
use provider_paper::PaperProvider;
use state::DaemonState;
use tokio::{fs, net::UnixListener, signal, sync::Semaphore};
use tracing::{error, info, warn};

const MAX_CONNECTIONS: usize = 64;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let config = mc_config::Config::load()?;
    let filter = std::env::var("RUST_LOG")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| config.daemon.log_level.clone());
    let filter = tracing_subscriber::EnvFilter::try_new(filter)
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    let state_dir = args
        .state_dir
        .clone()
        .unwrap_or(config.resolved_state_dir()?);
    if let Some(instance_id) = args.internal_runner {
        let instances = InstanceService::open(&state_dir).await?;
        mc_supervisor::run_instance_runner(instances, instance_id).await?;
        return Ok(());
    }

    config.ensure_directories()?;
    let user_agent = config.resolved_user_agent();
    let provider: Arc<dyn ServerProvider> = Arc::new(PaperProvider::new(Some(user_agent.clone()))?);
    let cache_dir = args
        .cache_dir
        .clone()
        .unwrap_or(config.resolved_cache_dir()?);
    let private_parent = args.socket.is_none();
    let socket_path = match args.socket.clone() {
        Some(path) => path,
        None => match config.daemon.socket.as_ref() {
            Some(path) if !path.as_os_str().is_empty() => config.resolved_socket()?,
            _ => state_dir.join("run").join("daemon.sock"),
        },
    };
    let downloader =
        DownloadManager::new(DownloadConfig::paper(cache_dir.clone(), user_agent.clone()))?;
    let instances = InstanceService::open(&state_dir).await?;
    let supervisor = ProcessSupervisor::new(instances.clone());
    let tasks = TaskManager::open(state_dir.join("index.sqlite3")).await?;
    let state = Arc::new(DaemonState {
        provider,
        downloader,
        instances,
        supervisor,
        backups: BackupService,
        tasks,
        user_agent,
        cache_dir,
    });

    transport::prepare_parent(&socket_path, private_parent).await?;
    transport::remove_stale_socket(&socket_path).await?;
    let listener = UnixListener::bind(&socket_path)?;
    fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o600)).await?;
    let connection_slots = Arc::new(Semaphore::new(MAX_CONNECTIONS));

    info!(socket = %socket_path.display(), "management daemon ready");
    loop {
        tokio::select! {
            accept = listener.accept() => match accept {
                Ok((stream, _)) => {
                    let Ok(connection_permit) = Arc::clone(&connection_slots).try_acquire_owned() else {
                        warn!("management connection limit reached; dropping request");
                        drop(stream);
                        continue;
                    };
                    let state = Arc::clone(&state);
                    tokio::spawn(async move {
                        let _connection_permit = connection_permit;
                        if let Err(error) = transport::serve_connection(stream, state).await {
                            warn!(error = %error, "management request failed");
                        }
                    });
                }
                Err(error) => error!(error = %error, "socket accept failed"),
            },
            _ = signal::ctrl_c() => {
                info!("shutdown requested");
                break;
            }
        }
    }

    if let Err(error) = fs::remove_file(&socket_path).await {
        if error.kind() != io::ErrorKind::NotFound {
            warn!(path = %socket_path.display(), error = %error, "could not remove socket file");
        }
    }
    Ok(())
}
