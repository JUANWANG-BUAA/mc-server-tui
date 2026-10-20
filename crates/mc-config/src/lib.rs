//! User configuration and platform paths shared by the daemon, TUI, CLI, and
//! the `mc` launcher.
//!
//! Precedence is intentionally simple: environment variables override
//! `config.toml`, which overrides platform defaults. CLI flags override both
//! where a binary exposes them.

use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const PROJECT_QUALIFIER: &str = "org";
const PROJECT_ORGANIZATION: &str = "mc-server-tui";
const PROJECT_APPLICATION: &str = "mc-server-tui";
static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

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
        Self::load_from(&path)
    }

    pub fn load_from(path: &Path) -> Result<Self, ConfigError> {
        let path = resolve_config_path(path)?;
        let mut config = match fs::metadata(&path) {
            Ok(metadata) if metadata.is_file() => {
                let text = fs::read_to_string(path)?;
                toml::from_str(&text)?
            }
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "configuration path is not a regular file",
                )
                .into());
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Self::default(),
            Err(error) => return Err(error.into()),
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

    /// Atomically replace a config file with owner-only Unix permissions.
    ///
    /// A directory-sync error is reported after the rename commit point, so
    /// the new file may already be visible even when this returns an error.
    pub fn save_to(&self, path: &Path) -> Result<(), ConfigError> {
        let path = resolve_config_path(path)?;
        let parent = parent_directory(&path);
        let parent_existed = parent.is_dir();
        fs::create_dir_all(parent)?;
        if !parent_existed {
            set_dir_mode(parent)?;
        }

        let text = toml::to_string_pretty(self)?;
        let mut temporary = create_temporary_file(parent)?;
        let file = temporary
            .file
            .as_mut()
            .expect("new temporary config file is open");
        file.write_all(text.as_bytes())?;
        file.flush()?;
        file.sync_all()?;
        drop(temporary.file.take());

        fs::rename(&temporary.path, &path)?;
        temporary.committed = true;
        sync_directory(parent).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!(
                    "configuration was replaced, but directory durability could not be confirmed: {error}"
                ),
            )
        })?;
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

fn resolve_config_path(path: &Path) -> Result<PathBuf, ConfigError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Ok(fs::canonicalize(path)?),
        Ok(_) => Ok(path.to_path_buf()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(path.to_path_buf()),
        Err(error) => Err(error.into()),
    }
}

fn parent_directory(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

struct TemporaryConfigFile {
    path: PathBuf,
    file: Option<File>,
    committed: bool,
}

impl Drop for TemporaryConfigFile {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn create_temporary_file(parent: &Path) -> Result<TemporaryConfigFile, ConfigError> {
    loop {
        let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(
            ".mc-server-tui-config-{}-{sequence}.tmp",
            std::process::id()
        ));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);

        match options.open(&path) {
            Ok(file) => {
                return Ok(TemporaryConfigFile {
                    path,
                    file: Some(file),
                    committed: false,
                });
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> io::Result<()> {
    File::open(path)?.sync_all()
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> io::Result<()> {
    Ok(())
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

    #[test]
    fn loading_defaults_only_for_a_missing_file() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let missing = directory.path().join("missing.toml");
        let mut expected = Config::default();
        expected.apply_env();
        assert_eq!(
            Config::load_from(&missing).expect("missing defaults"),
            expected
        );

        let error = Config::load_from(directory.path()).expect_err("directory is not a config");
        assert!(matches!(error, ConfigError::Io(_)));

        let malformed = directory.path().join("malformed.toml");
        fs::write(&malformed, "[daemon\n").expect("write malformed config");
        assert!(matches!(
            Config::load_from(&malformed),
            Err(ConfigError::Parse(_))
        ));
    }

    #[test]
    fn saving_replaces_config_and_leaves_no_temporary_file() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let config_path = directory.path().join("config.toml");
        let config = Config {
            install: InstallConfig {
                source: "self-managed".to_owned(),
            },
            ..Config::default()
        };

        config.save_to(&config_path).expect("save config");
        let saved = fs::read_to_string(&config_path).expect("read saved config");
        assert_eq!(
            Config::from_toml(&saved).expect("parse saved config"),
            config
        );
        assert_eq!(
            fs::read_dir(directory.path())
                .expect("read config directory")
                .count(),
            1
        );

        let blocked_path = directory.path().join("blocked.toml");
        fs::create_dir(&blocked_path).expect("create directory at config path");
        assert!(config.save_to(&blocked_path).is_err());
        assert_eq!(
            fs::read_dir(directory.path())
                .expect("read config directory after failure")
                .count(),
            2
        );
    }

    #[cfg(unix)]
    #[test]
    fn config_file_and_new_parent_have_private_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().expect("temporary directory");
        let parent = directory.path().join("private").join("config");
        let path = parent.join("config.toml");
        Config::default().save_to(&path).expect("save config");

        assert_eq!(
            fs::metadata(&parent).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[cfg(unix)]
    #[test]
    fn loading_and_saving_a_symlink_use_its_target() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().expect("temporary directory");
        let target = directory.path().join("target.toml");
        let link = directory.path().join("config.toml");
        let original = Config {
            install: InstallConfig {
                source: "brew".to_owned(),
            },
            ..Config::default()
        };
        fs::write(&target, original.to_toml().expect("serialize config"))
            .expect("write target config");
        symlink(&target, &link).expect("create config symlink");

        let mut expected = original.clone();
        expected.apply_env();
        assert_eq!(Config::load_from(&link).expect("load symlink"), expected);

        let updated = Config {
            install: InstallConfig {
                source: "apt".to_owned(),
            },
            ..Config::default()
        };
        updated.save_to(&link).expect("save through symlink");
        assert!(fs::symlink_metadata(&link)
            .expect("symlink remains")
            .file_type()
            .is_symlink());
        assert_eq!(
            Config::from_toml(&fs::read_to_string(&target).expect("read target"))
                .expect("parse target"),
            updated
        );

        let missing_target = directory.path().join("missing-target.toml");
        let dangling_link = directory.path().join("dangling.toml");
        symlink(&missing_target, &dangling_link).expect("create dangling symlink");
        assert!(matches!(
            Config::load_from(&dangling_link),
            Err(ConfigError::Io(error)) if error.kind() == io::ErrorKind::NotFound
        ));
    }

    #[test]
    fn relative_config_paths_use_the_current_directory() {
        assert_eq!(parent_directory(Path::new("config.toml")), Path::new("."));
    }
}
