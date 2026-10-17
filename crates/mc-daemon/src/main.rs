mod config;
mod dispatch;
mod state;
mod transport;

use std::{io, os::unix::fs::PermissionsExt, sync::Arc};

use clap::Parser;
use config::Args;
use directories::ProjectDirs;
use mc_core::{AppError, ErrorCode, ServerProvider};
use mc_download::{DownloadConfig, DownloadManager};
use mc_instance::InstanceService;
use mc_task::TaskManager;
use provider_paper::PaperProvider;
use state::DaemonState;
use tokio::{fs, net::UnixListener, signal, sync::Semaphore};
use tracing::{error, info, warn};

const MAX_CONNECTIONS: usize = 64;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let args = Args::parse();
    let user_agent = std::env::var("MC_SERVER_TUI_USER_AGENT").map_err(|_| {
        AppError::new(
            ErrorCode::UserAgentRequired,
            "Paper User-Agent is required.",
        )
    })?;
    let provider: Arc<dyn ServerProvider> = Arc::new(PaperProvider::new(Some(user_agent.clone()))?);
    let project_dirs = ProjectDirs::from("org", "mc-server-tui", "mc-server-tui")
        .ok_or("could not determine application directories")?;
    let state_dir = args.state_dir.unwrap_or_else(|| {
        project_dirs
            .state_dir()
            .unwrap_or(project_dirs.data_local_dir())
            .to_path_buf()
    });
    let cache_dir = args
        .cache_dir
        .unwrap_or_else(|| project_dirs.cache_dir().to_path_buf());
    let private_parent = args.socket.is_none();
    let socket_path = args
        .socket
        .unwrap_or_else(|| state_dir.join("run").join("daemon.sock"));
    let downloader = DownloadManager::new(DownloadConfig::paper(cache_dir, user_agent))?;
    let instances = InstanceService::open(&state_dir).await?;
    let tasks = TaskManager::open(state_dir.join("index.sqlite3")).await?;
    let state = Arc::new(DaemonState {
        provider,
        downloader,
        instances,
        tasks,
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
