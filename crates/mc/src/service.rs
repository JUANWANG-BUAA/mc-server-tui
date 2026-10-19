//! User-level service management for launchd (macOS) and systemd (Linux).
//!
//! The generated units deliberately use `AbandonProcessGroup` / `KillMode=process`
//! so restarting the management daemon does not kill the resident runner or the
//! Minecraft Java process it owns.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use mc_config::{Config, ConfigError};
use serde::Serialize;
use thiserror::Error;

#[cfg(target_os = "macos")]
const LABEL: &str = "org.mc-server-tui.daemon";
#[cfg(target_os = "linux")]
const SYSTEMD_UNIT: &str = "mc-server-tui.service";

#[derive(Debug, Error)]
pub enum ServiceError {
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    #[error("this platform does not have a supported user service manager")]
    Unsupported,
    #[error("could not determine the user home directory")]
    NoHome,
    #[error("could not access service files: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not resolve configuration paths: {0}")]
    Config(#[from] ConfigError),
    #[error("service command failed: {program} {arguments}: {stderr}")]
    Command {
        program: String,
        arguments: String,
        stderr: String,
    },
    #[error("the service manager is not available: {0}")]
    ManagerUnavailable(String),
    #[cfg(target_os = "macos")]
    #[error("the service is not installed")]
    NotInstalled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ServiceStatus {
    pub manager: String,
    pub installed: bool,
    pub running: bool,
    pub unit_path: String,
}

#[derive(Debug, Clone)]
pub struct ServicePlan {
    pub manager: String,
    pub unit_path: PathBuf,
    pub contents: String,
    pub commands: Vec<Vec<String>>,
}

pub fn manager_name() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "launchd"
    }
    #[cfg(target_os = "linux")]
    {
        "systemd"
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        "unsupported"
    }
}

pub fn plan_install(
    config: &Config,
    launcher: &Path,
    config_path: &Path,
) -> Result<ServicePlan, ServiceError> {
    #[cfg(target_os = "macos")]
    {
        plan_launchd(config, launcher, config_path)
    }
    #[cfg(target_os = "linux")]
    {
        plan_systemd(config, launcher, config_path)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = (config, launcher, config_path);
        Err(ServiceError::Unsupported)
    }
}

pub fn install(
    config: &Config,
    launcher: &Path,
    config_path: &Path,
    dry_run: bool,
) -> Result<ServicePlan, ServiceError> {
    let plan = plan_install(config, launcher, config_path)?;
    if dry_run {
        return Ok(plan);
    }
    if let Some(parent) = plan.unit_path.parent() {
        let existed = parent.is_dir();
        fs::create_dir_all(parent)?;
        if !existed {
            set_dir_mode(parent)?;
        }
    }
    if let Some(parent) = config.daemon_log_path()?.parent() {
        let existed = parent.is_dir();
        fs::create_dir_all(parent)?;
        if !existed {
            set_dir_mode(parent)?;
        }
    }
    fs::write(&plan.unit_path, &plan.contents)?;
    set_file_mode(&plan.unit_path)?;
    #[cfg(target_os = "macos")]
    {
        install_launchd(&plan)?;
    }
    #[cfg(target_os = "linux")]
    {
        install_systemd(&plan)?;
    }
    Ok(plan)
}

pub fn start() -> Result<(), ServiceError> {
    #[cfg(target_os = "macos")]
    {
        launchd_start()
    }
    #[cfg(target_os = "linux")]
    {
        systemd_command(&["start", SYSTEMD_UNIT])
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Err(ServiceError::Unsupported)
    }
}

pub fn stop() -> Result<(), ServiceError> {
    #[cfg(target_os = "macos")]
    {
        launchd_stop()
    }
    #[cfg(target_os = "linux")]
    {
        systemd_command(&["stop", SYSTEMD_UNIT])
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Err(ServiceError::Unsupported)
    }
}

