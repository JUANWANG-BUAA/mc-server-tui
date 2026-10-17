use std::path::PathBuf;

use clap::{Parser, Subcommand};
use mc_core::{ApiRequest, BackupCursor};
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
    /// Create world backups for stopped instances.
    Backup {
        #[command(subcommand)]
        command: BackupCommand,
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
pub(crate) enum BackupCommand {
    /// Queue a checksummed backup of a stopped instance's world directories.
    Create { instance_id: Uuid },
    /// Queue a bounded page of backup summaries, verifying the returned page.
    List {
        instance_id: Uuid,
        #[arg(long, default_value_t = 20)]
        limit: u16,
        #[arg(long)]
        cursor: Option<BackupCursor>,
    },
    /// Queue an integrity verification for one backup ID.
    Verify { instance_id: Uuid, backup_id: Uuid },
    /// Verify a backup and list every world root affected by a restore.
    PlanRestore { instance_id: Uuid, backup_id: Uuid },
    /// Queue a restore after copying the token from a successful plan task.
    Restore {
        instance_id: Uuid,
        backup_id: Uuid,
        #[arg(long)]
        confirm_token: String,
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
            Self::Backup { command } => match command {
                BackupCommand::Create { instance_id } => {
                    (ApiRequest::CreateWorldBackup { instance_id }, false)
                }
                BackupCommand::List {
                    instance_id,
                    limit,
                    cursor,
                } => (
                    ApiRequest::ListWorldBackups {
                        instance_id,
                        limit,
                        cursor,
                    },
                    false,
                ),
                BackupCommand::Verify {
                    instance_id,
                    backup_id,
                } => (
                    ApiRequest::VerifyWorldBackup {
                        instance_id,
                        backup_id,
                    },
                    false,
                ),
                BackupCommand::PlanRestore {
                    instance_id,
                    backup_id,
                } => (
                    ApiRequest::PlanWorldBackupRestore {
                        instance_id,
                        backup_id,
                    },
                    false,
                ),
                BackupCommand::Restore {
                    instance_id,
                    backup_id,
                    confirm_token,
                } => (
                    ApiRequest::RestoreWorldBackup {
                        instance_id,
                        backup_id,
                        confirmation_token: confirm_token,
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

#[cfg(test)]
mod tests {
    use clap::Parser;
    use mc_core::ApiRequest;
    use uuid::Uuid;

    use super::Args;

    #[test]
    fn backup_create_cli_command_maps_to_the_shared_api() {
        let instance_id = Uuid::new_v4();
        let args = Args::try_parse_from([
            "mc-server-tui",
            "backup",
            "create",
            &instance_id.to_string(),
        ])
        .expect("parse backup command");
        let (request, _) = args.command.into_request();

        assert!(matches!(
            request,
            ApiRequest::CreateWorldBackup { instance_id: parsed_id } if parsed_id == instance_id
        ));
    }

    #[test]
    fn backup_list_and_verify_cli_commands_map_to_the_shared_api() {
        let instance_id = Uuid::new_v4();
        let backup_id = Uuid::new_v4();
        let cursor: mc_core::BackupCursor = format!("2026-10-17T00:00:00Z~{backup_id}")
            .parse()
            .expect("parse cursor");
        let cursor_text = cursor.to_string();
        let args = Args::try_parse_from([
            "mc-server-tui",
            "backup",
            "list",
            &instance_id.to_string(),
            "--limit",
            "7",
            "--cursor",
            &cursor_text,
        ])
        .expect("parse backup list command");
        let (request, _) = args.command.into_request();
        assert!(matches!(
            request,
            ApiRequest::ListWorldBackups {
                instance_id: parsed_instance_id,
                limit: 7,
                cursor: Some(parsed_cursor),
            } if parsed_instance_id == instance_id && parsed_cursor == cursor
        ));

        let args = Args::try_parse_from([
            "mc-server-tui",
            "backup",
            "verify",
            &instance_id.to_string(),
            &backup_id.to_string(),
        ])
        .expect("parse backup verify command");
        let (request, _) = args.command.into_request();
        assert!(matches!(
            request,
            ApiRequest::VerifyWorldBackup {
                instance_id: parsed_instance_id,
                backup_id: parsed_backup_id,
            } if parsed_instance_id == instance_id && parsed_backup_id == backup_id
        ));
    }

    #[test]
    fn restore_plan_and_confirm_cli_commands_map_to_the_shared_api() {
        let instance_id = Uuid::new_v4();
        let backup_id = Uuid::new_v4();
        let token = format!("v1:{}", "a".repeat(64));
        let args = Args::try_parse_from([
            "mc-server-tui",
            "backup",
            "plan-restore",
            &instance_id.to_string(),
            &backup_id.to_string(),
        ])
        .expect("parse restore plan command");
        let (request, _) = args.command.into_request();
        assert!(matches!(
            request,
            ApiRequest::PlanWorldBackupRestore {
                instance_id: parsed_instance_id,
                backup_id: parsed_backup_id,
            } if parsed_instance_id == instance_id && parsed_backup_id == backup_id
        ));

        let args = Args::try_parse_from([
            "mc-server-tui",
            "backup",
            "restore",
            &instance_id.to_string(),
            &backup_id.to_string(),
            "--confirm-token",
            &token,
        ])
        .expect("parse confirmed restore command");
        let (request, _) = args.command.into_request();
        assert!(matches!(
            request,
            ApiRequest::RestoreWorldBackup {
                instance_id: parsed_instance_id,
                backup_id: parsed_backup_id,
                confirmation_token,
            } if parsed_instance_id == instance_id
                && parsed_backup_id == backup_id
                && confirmation_token == token
        ));
    }
}
