mod service;

use std::{
    env,
    error::Error,
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
    time::Duration,
};

use clap::{Args, Parser, Subcommand};
use mc_client::DaemonClient;
use mc_config::{Config, ConfigError};
use mc_core::{ApiRequest, ApiResponse, AppError, ErrorCode, ServerInfo};
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(
    name = "mc",
    version,
    about = "Minecraft Java server manager: TUI, daemon, CLI, and user service"
)]
struct Cli {
    /// Configuration path. Defaults to the platform application config directory.
    #[arg(long, global = true, env = "MC_SERVER_TUI_CONFIG")]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Open the terminal UI. Does not start the daemon automatically.
    Tui,
    /// Run the management daemon in the foreground (used by the service unit).
    Daemon {
        #[command(subcommand)]
        command: DaemonCommand,
    },
    /// Forward arguments to the text CLI.
    Cli {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Print version, target, API protocol, and service manager.
    Version {
        #[arg(long)]
        json: bool,
    },
    /// Probe the local daemon.
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Run local doctor checks, printing JSON when requested.
    Doctor {
        #[arg(long)]
        json: bool,
    },
    /// Create a default config, install the user service, and start it.
    Setup(SetupArgs),
    /// Remove a versioned installation and its service; instance data is preserved.
    Uninstall {
        #[arg(long)]
        dry_run: bool,
        /// Remove even if the user service is currently running.
        #[arg(long)]
        force: bool,
    },
    /// Install, remove, start, stop, restart, or inspect the user service.
    Service {
        #[command(subcommand)]
        command: ServiceCommand,
    },
}

#[derive(Debug, Subcommand)]
enum DaemonCommand {
    /// Run the daemon in the foreground; the service unit uses this.
    Run,
}

#[derive(Debug, Args)]
struct SetupArgs {
    /// Override the Paper User-Agent recorded in config.toml.
    #[arg(long)]
    user_agent: Option<String>,
    /// Write config but do not install or start a service.
    #[arg(long)]
    no_service: bool,
    /// Print the config and service plan without writing or executing anything.
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug, Subcommand)]
enum ServiceCommand {
    /// Write the launchd plist or systemd user unit and start the daemon.
    Install {
        #[arg(long)]
        dry_run: bool,
    },
    /// Stop and remove the service unit; instance data is preserved.
    Uninstall {
        #[arg(long)]
        dry_run: bool,
    },
    /// Start the installed service and wait for the daemon socket.
    Start,
    /// Stop the service; resident runners and Java are left alone.
    Stop,
    /// Restart the daemon service and wait for it to become ready.
    Restart,
    /// Show whether the unit exists and whether the daemon is running.
    Status {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Serialize)]
struct VersionInfo {
    product: &'static str,
    version: &'static str,
    target: &'static str,
    profile: &'static str,
    os: &'static str,
    arch: &'static str,
    api_protocol_min: u16,
    api_protocol_max: u16,
    service_manager: &'static str,
}

#[derive(Debug, Serialize)]
struct StatusReport {
    socket: String,
    reachable: bool,
    version: Option<String>,
    api_protocol_min: Option<u16>,
    api_protocol_max: Option<u16>,
    legacy_ping: bool,
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("mc: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    if let Some(Command::Version { json }) = cli.command {
        return print_version(json);
    }
    let config_path = resolve_config_path(cli.config)?;
    let config = Config::load_from(&config_path)?;
    // Children and the service unit must resolve exactly the same file.
    env::set_var("MC_SERVER_TUI_CONFIG", &config_path);

    match cli.command {
        None => {
            ensure_daemon(&config).await?;
            exec_sibling("mc-server-tui", &[]);
        }
        Some(Command::Tui) => exec_sibling("mc-server-tui", &[]),
        Some(Command::Daemon { command }) => match command {
            DaemonCommand::Run => exec_sibling("mc-server-tui-daemon", &[]),
        },
        Some(Command::Cli { args }) => exec_sibling("mc-server-tui-cli", &args),
        Some(Command::Version { json }) => print_version(json)?,
        Some(Command::Status { json }) => print_status(&config, json).await,
        Some(Command::Doctor { json }) => print_doctor(&config, json).await?,
        Some(Command::Setup(args)) => setup(&config_path, config, args).await?,
        Some(Command::Uninstall { dry_run, force }) => uninstall(dry_run, force)?,
        Some(Command::Service { command }) => {
            service_command(&config_path, &config, command).await?
        }
    }
    Ok(())
}

