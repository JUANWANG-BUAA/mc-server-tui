use std::path::PathBuf;

use clap::{Parser, Subcommand};
use mc_core::ApiRequest;
use uuid::Uuid;

#[derive(Debug, Parser)]
#[command(version, about = "Minecraft Java server management CLI")]
pub(crate) struct Args {
    /// Unix socket path. Defaults to the user's application state directory.
    #[arg(long, global = true, env = "MC_SERVER_TUI_SOCKET")]
    pub socket: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Check local Java runtimes, data directory, disk space, and the default port.
    Doctor,
    /// Check whether the local management daemon is available.
    Status,
    /// Query the official Paper metadata service through the daemon.
    Paper {
        #[command(subcommand)]
        command: PaperCommand,
    },
    /// Create and inspect persistent server instances.
    Instance {
        #[command(subcommand)]
        command: InstanceCommand,
    },
    /// Inspect durable background operations.
    Task {
        #[command(subcommand)]
        command: TaskCommand,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum InstanceCommand {
    List,
    Create {
        name: String,
        game_version: String,
        #[arg(long)]
        build_id: Option<u64>,
        /// Absolute path to the Java executable selected for this instance.
        #[arg(long)]
        java: PathBuf,
        #[arg(long, default_value_t = 1024)]
        min_memory_mb: u32,
        #[arg(long, default_value_t = 2048)]
        max_memory_mb: u32,
        #[arg(long, default_value_t = 25565)]
        port: u16,
        /// Explicitly record acceptance of the Minecraft EULA.
        #[arg(long, action = clap::ArgAction::SetTrue)]
        accept_eula: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum PaperCommand {
    Versions,
    Builds {
        game_version: String,
        #[arg(long)]
        include_experimental: bool,
    },
    Plan {
        game_version: String,
        #[arg(long)]
        build_id: Option<u64>,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum TaskCommand {
    List {
        #[arg(long, default_value_t = 20)]
        limit: u16,
    },
    Show {
        task_id: Uuid,
    },
    Events {
        task_id: Uuid,
        #[arg(long, default_value_t = 0)]
        after_sequence: i64,
    },
}

impl Command {
    pub fn into_request(self) -> (ApiRequest, bool) {
        match self {
            Self::Doctor => (ApiRequest::Doctor, false),
            Self::Status => (ApiRequest::Ping, false),
            Self::Paper { command } => match command {
                PaperCommand::Versions => (ApiRequest::PaperVersions, false),
                PaperCommand::Builds {
                    game_version,
                    include_experimental,
                } => (
                    ApiRequest::PaperBuilds { game_version },
                    include_experimental,
                ),
                PaperCommand::Plan {
                    game_version,
                    build_id,
                } => (
                    ApiRequest::PaperPlan {
                        game_version,
                        build_id,
                    },
                    false,
                ),
            },
            Self::Instance { command } => match command {
                InstanceCommand::List => (ApiRequest::ListInstances, false),
                InstanceCommand::Create {
                    name,
                    game_version,
                    build_id,
                    java,
                    min_memory_mb,
                    max_memory_mb,
                    port,
                    accept_eula,
                } => (
                    ApiRequest::CreatePaperInstance {
                        name,
                        game_version,
                        build_id,
                        java_path: java,
                        min_memory_mb,
                        max_memory_mb,
                        port,
                        accept_eula,
                    },
                    false,
                ),
            },
            Self::Task { command } => match command {
                TaskCommand::List { limit } => (ApiRequest::ListTasks { limit }, false),
                TaskCommand::Show { task_id } => (ApiRequest::GetTask { task_id }, false),
                TaskCommand::Events {
                    task_id,
                    after_sequence,
                } => (
                    ApiRequest::ListTaskEvents {
                        task_id,
                        after_sequence,
                    },
                    false,
                ),
            },
        }
    }
}
