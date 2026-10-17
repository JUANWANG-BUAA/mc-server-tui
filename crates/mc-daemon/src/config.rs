use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(version, about = "Local management daemon for mc-server-tui")]
pub(crate) struct Args {
    /// Unix socket path. Defaults to the user's application state directory.
    #[arg(long, env = "MC_SERVER_TUI_SOCKET")]
    pub socket: Option<PathBuf>,
    /// Application state directory. Defaults to the platform application directory.
    #[arg(long, env = "MC_SERVER_TUI_STATE_DIR")]
    pub state_dir: Option<PathBuf>,
    /// Download cache base directory. Defaults to the platform cache directory.
    #[arg(long, env = "MC_SERVER_TUI_CACHE_DIR")]
    pub cache_dir: Option<PathBuf>,
    /// Internal resident runner mode. Managed by the daemon, not a user command.
    #[arg(long, hide = true)]
    pub internal_runner: Option<uuid::Uuid>,
}