pub fn restart() -> Result<(), ServiceError> {
    #[cfg(target_os = "macos")]
    {
        launchd_restart()
    }
    #[cfg(target_os = "linux")]
    {
        systemd_command(&["restart", SYSTEMD_UNIT])
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Err(ServiceError::Unsupported)
    }
}

pub fn uninstall(dry_run: bool) -> Result<ServicePlan, ServiceError> {
    #[cfg(target_os = "macos")]
    {
        let plan = launchd_plan_uninstall()?;
        if !dry_run {
            launchd_uninstall(&plan)?;
        }
        Ok(plan)
    }
    #[cfg(target_os = "linux")]
    {
        let plan = systemd_plan_uninstall()?;
        if !dry_run {
            systemd_uninstall(&plan)?;
        }
        Ok(plan)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = dry_run;
        Err(ServiceError::Unsupported)
    }
}

pub fn status() -> Result<ServiceStatus, ServiceError> {
    #[cfg(target_os = "macos")]
    {
        launchd_status()
    }
    #[cfg(target_os = "linux")]
    {
        systemd_status()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Err(ServiceError::Unsupported)
    }
}

fn set_dir_mode(path: &Path) -> Result<(), ServiceError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

fn set_file_mode(path: &Path) -> Result<(), ServiceError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

fn home_dir() -> Result<PathBuf, ServiceError> {
    env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or(ServiceError::NoHome)
}

fn command_output(program: &str, arguments: &[&str]) -> Result<Output, ServiceError> {
    Command::new(program)
        .args(arguments)
        .output()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                ServiceError::ManagerUnavailable(format!("`{program}` was not found on PATH"))
            } else {
                ServiceError::Io(error)
            }
        })
}

fn check_output(program: &str, arguments: &[&str], output: &Output) -> Result<(), ServiceError> {
    if output.status.success() {
        return Ok(());
    }
    Err(ServiceError::Command {
        program: program.to_owned(),
        arguments: arguments.join(" "),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
    })
}

fn command_plan(
    manager: &str,
    unit_path: &Path,
    contents: String,
    commands: Vec<Vec<String>>,
) -> ServicePlan {
    ServicePlan {
        manager: manager.to_owned(),
        unit_path: unit_path.to_path_buf(),
        contents,
        commands,
    }
}

#[cfg(target_os = "macos")]
fn launchd_plist(
    config: &Config,
    launcher: &Path,
    config_path: &Path,
) -> Result<String, ServiceError> {
    let log_path = config.daemon_log_path()?;
    let launcher = xml_escape(&launcher.display().to_string());
    let config_path = xml_escape(&config_path.display().to_string());
    let log_path = xml_escape(&log_path.display().to_string());
    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>{LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{launcher}</string>
    <string>daemon</string>
    <string>run</string>
  </array>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
  <key>AbandonProcessGroup</key><true/>
  <key>ProcessType</key><string>Standard</string>
  <key>StandardOutPath</key><string>{log_path}</string>
  <key>StandardErrorPath</key><string>{log_path}</string>
  <key>EnvironmentVariables</key>
  <dict>
    <key>MC_SERVER_TUI_CONFIG</key><string>{config_path}</string>
  </dict>
</dict>
</plist>
"#
    ))
}

#[cfg(target_os = "macos")]
fn plan_launchd(
    config: &Config,
    launcher: &Path,
    config_path: &Path,
) -> Result<ServicePlan, ServiceError> {
    let uid = unsafe { libc::geteuid() };
    let unit_path = home_dir()?
        .join("Library/LaunchAgents")
        .join(format!("{LABEL}.plist"));
    let contents = launchd_plist(config, launcher, config_path)?;
    let domain = format!("gui/{uid}");
    let service = format!("{domain}/{LABEL}");
    let commands = vec![
        vec![
            "launchctl".to_owned(),
            "bootout".to_owned(),
            service.clone(),
        ],
        vec![
            "launchctl".to_owned(),
            "bootstrap".to_owned(),
            domain,
            unit_path.display().to_string(),
        ],
        vec![
            "launchctl".to_owned(),
            "kickstart".to_owned(),
            "-k".to_owned(),
            service,
        ],
    ];
    Ok(command_plan("launchd", &unit_path, contents, commands))
}

