use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModpackStatus {
    Ready,
    Review,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModpackSeverity {
    Blocker,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModpackIssue {
    pub severity: ModpackSeverity,
    pub code: String,
    pub title: String,
    pub detail: String,
    pub action: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModpackFileDisposition {
    Install,
    Review,
    Exclude,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModpackFilePlan {
    pub path: String,
    pub disposition: ModpackFileDisposition,
    pub reason: String,
    pub size: Option<u64>,
    pub has_verified_download: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModpackLoader {
    pub dependency: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModpackAnalysisReport {
    pub path: PathBuf,
    pub name: String,
    pub version_id: String,
    pub minecraft_version: Option<String>,
    pub loader: Option<ModpackLoader>,
    pub status: ModpackStatus,
    pub install_files: usize,
    pub review_files: usize,
    pub excluded_files: usize,
    pub mod_files: usize,
    pub total_install_bytes: u64,
    pub server_override_files: usize,
    pub issues: Vec<ModpackIssue>,
    pub files: Vec<ModpackFilePlan>,
    pub files_truncated: bool,
}
