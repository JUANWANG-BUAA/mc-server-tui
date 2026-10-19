use serde::{Deserialize, Serialize};

/// Explicit operations prevent shell execution and keep player actions auditable.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum InstanceOperation {
    Connection,
    Monitor,
    SearchLogs {
        pattern: String,
        regex: bool,
        ignore_case: bool,
    },
    Players {
        command: String,
        player: Option<String>,
        reason: Option<String>,
    },
    SearchAddons {
        query: String,
    },
    InstallAddon {
        project: String,
    },
    ListAddons,
    ExportPack {
        path: std::path::PathBuf,
    },
    ImportPack {
        path: std::path::PathBuf,
        apply: bool,
    },
    CreateFabric {
        name: String,
        game_version: String,
        accept_eula: bool,
    },
}