#[cfg(target_os = "macos")]
fn install_launchd(plan: &ServicePlan) -> Result<(), ServiceError> {
    let uid = unsafe { libc::geteuid() };
    let domain = format!("gui/{uid}");
    let service = format!("{domain}/{LABEL}");
    let _ = command_output("launchctl", &["bootout", &service]);
    let arguments = ["bootstrap", &domain, &plan.unit_path.display().to_string()];
    let output = command_output("launchctl", &arguments)?;
    check_output("launchctl", &arguments, &output)?;
    let arguments = ["kickstart", "-k", &service];
    let output = command_output("launchctl", &arguments)?;
    check_output("launchctl", &arguments, &output)
}

#[cfg(target_os = "macos")]
fn launchd_start() -> Result<(), ServiceError> {
    let uid = unsafe { libc::geteuid() };
    let domain = format!("gui/{uid}");
    let service = format!("{domain}/{LABEL}");
    let plist = home_dir()?
        .join("Library/LaunchAgents")
        .join(format!("{LABEL}.plist"));
    if !plist.is_file() {
        return Err(ServiceError::NotInstalled);
    }
    let print = command_output("launchctl", &["print", &service])?;
    if print.status.success() {
        let arguments = ["kickstart", "-k", &service];
        let output = command_output("launchctl", &arguments)?;
        check_output("launchctl", &arguments, &output)
    } else {
        let arguments = ["bootstrap", &domain, &plist.display().to_string()];
        let output = command_output("launchctl", &arguments)?;
        check_output("launchctl", &arguments, &output)
    }
}

#[cfg(target_os = "macos")]
fn launchd_stop() -> Result<(), ServiceError> {
    let uid = unsafe { libc::geteuid() };
    let service = format!("gui/{uid}/{LABEL}");
    let print = command_output("launchctl", &["print", &service])?;
    if !print.status.success() {
        return Ok(());
    }
    let output = command_output("launchctl", &["bootout", &service])?;
    check_output("launchctl", &["bootout", &service], &output)
}

#[cfg(target_os = "macos")]
fn launchd_restart() -> Result<(), ServiceError> {
    let uid = unsafe { libc::geteuid() };
    let service = format!("gui/{uid}/{LABEL}");
    let print = command_output("launchctl", &["print", &service])?;
    if !print.status.success() {
        return launchd_start();
    }
    let arguments = ["kickstart", "-k", &service];
    let output = command_output("launchctl", &arguments)?;
    check_output("launchctl", &arguments, &output)
}

#[cfg(target_os = "macos")]
fn launchd_plan_uninstall() -> Result<ServicePlan, ServiceError> {
    let uid = unsafe { libc::geteuid() };
    let unit_path = home_dir()?
        .join("Library/LaunchAgents")
        .join(format!("{LABEL}.plist"));
    let service = format!("gui/{uid}/{LABEL}");
    Ok(command_plan(
        "launchd",
        &unit_path,
        String::new(),
        vec![
            vec!["launchctl".to_owned(), "bootout".to_owned(), service],
            vec!["rm".to_owned(), unit_path.display().to_string()],
        ],
    ))
}