fn resolve_config_path(explicit: Option<PathBuf>) -> Result<PathBuf, ConfigError> {
    let path = match explicit {
        Some(path) => path,
        None => match env::var_os("MC_SERVER_TUI_CONFIG") {
            Some(path) if !path.is_empty() => PathBuf::from(path),
            _ => mc_config::default_config_path()?,
        },
    };
    Ok(std::path::absolute(path)?)
}

fn print_version(json: bool) -> Result<(), Box<dyn Error>> {
    let info = VersionInfo {
        product: "mc-server-tui",
        version: env!("CARGO_PKG_VERSION"),
        target: env!("MC_BUILD_TARGET"),
        profile: env!("MC_BUILD_PROFILE"),
        os: env::consts::OS,
        arch: env::consts::ARCH,
        api_protocol_min: mc_core::API_PROTOCOL_MIN,
        api_protocol_max: mc_core::API_PROTOCOL_MAX,
        service_manager: service::manager_name(),
    };
    if json {
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else {
        println!("{} {}", info.product, info.version);
        println!("target: {}", info.target);
        println!("profile: {}", info.profile);
        println!(
            "API protocol: {}–{}",
            info.api_protocol_min, info.api_protocol_max
        );
        println!("service manager: {}", info.service_manager);
    }
    Ok(())
}

async fn print_status(config: &Config, json: bool) {
    let socket = match config.resolved_socket() {
        Ok(socket) => socket,
        Err(error) => {
            print_status_error("<unknown>", &error.to_string(), json);
            return;
        }
    };
    match probe(&socket).await {
        Ok(Some(info)) => {
            let report = StatusReport {
                socket: socket.display().to_string(),
                reachable: true,
                version: Some(info.version),
                api_protocol_min: Some(info.api_protocol_min),
                api_protocol_max: Some(info.api_protocol_max),
                legacy_ping: false,
            };
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).expect("serialize status")
                );
            } else {
                println!("daemon: running");
                println!("socket: {}", report.socket);
                println!(
                    "version: {} (API {}–{})",
                    report.version.as_deref().unwrap_or("unknown"),
                    report.api_protocol_min.unwrap_or_default(),
                    report.api_protocol_max.unwrap_or_default()
                );
            }
        }
        Ok(None) => {
            let report = StatusReport {
                socket: socket.display().to_string(),
                reachable: true,
                version: None,
                api_protocol_min: None,
                api_protocol_max: None,
                legacy_ping: true,
            };
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).expect("serialize status")
                );
            } else {
                println!("daemon: running (legacy ping; restart the daemon for version info)");
                println!("socket: {}", report.socket);
            }
        }
        Err(error) => print_status_error(&socket.display().to_string(), &error.to_string(), json),
    }
}

fn print_status_error(socket: &str, error: &str, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::json!({
                "socket": socket,
                "reachable": false,
                "error": error,
            })
        );
    } else {
        eprintln!("daemon: unavailable");
        eprintln!("socket: {socket}");
        eprintln!("reason: {error}");
        eprintln!("next: run `mc setup` to install the user service, or `mc daemon run` in another terminal");
    }
    std::process::exit(1);
}

async fn print_doctor(config: &Config, json: bool) -> Result<(), Box<dyn Error>> {
    if !json {
        exec_sibling("mc-server-tui-cli", &["doctor".to_owned()]);
    }
    let client = DaemonClient::new(config.resolved_socket()?);
    match client.request(ApiRequest::Doctor).await? {
        ApiResponse::Doctor(report) => {
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(())
        }
        ApiResponse::Error(error) => Err(error.into()),
        _ => Err("the daemon returned an unexpected response to doctor".into()),
    }
}

