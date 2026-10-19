mod command;
mod output;

use clap::Parser;
use command::Args;
use mc_client::DaemonClient;
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
    let config = mc_config::Config::load().map_err(|error| {
        AppError::new(
            ErrorCode::Internal,
            "Could not load the local configuration.",
        )
        .with_evidence(error.to_string())
    })?;
    let socket = match args.socket {
        Some(path) => path,
        None => config.resolved_socket().map_err(|error| {
            AppError::new(
                ErrorCode::Internal,
                "Could not determine the daemon socket path.",
            )
            .with_evidence(error.to_string())
        })?,
    };
    let (request, include_experimental) = args.command.into_request();
    let response = DaemonClient::new(socket).request(request).await?;
    output::print_response(response, include_experimental)
}