#[cfg(target_os = "macos")]
fn launchd_uninstall(plan: &ServicePlan) -> Result<(), ServiceError> {
    let uid = unsafe { libc::geteuid() };
    let service = format!("gui/{uid}/{LABEL}");
    let _ = command_output("launchctl", &["bootout", &service]);
    if plan.unit_path.is_file() {
        fs::remove_file(&plan.unit_path)?;
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn launchd_status() -> Result<ServiceStatus, ServiceError> {
    let unit_path = home_dir()?
        .join("Library/LaunchAgents")
        .join(format!("{LABEL}.plist"));
    if !unit_path.is_file() {
        return Ok(ServiceStatus {
            manager: manager_name().to_owned(),
            installed: false,
            running: false,
            unit_path: unit_path.display().to_string(),
        });
    }
    let uid = unsafe { libc::geteuid() };
    let service = format!("gui/{uid}/{LABEL}");
    let output = command_output("launchctl", &["print", &service])?;
    let running = output.status.success()
        && String::from_utf8_lossy(&output.stdout).contains("state = running");
    Ok(ServiceStatus {
        manager: manager_name().to_owned(),
        installed: true,
        running,
        unit_path: unit_path.display().to_string(),
    })
}

#[cfg(target_os = "linux")]
fn plan_systemd(
    config: &Config,
    launcher: &Path,
    config_path: &Path,
) -> Result<ServicePlan, ServiceError> {
    let unit_path = home_dir()?.join(".config/systemd/user").join(SYSTEMD_UNIT);
    let contents = systemd_unit(config, launcher, config_path)?;
    let commands = vec![
        vec![
            "systemctl".to_owned(),
            "--user".to_owned(),
            "daemon-reload".to_owned(),
        ],
        vec![
            "systemctl".to_owned(),
            "--user".to_owned(),
            "enable".to_owned(),
            "--now".to_owned(),
            SYSTEMD_UNIT.to_owned(),
        ],
    ];
    Ok(command_plan("systemd", &unit_path, contents, commands))
}

#[cfg(target_os = "linux")]
fn systemd_unit(
    _config: &Config,
    launcher: &Path,
    config_path: &Path,
) -> Result<String, ServiceError> {
    let launcher = systemd_quote(&launcher.display().to_string());
    let environment = systemd_quote(&format!("MC_SERVER_TUI_CONFIG={}", config_path.display()));
    Ok(format!(
        r#"[Unit]
Description=mc-server-tui management daemon

[Service]
Type=simple
ExecStart={launcher} daemon run
Environment={environment}
Restart=on-failure
RestartSec=2
KillMode=process
KillSignal=SIGTERM
TimeoutStopSec=20
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=default.target
"#
    ))
}

#[cfg(target_os = "linux")]
fn install_systemd(_plan: &ServicePlan) -> Result<(), ServiceError> {
    let reload = command_output("systemctl", &["--user", "daemon-reload"])?;
    check_output("systemctl", &["--user", "daemon-reload"], &reload)?;
    let arguments = ["--user", "enable", "--now", SYSTEMD_UNIT];
    let output = command_output("systemctl", &arguments)?;
    check_output("systemctl", &arguments, &output)
}

#[cfg(target_os = "linux")]
fn systemd_command(arguments: &[&str]) -> Result<(), ServiceError> {
    let mut full = vec!["--user"];
    full.extend_from_slice(arguments);
    let output = command_output("systemctl", &full)?;
    check_output("systemctl", &full, &output)
}

#[cfg(target_os = "linux")]
fn systemd_plan_uninstall() -> Result<ServicePlan, ServiceError> {
    let unit_path = home_dir()?.join(".config/systemd/user").join(SYSTEMD_UNIT);
    Ok(command_plan(
        "systemd",
        &unit_path,
        String::new(),
        vec![
            vec![
                "systemctl".to_owned(),
                "--user".to_owned(),
                "disable".to_owned(),
                "--now".to_owned(),
                SYSTEMD_UNIT.to_owned(),
            ],
            vec!["rm".to_owned(), unit_path.display().to_string()],
            vec![
                "systemctl".to_owned(),
                "--user".to_owned(),
                "daemon-reload".to_owned(),
            ],
        ],
    ))
}

#[cfg(target_os = "linux")]
fn systemd_uninstall(plan: &ServicePlan) -> Result<(), ServiceError> {
    let _ = command_output("systemctl", &["--user", "disable", "--now", SYSTEMD_UNIT]);
    if plan.unit_path.is_file() {
        fs::remove_file(&plan.unit_path)?;
    }
    let _ = command_output("systemctl", &["--user", "daemon-reload"]);
    Ok(())
}

#[cfg(target_os = "linux")]
fn systemd_status() -> Result<ServiceStatus, ServiceError> {
    let unit_path = home_dir()?.join(".config/systemd/user").join(SYSTEMD_UNIT);
    if !unit_path.is_file() {
        return Ok(ServiceStatus {
            manager: manager_name().to_owned(),
            installed: false,
            running: false,
            unit_path: unit_path.display().to_string(),
        });
    }
    let output = command_output("systemctl", &["--user", "is-active", SYSTEMD_UNIT])?;
    let state = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    Ok(ServiceStatus {
        manager: manager_name().to_owned(),
        installed: true,
        running: state == "active",
        unit_path: unit_path.display().to_string(),
    })
}

#[cfg(target_os = "macos")]
fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(target_os = "linux")]
fn systemd_quote(value: &str) -> String {
    let mut quoted = String::from("\"");
    for character in value.chars() {
        match character {
            '\\' => quoted.push_str("\\\\"),
            '"' => quoted.push_str("\\\""),
            '%' => quoted.push_str("%%"),
            _ => quoted.push(character),
        }
    }
    quoted.push('"');
    quoted
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_config() -> Config {
        Config {
            daemon: mc_config::DaemonConfig {
                state_dir: Some(PathBuf::from("/tmp/mcst state")),
                ..mc_config::DaemonConfig::default()
            },
            ..Config::default()
        }
    }

    #[test]
    fn manager_name_is_known_on_supported_platforms() {
        assert!(matches!(manager_name(), "launchd" | "systemd"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn launchd_plist_preserves_runner_after_restart() {
        let plan = plan_install(
            &test_config(),
            Path::new("/tmp/mc"),
            Path::new("/tmp/mc config.toml"),
        )
        .expect("launchd plan");
        assert!(plan.contents.contains("AbandonProcessGroup"));
        assert!(plan.contents.contains("daemon</string>"));
        assert!(plan.contents.contains("MC_SERVER_TUI_CONFIG"));
        assert!(plan.contents.contains("/tmp/mc config.toml"));
        assert!(plan.unit_path.ends_with("org.mc-server-tui.daemon.plist"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn launchd_xml_escapes_paths() {
        let plan = plan_install(
            &test_config(),
            Path::new("/tmp/a&b"),
            Path::new("/tmp/c<d>.toml"),
        )
        .expect("launchd plan");
        assert!(plan.contents.contains("a&amp;b"));
        assert!(plan.contents.contains("c&lt;d&gt;.toml"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn systemd_unit_keeps_runner_out_of_kill_scope() {
        let plan = plan_install(
            &test_config(),
            Path::new("/tmp/mc"),
            Path::new("/tmp/mc config.toml"),
        )
        .expect("systemd plan");
        assert!(plan.contents.contains("KillMode=process"));
        assert!(plan.contents.contains("Restart=on-failure"));
        assert!(plan.contents.contains("ExecStart=\"/tmp/mc\" daemon run"));
        assert!(plan
            .contents
            .contains("Environment=\"MC_SERVER_TUI_CONFIG=/tmp/mc config.toml\""));
        assert!(plan.unit_path.ends_with("mc-server-tui.service"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn systemd_quotes_paths_and_percent_signs() {
        assert_eq!(systemd_quote("/tmp/a b"), "\"/tmp/a b\"");
        assert_eq!(systemd_quote("/tmp/100%/a\"b"), "\"/tmp/100%%/a\\\"b\"");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn generated_unit_passes_systemd_analyze() {
        let plan = plan_install(
            &test_config(),
            Path::new("/bin/true"),
            Path::new("/tmp/mc config.toml"),
        )
        .expect("systemd plan");
        let dir = std::env::temp_dir().join(format!("mcst-systemd-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("mc-server-tui.service");
        std::fs::write(&path, &plan.contents).expect("write unit");
        let output = Command::new("systemd-analyze")
            .arg("verify")
            .arg(&path)
            .output();
        let _ = std::fs::remove_dir_all(&dir);
        match output {
            Ok(output) => assert!(
                output.status.success(),
                "systemd-analyze verify failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("could not run systemd-analyze: {error}"),
        }
    }
}
