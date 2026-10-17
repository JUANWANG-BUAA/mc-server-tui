mod client;
mod command;
mod output;

use std::{io, path::PathBuf};

use clap::Parser;
use command::Args;
use directories::ProjectDirs;
use mc_core::{AppError, ErrorCode};

#[tokio::main]
async fn main() {
    if let Err(error) = run(Args::parse()).await {
        eprintln!("Error [{}]: {}", error.code, error.reason);
        if let Some(action) = error.next_action {
            eprintln!("Next: {action}");
        }
        if let Some(evidence) = error.evidence {
            eprintln!("Evidence: {evidence}");
        }
        if error.retryable {
            eprintln!("This operation can be retried.");
        }
        std::process::exit(1);
    }
}

async fn run(args: Args) -> Result<(), AppError> {
    let socket = match args.socket {
        Some(path) => path,
        None => default_socket_path().map_err(|error| {
            AppError::new(
                ErrorCode::Internal,
                "Could not determine the daemon socket path.",
            )
            .with_evidence(error.to_string())
        })?,
    };
    let (request, include_experimental) = args.command.into_request();
    let response = client::request_daemon(&socket, request).await?;
    output::print_response(response, include_experimental)
}

fn default_socket_path() -> Result<PathBuf, io::Error> {
    ProjectDirs::from("org", "mc-server-tui", "mc-server-tui")
        .map(|dirs| {
            dirs.state_dir()
                .unwrap_or(dirs.data_local_dir())
                .join("run")
                .join("daemon.sock")
        })
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "could not determine application data directory",
            )
        })
}
