//! Doctor checks for local storage, ports, and platform state.

use std::{
    env,
    net::TcpListener,
    path::{Path, PathBuf},
};

use directories::ProjectDirs;
use fs2::available_space;
use mc_core::{DoctorCheck, DoctorCheckStatus, DoctorReport};
use tokio::{
    fs::{self as async_fs, OpenOptions},
    io::AsyncWriteExt,
};
use uuid::Uuid;

const DEFAULT_MINECRAFT_PORT: u16 = 25565;

struct ProbeCleanup {
    path: PathBuf,
    armed: bool,
}

impl ProbeCleanup {
    fn new(path: PathBuf) -> Self {
        Self { path, armed: false }
    }

    fn arm(&mut self) {
        self.armed = true;
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for ProbeCleanup {
    fn drop(&mut self) {
        if self.armed {
            let path = self.path.clone();
            if let Ok(runtime) = tokio::runtime::Handle::try_current() {
                runtime.spawn(async move {
                    let _ = async_fs::remove_file(path).await;
                });
            }
        }
    }
}

pub async fn run_doctor() -> DoctorReport {
    let state_directory = data_directory();
    let mut checks = vec![DoctorCheck {
        id: "platform".to_owned(),
        status: DoctorCheckStatus::Info,
        summary: format!("{} ({})", env::consts::OS, env::consts::ARCH),
        detail: Some("The base target is local macOS and Linux management.".to_owned()),
        next_action: None,
    }];

    let port_check = tokio::task::spawn_blocking(check_default_port);
    let (directory_result, java_runtimes, port_check) = tokio::join!(
        check_data_directory(state_directory.as_deref()),
        super::java::discover_java_runtimes(),
        port_check
    );
    checks.push(directory_result.0);
    if let Some(check) = directory_result.1 {
        checks.push(check);
    }
    checks.push(port_check.unwrap_or_else(|error| DoctorCheck {
        id: "default_port".to_owned(),
        status: DoctorCheckStatus::Warning,
        summary: "Could not check whether the default port is available.".to_owned(),
        detail: Some(error.to_string()),
        next_action: Some("Retry doctor and inspect system resource limits.".to_owned()),
    }));
    checks.push(if java_runtimes.is_empty() {
        DoctorCheck {
            id: "java".to_owned(),
            status: DoctorCheckStatus::Warning,
            summary: "No usable Java runtime was found.".to_owned(),
            detail: Some("Paper instances must be bound to a compatible Java executable before startup.".to_owned()),
            next_action: Some("Install a supported Java runtime or configure a managed JDK, then run doctor again.".to_owned()),
        }
    } else {
        DoctorCheck {
            id: "java".to_owned(),
            status: DoctorCheckStatus::Ok,
            summary: format!("Found {} usable Java runtime(s).", java_runtimes.len()),
            detail: Some("Compatibility is checked per server version when an instance is created or started.".to_owned()),
            next_action: None,
        }
    });

    DoctorReport {
        platform: env::consts::OS.to_owned(),
        architecture: env::consts::ARCH.to_owned(),
        data_directory: state_directory.map(|path| path.display().to_string()),
        java_runtimes,
        checks,
    }
}

fn data_directory() -> Option<PathBuf> {
    ProjectDirs::from("org", "mc-server-tui", "mc-server-tui").map(|directories| {
        directories
            .state_dir()
            .unwrap_or(directories.data_local_dir())
            .to_path_buf()
    })
}

async fn check_data_directory(path: Option<&Path>) -> (DoctorCheck, Option<DoctorCheck>) {
    let Some(path) = path else {
        return (
            DoctorCheck {
                id: "data_directory".to_owned(),
                status: DoctorCheckStatus::Warning,
                summary: "Could not determine the application data directory.".to_owned(),
                detail: None,
                next_action: Some(
                    "Set a supported home directory and run doctor again.".to_owned(),
                ),
            },
            None,
        );
    };

    if let Err(error) = async_fs::create_dir_all(path).await {
        return (
            DoctorCheck {
                id: "data_directory".to_owned(),
                status: DoctorCheckStatus::Warning,
                summary: "The application data directory is not writable.".to_owned(),
                detail: Some(error.to_string()),
                next_action: Some(
                    "Choose a writable data directory or correct its permissions.".to_owned(),
                ),
            },
            None,
        );
    }

    let probe_path = path.join(format!(".doctor-{}", Uuid::new_v4()));
    let mut cleanup = ProbeCleanup::new(probe_path.clone());
    let probe_result = async {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&probe_path)
            .await?;
        cleanup.arm();
        file.write_all(b"mc-server-tui doctor probe").await?;
        file.sync_all().await
    }
    .await;
    let cleanup_result = if cleanup.armed {
        async_fs::remove_file(&probe_path).await
    } else {
        Ok(())
    };
    if cleanup_result.is_ok() {
        cleanup.disarm();
    }
    let probe_result = match probe_result {
        Ok(()) => cleanup_result,
        Err(error) => {
            let _ = cleanup_result;
            Err(error)
        }
    };
    let data_check = match probe_result {
        Ok(()) => DoctorCheck {
            id: "data_directory".to_owned(),
            status: DoctorCheckStatus::Ok,
            summary: "Application data directory is writable.".to_owned(),
            detail: Some(path.display().to_string()),
            next_action: None,
        },
        Err(error) => DoctorCheck {
            id: "data_directory".to_owned(),
            status: DoctorCheckStatus::Warning,
            summary: "The application data directory is not writable.".to_owned(),
            detail: Some(error.to_string()),
            next_action: Some(
                "Choose a writable data directory or correct its permissions.".to_owned(),
            ),
        },
    };

    let disk_path = path.to_path_buf();
    let disk_space = tokio::task::spawn_blocking(move || available_space(disk_path)).await;
    let disk_check = match disk_space {
        Ok(Ok(bytes)) => DoctorCheck {
            id: "disk_space".to_owned(),
            status: DoctorCheckStatus::Info,
            summary: format!("{} free in the application data volume.", format_bytes(bytes)),
            detail: Some("Available space is a point-in-time reading; an install preflight will check its exact needs.".to_owned()),
            next_action: None,
        },
        Ok(Err(error)) => DoctorCheck {
            id: "disk_space".to_owned(),
            status: DoctorCheckStatus::Warning,
            summary: "Could not read available disk space.".to_owned(),
            detail: Some(error.to_string()),
            next_action: Some("Check the filesystem and available disk space.".to_owned()),
        },
        Err(error) => DoctorCheck {
            id: "disk_space".to_owned(),
            status: DoctorCheckStatus::Warning,
            summary: "Could not read available disk space.".to_owned(),
            detail: Some(error.to_string()),
            next_action: Some("Check the filesystem and available disk space.".to_owned()),
        },
    };
    (data_check, Some(disk_check))
}

fn check_default_port() -> DoctorCheck {
    match TcpListener::bind(("127.0.0.1", DEFAULT_MINECRAFT_PORT)) {
        Ok(listener) => {
            drop(listener);
            DoctorCheck {
                id: "default_port".to_owned(),
                status: DoctorCheckStatus::Ok,
                summary: format!(
                    "TCP port {DEFAULT_MINECRAFT_PORT} appears available on localhost."
                ),
                detail: Some(
                    "This is a point-in-time check and does not reserve the port.".to_owned(),
                ),
                next_action: None,
            }
        }
        Err(error) => DoctorCheck {
            id: "default_port".to_owned(),
            status: DoctorCheckStatus::Warning,
            summary: format!("TCP port {DEFAULT_MINECRAFT_PORT} could not be bound on localhost."),
            detail: Some(error.to_string()),
            next_action: Some(
                "Choose another port or stop the process currently using 25565.".to_owned(),
            ),
        },
    }
}

fn format_bytes(bytes: u64) -> String {
    const GIB: u64 = 1024 * 1024 * 1024;
    const MIB: u64 = 1024 * 1024;
    if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else {
        format!("{bytes} bytes")
    }
}