async fn probe(socket: &Path) -> Result<Option<ServerInfo>, AppError> {
    let client = DaemonClient::new(socket);
    let probe = tokio::time::timeout(
        Duration::from_secs(2),
        client.request(ApiRequest::ServerInfo),
    )
    .await;
    match probe {
        Ok(Ok(ApiResponse::ServerInfo(info))) => Ok(Some(info)),
        Ok(Ok(ApiResponse::Error(_))) | Ok(Ok(ApiResponse::Pong)) => legacy_ping(&client).await,
        Ok(Ok(_)) => Err(AppError::new(
            ErrorCode::ProtocolInvalid,
            "The daemon returned an unexpected response to a version probe.",
        )),
        Ok(Err(error)) => Err(error),
        Err(_) => Err(AppError::new(
            ErrorCode::RequestTimeout,
            "The daemon did not answer the version probe in time.",
        )),
    }
}

async fn legacy_ping(client: &DaemonClient) -> Result<Option<ServerInfo>, AppError> {
    match tokio::time::timeout(Duration::from_secs(2), client.request(ApiRequest::Ping)).await {
        Ok(Ok(ApiResponse::Pong)) => Ok(None),
        Ok(Ok(ApiResponse::Error(error))) => Err(error),
        Ok(Ok(_)) => Err(AppError::new(
            ErrorCode::ProtocolInvalid,
            "The daemon returned an unexpected response to Ping.",
        )),
        Ok(Err(error)) => Err(error),
        Err(_) => Err(AppError::new(
            ErrorCode::RequestTimeout,
            "The daemon did not answer Ping in time.",
        )),
    }
}

async fn ensure_daemon(config: &Config) -> Result<(), Box<dyn Error>> {
    let socket = config.resolved_socket()?;
    if probe(&socket).await.is_ok() {
        return Ok(());
    }
    let service = service::status();
    if let Ok(status) = service {
        if status.installed {
            if !status.running {
                service::start()?;
            }
            if wait_for_daemon(&socket, Duration::from_secs(10)).await {
                return Ok(());
            }
            return Err(format!(
                "the user service is installed at {} but the daemon did not become ready; run `mc service status` and inspect the service log",
                status.unit_path
            )
            .into());
        }
    }
    Err(format!(
        "no daemon is running at {}.\nRun `mc setup` to install the user service, or `mc daemon run` in another terminal.",
        socket.display()
    )
    .into())
}

