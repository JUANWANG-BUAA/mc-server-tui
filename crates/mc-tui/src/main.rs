mod app;
mod runtime;
mod source;
mod task_detail;
mod task_detail_view;
mod task_presentation;
mod terminal;
mod view;
mod wizard;
mod wizard_view;

use std::{path::PathBuf, sync::Arc};

use clap::Parser;
use mc_client::{default_socket_path, DaemonClient};
use source::DaemonDashboardSource;
use terminal::TerminalSession;

#[derive(Debug, Parser)]
#[command(version, about = "Minecraft Java server management TUI")]
struct Args {
    /// Unix socket path. Defaults to the user's application state directory.
    #[arg(long, env = "MC_SERVER_TUI_SOCKET")]
    socket: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    if let Err(error) = run(Args::parse()).await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

async fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    let socket = match args.socket {
        Some(path) => path,
        None => default_socket_path()?,
    };
    let source = Arc::new(DaemonDashboardSource::new(DaemonClient::new(socket)));
    let mut terminal = TerminalSession::enter()?;
    let result = runtime::run_app(&mut terminal.terminal, source).await;
    drop(terminal);
    result?;
    Ok(())
}
