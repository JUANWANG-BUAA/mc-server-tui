//! User configuration and platform paths shared by the daemon, TUI, CLI, and
//! the `mc` launcher.
//!
//! Precedence is intentionally simple: environment variables override
//! `config.toml`, which overrides platform defaults. CLI flags override both
//! where a binary exposes them.

use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const PROJECT_QUALIFIER: &str = "org";
const PROJECT_ORGANIZATION: &str = "mc-server-tui";
const PROJECT_APPLICATION: &str = "mc-server-tui";

/// Project contact URL used in Paper's User-Agent. Forks should replace it.
const DEFAULT_PROJECT_URL: &str = "https://github.com/JUANWANG-BUAA/mc-server-tui";

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not determine the platform application directories")]
    ProjectDirs,
    #[error("could not access configuration: {0}")]
    Io(#[from] io::Error),
    #[error("could not parse configuration: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("could not encode configuration: {0}")]
    Encode(#[from] toml::ser::Error),
}

/// The on-disk configuration. Every section has defaults so an older file can
/// be read after a new optional field is introduced.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub install: InstallConfig,
    pub daemon: DaemonConfig,
    pub update: UpdateConfig,
    pub ui: UiConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct InstallConfig {
    /// source | tarball | brew | apt | rpm | self-managed
    pub source: String,
}

impl Default for InstallConfig {
    fn default() -> Self {
        Self {
            source: "source".to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DaemonConfig {
    /// Empty/unset means the platform state directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_dir: Option<PathBuf>,
    /// Empty/unset means `<state_dir>/run/daemon.sock`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub socket: Option<PathBuf>,
    /// Empty/unset means the platform cache directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_dir: Option<PathBuf>,
    /// Paper requires a descriptive User-Agent. An unset/empty value uses the
    /// project default; `mc setup` can record a deployment-specific value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    pub log_level: String,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            state_dir: None,
            socket: None,
            cache_dir: None,
            user_agent: Some(default_user_agent()),
            log_level: "info".to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdateConfig {
    /// stable | beta
    pub channel: String,
    /// Check for a newer signed manifest at startup. Downloading remains off
    /// until the updater is implemented.
    pub auto_check: bool,
    /// Reserved for the signed self-updater; currently always false by default.
    pub auto_install: bool,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            channel: "stable".to_owned(),
            auto_check: true,
            auto_install: false,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
}

pub fn project_dirs() -> Result<ProjectDirs, ConfigError> {
    ProjectDirs::from(PROJECT_QUALIFIER, PROJECT_ORGANIZATION, PROJECT_APPLICATION)
        .ok_or(ConfigError::ProjectDirs)
}

/// The default config path. `MC_SERVER_TUI_CONFIG` takes precedence.
pub fn default_config_path() -> Result<PathBuf, ConfigError> {
    Ok(project_dirs()?.config_dir().join("config.toml"))
}

pub fn config_path() -> Result<PathBuf, ConfigError> {
    match env::var_os("MC_SERVER_TUI_CONFIG") {
        Some(path) if !path.is_empty() => Ok(PathBuf::from(path)),
        _ => default_config_path(),
    }
}

pub fn default_state_dir() -> Result<PathBuf, ConfigError> {
    let directories = project_dirs()?;
    Ok(directories
        .state_dir()
        .unwrap_or(directories.data_local_dir())
        .to_path_buf())
}

pub fn default_cache_dir() -> Result<PathBuf, ConfigError> {
    Ok(project_dirs()?.cache_dir().to_path_buf())
}

pub fn default_user_agent() -> String {
    format!(
        "mc-server-tui/{} (+{DEFAULT_PROJECT_URL})",
        env!("CARGO_PKG_VERSION")
    )
}

impl Config {
    /// Load `MC_SERVER_TUI_CONFIG` or the platform config path. A missing file
    /// yields defaults; a malformed file is an error rather than silently
    /// discarding user choices.
    pub fn load() -> Result<Self, ConfigError> {
        let path = config_path()?;
        let mut config = if path.is_file() {
            let text = fs::read_to_string(&path)?;
            toml::from_str(&text)?
        } else {
            Self::default()
        };
        config.apply_env();
        Ok(config)
    }

    pub fn load_from(path: &Path) -> Result<Self, ConfigError> {
        let mut config = if path.is_file() {
            let text = fs::read_to_string(path)?;
            toml::from_str(&text)?
        } else {
            Self::default()
        };
        config.apply_env();
        Ok(config)
    }

    /// Write the config atomically with owner-only permissions. The parent
    /// directory is created as needed.
    pub fn save(&self) -> Result<PathBuf, ConfigError> {
        let path = config_path()?;
        self.save_to(&path)?;
        Ok(path)
    }

    pub fn save_to(&self, path: &Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            let existed = parent.is_dir();
            fs::create_dir_all(parent)?;
            if !existed {
                set_dir_mode(parent)?;
            }
        }
        let text = toml::to_string_pretty(self)?;
        let temporary = path.with_extension("toml.tmp");
        fs::write(&temporary, text)?;
        set_file_mode(&temporary)?;
        fs::rename(&temporary, path)?;
        Ok(())
    }

    pub fn to_toml(&self) -> Result<String, ConfigError> {
        Ok(toml::to_string_pretty(self)?)
    }

    pub fn from_toml(text: &str) -> Result<Self, ConfigError> {
        Ok(toml::from_str(text)?)
    }

    /// Apply environment overrides. Empty variables are ignored.
    pub fn apply_env(&mut self) {
        if let Some(value) = non_empty_env("MC_SERVER_TUI_STATE_DIR") {
            self.daemon.state_dir = Some(PathBuf::from(value));
        }
        if let Some(value) = non_empty_env("MC_SERVER_TUI_SOCKET") {
            self.daemon.socket = Some(PathBuf::from(value));
        }
        if let Some(value) = non_empty_env("MC_SERVER_TUI_CACHE_DIR") {
            self.daemon.cache_dir = Some(PathBuf::from(value));
        }
        if let Some(value) = non_empty_env("MC_SERVER_TUI_USER_AGENT") {
            self.daemon.user_agent = Some(value);
        }
    }

    pub fn resolved_state_dir(&self) -> Result<PathBuf, ConfigError> {
        match &self.daemon.state_dir {
            Some(path) if !path.as_os_str().is_empty() => Ok(path.clone()),
            _ => default_state_dir(),
        }
    }

    pub fn resolved_socket(&self) -> Result<PathBuf, ConfigError> {
        match &self.daemon.socket {
            Some(path) if !path.as_os_str().is_empty() => Ok(path.clone()),
            _ => Ok(self.resolved_state_dir()?.join("run").join("daemon.sock")),
        }
    }

    pub fn resolved_cache_dir(&self) -> Result<PathBuf, ConfigError> {
        match &self.daemon.cache_dir {
            Some(path) if !path.as_os_str().is_empty() => Ok(path.clone()),
            _ => default_cache_dir(),
        }
    }

    pub fn resolved_user_agent(&self) -> String {
        match &self.daemon.user_agent {
            Some(value) if !value.trim().is_empty() => value.clone(),
            _ => default_user_agent(),
        }
    }

    pub fn log_dir(&self) -> Result<PathBuf, ConfigError> {
        Ok(self.resolved_state_dir()?.join("logs"))
    }

    pub fn daemon_log_path(&self) -> Result<PathBuf, ConfigError> {
        Ok(self.log_dir()?.join("daemon.log"))
    }

    /// Create all directories that must exist before the daemon starts.
    pub fn ensure_directories(&self) -> Result<(), ConfigError> {
        for directory in [
            self.resolved_state_dir()?,
            self.resolved_cache_dir()?,
            self.log_dir()?,
        ] {
            fs::create_dir_all(&directory)?;
            set_dir_mode(&directory)?;
        }
        if let Some(parent) = self.resolved_socket()?.parent() {
            fs::create_dir_all(parent)?;
            set_dir_mode(parent)?;
        }
        Ok(())
    }
}

fn non_empty_env(name: &str) -> Option<String> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string_lossy().into_owned())
}

#[cfg(unix)]
fn set_dir_mode(path: &Path) -> Result<(), ConfigError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_dir_mode(_path: &Path) -> Result<(), ConfigError> {
    Ok(())
}

#[cfg(unix)]
fn set_file_mode(path: &Path) -> Result<(), ConfigError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_file_mode(_path: &Path) -> Result<(), ConfigError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_round_trips_through_toml() {
        let config = Config::default();
        let text = config.to_toml().expect("serialize config");
        let decoded = Config::from_toml(&text).expect("deserialize config");
        assert_eq!(decoded, config);
        assert!(text.contains("[daemon]"));
        assert!(text.contains("[update]"));
        assert!(text.contains("auto_install = false"));
    }

    #[test]
    fn default_user_agent_has_project_identity() {
        let user_agent = default_user_agent();
        assert!(user_agent.starts_with("mc-server-tui/"));
        assert!(user_agent.contains(DEFAULT_PROJECT_URL));
    }

    #[test]
    fn resolved_paths_honor_explicit_values() {
        let config = Config {
            daemon: DaemonConfig {
                state_dir: Some(PathBuf::from("/tmp/mcst-state")),
                socket: Some(PathBuf::from("/tmp/mcst.sock")),
                cache_dir: Some(PathBuf::from("/tmp/mcst-cache")),
                user_agent: Some("test-agent/1.0 (test@example.invalid)".to_owned()),
                log_level: "debug".to_owned(),
            },
            ..Config::default()
        };
        assert_eq!(
            config.resolved_state_dir().expect("state"),
            PathBuf::from("/tmp/mcst-state")
        );
        assert_eq!(
            config.resolved_socket().expect("socket"),
            PathBuf::from("/tmp/mcst.sock")
        );
        assert_eq!(
            config.resolved_cache_dir().expect("cache"),
            PathBuf::from("/tmp/mcst-cache")
        );
        assert_eq!(
            config.resolved_user_agent(),
            "test-agent/1.0 (test@example.invalid)"
        );
    }

    #[test]
    fn unset_state_dir_uses_platform_default() {
        let config = Config::default();
        let state = config.resolved_state_dir().expect("state");
        let socket = config.resolved_socket().expect("socket");
        assert_eq!(socket, state.join("run").join("daemon.sock"));
    }
}