async fn wait_for_daemon(socket: &Path, timeout: Duration) -> bool {
    let deadline = tokio::time::Instant::now() + timeout;
    while tokio::time::Instant::now() < deadline {
        if probe(socket).await.is_ok() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    false
}

async fn wait_or_error(config: &Config, action: &str) -> Result<(), Box<dyn Error>> {
    if wait_for_daemon(&config.resolved_socket()?, Duration::from_secs(10)).await {
        Ok(())
    } else {
        Err(format!(
            "{action} but the daemon did not become ready; run `mc service status` and inspect the service log"
        )
        .into())
    }
}

async fn setup(
    config_path: &Path,
    mut config: Config,
    args: SetupArgs,
) -> Result<(), Box<dyn Error>> {
    if let Some(user_agent) = args.user_agent {
        if user_agent.trim().is_empty() {
            return Err("--user-agent cannot be empty".into());
        }
        config.daemon.user_agent = Some(user_agent);
    }
    config.ensure_directories()?;
    if !args.dry_run {
        config.save_to(config_path)?;
    }
    println!("config: {}", config_path.display());
    println!("state: {}", config.resolved_state_dir()?.display());
    println!("socket: {}", config.resolved_socket()?.display());
    println!("user-agent: {}", config.resolved_user_agent());
    if config.resolved_user_agent() == mc_config::default_user_agent() {
        eprintln!(
            "warning: using the default project User-Agent; set --user-agent if this deployment needs its own contact"
        );
    }
    if args.dry_run {
        println!("\n{}", config.to_toml()?);
    }

    let doctor = mc_system::run_doctor().await;
    if let Some(java) = doctor
        .java_runtimes
        .iter()
        .find(|runtime| runtime.major_version.is_some_and(|major| major >= 21))
    {
        println!(
            "java: {} (Java {})",
            java.executable,
            java.major_version
                .map(|major| major.to_string())
                .unwrap_or_else(|| "unknown".to_owned())
        );
    } else if let Some(java) = doctor.java_runtimes.first() {
        println!(
            "java: {} (Java {})",
            java.executable,
            java.major_version
                .map(|major| major.to_string())
                .unwrap_or_else(|| "unknown".to_owned())
        );
        eprintln!(
            "warning: no Java 21+ runtime was found; Paper creation will require a compatible Java"
        );
    } else {
        eprintln!("warning: no Java runtime was found; install Java before creating an instance");
    }

    if args.no_service {
        println!("service: skipped (--no-service)");
        println!("next: run `mc daemon run` in this terminal, then `mc tui` in another");
        return Ok(());
    }

    let launcher = launcher_path()?;
    let plan = service::install(&config, &launcher, config_path, args.dry_run)?;
    if args.dry_run {
        print_service_plan(&plan);
        return Ok(());
    }
    println!(
        "service: {} installed at {}",
        plan.manager,
        plan.unit_path.display()
    );
    if wait_for_daemon(&config.resolved_socket()?, Duration::from_secs(10)).await {
        println!("daemon: ready");
        println!("next: run `mc` to open the TUI");
        Ok(())
    } else {
        Err("the service was installed but the daemon did not become ready; run `mc service status` and inspect the service log".into())
    }
}

fn uninstall(dry_run: bool, force: bool) -> Result<(), Box<dyn Error>> {
    let executable = std::fs::canonicalize(env::current_exe()?)?;
    let root = versioned_root(&executable).ok_or(
        "this is not a versioned installation; remove a source build manually or use the installer layout",
    )?;
    let root = std::fs::canonicalize(&root)?;
    if root == Path::new("/") || root == env::var_os("HOME").map(PathBuf::from).unwrap_or_default()
    {
        return Err("refusing to remove a broad or home directory".into());
    }
    if !root.join("versions").is_dir() {
        return Err(format!(
            "{} does not look like a versioned installation",
            root.display()
        )
        .into());
    }

    let service = service::status().ok();
    if let Some(service) = &service {
        if service.running && !force {
            return Err(
                "the user service is running; stop Minecraft instances and `mc service stop` first, or pass --force"
                    .into(),
            );
        }
    }
    if service.as_ref().is_some_and(|service| service.running) {
        eprintln!("warning: removing the service while it is running; resident runners and Java are not stopped");
    }

    let launcher = launcher_path()?;
    println!("install: {}", root.display());
    println!("launcher: {}", launcher.display());
    if let Some(service) = &service {
        println!("service: {}", service.unit_path);
    } else {
        println!("service: status unavailable");
    }
    if dry_run {
        println!("dry-run: no files or services were removed");
        return Ok(());
    }

    service::uninstall(false)?;
    if launcher != executable {
        match std::fs::symlink_metadata(&launcher) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                let target = std::fs::canonicalize(&launcher)?;
                if target.starts_with(&root) {
                    std::fs::remove_file(&launcher)?;
                    println!("removed launcher: {}", launcher.display());
                } else {
                    eprintln!(
                        "leaving launcher {} because it points outside {}",
                        launcher.display(),
                        root.display()
                    );
                }
            }
            Ok(_) => eprintln!(
                "leaving launcher {} because it is not a symlink",
                launcher.display()
            ),
            Err(_) => {}
        }
    }
    std::fs::remove_dir_all(&root)?;
    println!("removed: {}", root.display());
    println!("instance data, configuration, and cache were preserved");
    Ok(())
}

fn versioned_root(executable: &Path) -> Option<PathBuf> {
    let mut current = executable.parent()?;
    loop {
        if current.file_name().is_some_and(|name| name == "versions") {
            return current.parent().map(Path::to_path_buf);
        }
        current = current.parent()?;
    }
}

