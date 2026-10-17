use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DoctorCheckStatus {
    Ok,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoctorCheck {
    pub id: String,
    pub status: DoctorCheckStatus,
    pub summary: String,
    pub detail: Option<String>,
    pub next_action: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JavaRuntime {
    pub executable: String,
    pub version: String,
    pub major_version: Option<u16>,
    pub architecture: Option<String>,
    pub vendor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoctorReport {
    pub platform: String,
    pub architecture: String,
    pub data_directory: Option<String>,
    pub java_runtimes: Vec<JavaRuntime>,
    pub checks: Vec<DoctorCheck>,
}