async fn service_command(
    config_path: &Path,
    config: &Config,
    command: ServiceCommand,
) -> Result<(), Box<dyn Error>> {
    match command {
        ServiceCommand::Install { dry_run } => {
            if !config_path.is_file() && !dry_run {
                config.save_to(config_path)?;
            }
            if !dry_run {
                config.ensure_directories()?;
            }
            let launcher = launcher_path()?;
            let plan = service::install(config, &launcher, config_path, dry_run)?;
            if dry_run {
                print_service_plan(&plan);
            } else {
                println!(
                    "service: {} installed at {}",
                    plan.manager,
                    plan.unit_path.display()
                );
                wait_or_error(config, "the service was installed").await?;
                println!("next: run `mc` to open the TUI");
            }
            Ok(())
        }
        ServiceCommand::Uninstall { dry_run } => {
            let plan = service::uninstall(dry_run)?;
            println!(
                "service: {} at {}",
                if dry_run { "would remove" } else { "removed" },
                plan.unit_path.display()
            );
            Ok(())
        }
        ServiceCommand::Start => {
            service::start()?;
            wait_or_error(config, "the service was started").await?;
            println!("service: started");
            Ok(())
        }
        ServiceCommand::Stop => {
            service::stop()?;
            println!("service: stopped");
            Ok(())
        }
        ServiceCommand::Restart => {
            service::restart()?;
            wait_or_error(config, "the service was restarted").await?;
            println!("service: restarted");
            Ok(())
        }
        ServiceCommand::Status { json } => {
            let status = service::status()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&status)?);
            } else {
                println!("manager: {}", status.manager);
                println!("unit: {}", status.unit_path);
                println!(
                    "installed: {}; running: {}",
                    status.installed, status.running
                );
            }
            Ok(())
        }
    }
}

fn print_service_plan(plan: &service::ServicePlan) {
    println!("manager: {}", plan.manager);
    println!("unit: {}", plan.unit_path.display());
    println!("commands:");
    for command in &plan.commands {
        println!("  {}", command.join(" "));
    }
    println!("contents:");
    println!("{}", plan.contents);
}

fn launcher_path() -> Result<PathBuf, Box<dyn Error>> {
    let argv0 = env::args_os().next().unwrap_or_default();
    let path = PathBuf::from(argv0);
    if path.components().count() > 1 {
        return Ok(std::path::absolute(path)?);
    }
    if let Some(paths) = env::var_os("PATH") {
        for directory in env::split_paths(&paths) {
            let candidate = directory.join(&path);
            if candidate.is_file() {
                return Ok(std::path::absolute(candidate)?);
            }
        }
    }
    Ok(env::current_exe()?)
}

fn sibling_path(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let launcher = launcher_path()?;
    let resolved = std::fs::canonicalize(&launcher).unwrap_or(launcher);
    let directory = resolved
        .parent()
        .ok_or("could not determine the mc installation directory")?;
    let candidate = directory.join(name);
    if candidate.is_file() {
        Ok(candidate)
    } else {
        Err(format!(
            "{} was not found next to {}; build the workspace with `cargo build --workspace`",
            name,
            resolved.display()
        )
        .into())
    }
}

fn exec_sibling(name: &str, args: &[String]) -> ! {
    let path = match sibling_path(name) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("mc: {error}");
            std::process::exit(127);
        }
    };
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let error = ProcessCommand::new(&path).args(args).exec();
        eprintln!("mc: could not execute {}: {error}", path.display());
        std::process::exit(127);
    }
    #[cfg(not(unix))]
    {
        let _ = (path, args);
        eprintln!("mc: this platform is not supported");
        std::process::exit(127);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_json_contains_protocol_and_target() {
        let info = VersionInfo {
            product: "mc-server-tui",
            version: env!("CARGO_PKG_VERSION"),
            target: env!("MC_BUILD_TARGET"),
            profile: env!("MC_BUILD_PROFILE"),
            os: env::consts::OS,
            arch: env::consts::ARCH,
            api_protocol_min: mc_core::API_PROTOCOL_MIN,
            api_protocol_max: mc_core::API_PROTOCOL_MAX,
            service_manager: service::manager_name(),
        };
        let json = serde_json::to_string(&info).expect("serialize version");
        assert!(json.contains("api_protocol_min"));
        assert!(json.contains("mc-server-tui"));
        assert!(json.contains(env!("MC_BUILD_TARGET")));
    }

    #[test]
    fn versioned_root_is_derived_from_the_binary_path() {
        let root =
            versioned_root(Path::new("/tmp/mcst/versions/0.1.0/bin/mc")).expect("versioned root");
        assert_eq!(root, PathBuf::from("/tmp/mcst"));
        assert!(versioned_root(Path::new("/tmp/mcst/target/debug/mc")).is_none());
    }

    #[test]
    fn config_path_is_absolute() {
        let path = resolve_config_path(Some(PathBuf::from("relative/config.toml")))
            .expect("resolve config path");
        assert!(path.is_absolute());
    }
}
