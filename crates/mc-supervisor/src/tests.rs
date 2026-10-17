use std::{path::PathBuf, time::Duration};

use chrono::Utc;
use mc_backup::{BackupConsistency, BackupService};
use mc_core::{InstanceSummary, ServerState};
use mc_instance::{InstanceConfig, InstanceLock, InstanceService};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{storage, ProcessSupervisor};

#[tokio::test]
async fn stopped_instance_guard_excludes_lifecycle_changes_and_running_worlds() {
    let mut fixture = Fixture::create().await;
    fixture.release_port();
    let supervisor = ProcessSupervisor::new(fixture.instances.clone());
    let guard = supervisor
        .lock_stopped_instance(fixture.instance.id)
        .await
        .expect("reserve a stopped instance");
    assert_eq!(guard.instance().id, fixture.instance.id);

    let start_supervisor = supervisor.clone();
    let instance_id = fixture.instance.id;
    let mut start = tokio::spawn(async move { start_supervisor.start(instance_id).await });
    assert!(tokio::time::timeout(Duration::from_millis(100), &mut start)
        .await
        .is_err());
    drop(guard);

    let started = tokio::time::timeout(Duration::from_secs(4), start)
        .await
        .expect("start continues after releasing the guard")
        .expect("start task joins")
        .expect("server starts");
    assert!(matches!(
        started.state,
        ServerState::Starting | ServerState::Running
    ));

    let denied = match supervisor.lock_stopped_instance(fixture.instance.id).await {
        Ok(_guard) => panic!("running server was reserved for world changes"),
        Err(error) => error,
    };
    assert_eq!(denied.code, mc_core::ErrorCode::InstanceBusy);

    let stopped = supervisor
        .stop(fixture.instance.id, Some(Duration::from_secs(2)))
        .await
        .expect("stop fake server");
    assert_eq!(stopped.state, ServerState::Stopped);
    fixture.remove().await;
}

#[tokio::test]
async fn backup_service_uses_the_supervisor_lease_and_releases_it_after_copy() {
    let mut fixture = Fixture::create().await;
    fixture.release_port();
    let server_dir = fixture.instance.directory.join("server");
    tokio::fs::write(server_dir.join("server.properties"), "level-name=world\n")
        .await
        .expect("write world configuration");
    tokio::fs::create_dir_all(server_dir.join("world/region"))
        .await
        .expect("create world region");
    tokio::fs::write(server_dir.join("world/region/r.0.0.mca"), b"world data")
        .await
        .expect("write world data");

    let supervisor = ProcessSupervisor::new(fixture.instances.clone());
    let guard = supervisor
        .lock_stopped_instance(fixture.instance.id)
        .await
        .expect("reserve stopped instance");
    let backup_id = Uuid::new_v4();
    let manifest = BackupService
        .create(guard, backup_id)
        .await
        .expect("create backup through the async API");
    assert_eq!(manifest.instance_id, fixture.instance.id);
    assert_eq!(manifest.consistency, BackupConsistency::ServerStopped);
    assert!(manifest
        .entries
        .iter()
        .any(|entry| entry.path == "world/region/r.0.0.mca"));

    let listed = BackupService
        .list(&fixture.instance)
        .await
        .expect("list backup through async API");
    assert_eq!(listed, [manifest]);
    fixture.remove().await;
}

#[tokio::test]
async fn starts_idempotently_accepts_console_input_and_stops_gracefully() {
    let mut fixture = Fixture::create().await;
    fixture.release_port();
    let supervisor = ProcessSupervisor::new(fixture.instances.clone());
    let result = lifecycle_sequence(&supervisor, &fixture.instance).await;
    let _ = supervisor
        .stop(fixture.instance.id, Some(Duration::from_secs(2)))
        .await;
    fixture.remove().await;
    assert!(result.is_ok(), "{}", result.unwrap_err());
}

#[tokio::test]
async fn unknown_pid_is_never_signalled_or_replaced() {
    let fixture = Fixture::create().await;
    let supervisor = ProcessSupervisor::new(fixture.instances.clone());
    let record = storage::RuntimeRecord {
        schema_version: storage::RUNTIME_SCHEMA_VERSION,
        instance_id: fixture.instance.id,
        pid: std::process::id(),
        java_pid: None,
        marker: "tests::runner_entry".to_owned(),
        started_at: Utc::now(),
        stopping: false,
        last_exit_code: None,
        last_error: None,
    };
    storage::write_record(&fixture.instance, &record)
        .await
        .expect("write unverified PID fixture");

    let reservation = match supervisor.lock_stopped_instance(fixture.instance.id).await {
        Ok(_guard) => panic!("unverified process was reserved for world changes"),
        Err(error) => error,
    };
    assert_eq!(reservation.code, mc_core::ErrorCode::ServerStatusUnknown);

    let result = supervisor
        .start(fixture.instance.id)
        .await
        .expect("unknown process is returned as state, not killed");
    assert_eq!(result.state, ServerState::Unknown);
    assert_eq!(result.pid, Some(std::process::id()));
    assert!(tokio::fs::try_exists(&fixture.java_path)
        .await
        .expect("inspect Java fixture"));
    fixture.remove().await;
}

#[tokio::test]
async fn occupied_port_without_verified_process_is_unknown_and_not_started() {
    let fixture = Fixture::create().await;
    let supervisor = ProcessSupervisor::new(fixture.instances.clone());

    let detail = supervisor
        .detail(fixture.instance.id)
        .await
        .expect("inspect occupied port");
    let start = supervisor
        .start(fixture.instance.id)
        .await
        .expect("refuse to start on occupied port");
    assert_eq!(detail.server.state, ServerState::Unknown);
    assert_eq!(start.state, ServerState::Unknown);
    assert!(start.pid.is_none());
    let reservation = match supervisor.lock_stopped_instance(fixture.instance.id).await {
        Ok(_guard) => panic!("occupied port was reserved for world changes"),
        Err(error) => error,
    };
    assert_eq!(reservation.code, mc_core::ErrorCode::ServerStatusUnknown);
    fixture.remove().await;
}

#[tokio::test]
async fn server_remains_manageable_after_daemon_process_exits() {
    let mut fixture = Fixture::create().await;
    fixture.release_port();
    let helper = tokio::process::Command::new(std::env::current_exe().expect("test binary path"))
        .args([
            "--exact",
            "tests::helper_start_server_and_exit",
            "--nocapture",
        ])
        .env("MC_SUPERVISOR_HELPER_STATE_DIR", &fixture.root)
        .env(
            "MC_SUPERVISOR_HELPER_INSTANCE_ID",
            fixture.instance.id.to_string(),
        )
        .output()
        .await
        .expect("run daemon-exit helper process");
    assert!(
        helper.status.success(),
        "helper process failed: {}",
        String::from_utf8_lossy(&helper.stderr)
    );

    let restarted_supervisor = ProcessSupervisor::new(fixture.instances.clone());
    let mut detail = restarted_supervisor
        .detail(fixture.instance.id)
        .await
        .expect("reattach after helper process exit");
    for _ in 0..40 {
        if detail.server.state == ServerState::Running {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
        detail = restarted_supervisor
            .detail(fixture.instance.id)
            .await
            .expect("refresh reattached process");
    }
    assert_eq!(detail.server.state, ServerState::Running);
    let stopped = restarted_supervisor
        .stop(fixture.instance.id, Some(Duration::from_secs(3)))
        .await
        .expect("stop reattached server");
    assert_eq!(stopped.state, ServerState::Stopped);
    fixture.remove().await;
}

#[tokio::test]
async fn helper_start_server_and_exit() {
    let (Ok(state_dir), Ok(instance_id)) = (
        std::env::var("MC_SUPERVISOR_HELPER_STATE_DIR"),
        std::env::var("MC_SUPERVISOR_HELPER_INSTANCE_ID"),
    ) else {
        return;
    };
    let instance_id = Uuid::parse_str(&instance_id).expect("helper instance ID");
    let instances = InstanceService::open(state_dir)
        .await
        .expect("open helper instance service");
    let supervisor = ProcessSupervisor::new(instances);
    let server = supervisor
        .start(instance_id)
        .await
        .expect("start helper server");
    println!("ORPHAN_SERVER_PID={}", server.pid.expect("server PID"));
}

#[tokio::test]
async fn runner_entry() {
    let (Ok(state_dir), Ok(instance_id)) = (
        std::env::var("MC_SUPERVISOR_RUNNER_STATE_DIR"),
        std::env::var("MC_SUPERVISOR_RUNNER_INSTANCE_ID"),
    ) else {
        return;
    };
    let instance_id = Uuid::parse_str(&instance_id).expect("runner instance ID");
    let instances = InstanceService::open(state_dir)
        .await
        .expect("open runner instance service");
    super::run_instance_runner(instances, instance_id)
        .await
        .expect("run instance runner");
}

#[test]
fn console_command_rejects_line_breaks_and_control_characters() {
    assert!(super::validate_console_command("say hello").is_ok());
    assert!(super::validate_console_command("stop\nkill").is_err());
    assert!(super::validate_console_command("say\u{1b}red").is_err());
    assert!(super::validate_console_command(&"x".repeat(513)).is_err());
}

async fn lifecycle_sequence(
    supervisor: &ProcessSupervisor,
    instance: &InstanceSummary,
) -> Result<(), String> {
    let first = supervisor
        .start(instance.id)
        .await
        .map_err(|error| error.to_string())?;
    let first_pid = first.pid.ok_or("start returned no process ID")?;
    let runtime = storage::read_record(instance)
        .await
        .map_err(|error| error.to_string())?
        .ok_or("start returned without a runtime record")?;
    if runtime.java_pid != Some(first_pid) {
        return Err("snapshot PID is not the managed Java process ID".to_owned());
    }
    if runtime.pid == first_pid {
        return Err("runner PID was exposed as the Java process ID".to_owned());
    }
    let mut verified = false;
    for _ in 0..40 {
        let detail = supervisor
            .detail(instance.id)
            .await
            .map_err(|error| error.to_string())?;
        if matches!(
            detail.server.state,
            ServerState::Starting | ServerState::Running
        ) {
            verified = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    if !verified {
        return Err("new process identity was not verified".to_owned());
    }
    let second = supervisor
        .start(instance.id)
        .await
        .map_err(|error| format!("idempotent start failed: {error}"))?;
    if second.pid != Some(first_pid) {
        return Err("repeated start did not reuse the verified process".to_owned());
    }

    let mut ready = false;
    for _ in 0..40 {
        let detail = supervisor
            .detail(instance.id)
            .await
            .map_err(|error| error.to_string())?;
        if detail.server.state == ServerState::Running {
            ready = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    if !ready {
        return Err("fake server did not reach its ready state".to_owned());
    }

    supervisor
        .send_console_command(instance.id, "say lifecycle test")
        .await
        .map_err(|error| error.to_string())?;
    let mut saw_command = false;
    for _ in 0..40 {
        let logs = supervisor
            .read_logs(instance.id, 0, 4096)
            .await
            .map_err(|error| error.to_string())?;
        if logs.content.contains("COMMAND:say lifecycle test") {
            saw_command = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    if !saw_command {
        return Err("console command was not captured in the log".to_owned());
    }

    let stopped = supervisor
        .stop(instance.id, Some(Duration::from_secs(3)))
        .await
        .map_err(|error| error.to_string())?;
    if stopped.state != ServerState::Stopped {
        return Err(format!("graceful stop returned {:?}", stopped.state));
    }
    Ok(())
}

struct Fixture {
    root: PathBuf,
    java_path: PathBuf,
    port_reservation: Option<std::net::TcpListener>,
    instances: InstanceService,
    instance: InstanceSummary,
}

impl Fixture {
    async fn create() -> Self {
        use std::os::unix::fs::PermissionsExt;

        let root = std::env::temp_dir().join(format!("mc-supervisor-{}", Uuid::new_v4()));
        let id = Uuid::new_v4();
        let port_reservation =
            std::net::TcpListener::bind(("0.0.0.0", 0)).expect("reserve a free test port");
        let port = port_reservation
            .local_addr()
            .expect("read test port")
            .port();
        let instance_dir = root.join("instances").join(id.to_string());
        for directory in ["server", "logs", "backups", "tmp"] {
            tokio::fs::create_dir_all(instance_dir.join(directory))
                .await
                .expect("create instance fixture directory");
        }
        let java_path = root.join("fake-java");
        tokio::fs::write(
            &java_path,
            b"#!/bin/sh\nprintf 'Booting fake server\\nDone (0.2s)! For help, type help\\n'\nwhile IFS= read -r input; do\n  printf 'COMMAND:%s\\n' \"$input\"\n  if [ \"$input\" = stop ]; then\n    printf 'Stopping server\\n'\n    exit 0\n  fi\ndone\n",
        )
        .await
        .expect("write fake Java executable");
        tokio::fs::set_permissions(&java_path, std::fs::Permissions::from_mode(0o700))
            .await
            .expect("make fake Java executable runnable");
        let java_path = tokio::fs::canonicalize(java_path)
            .await
            .expect("canonicalize fake Java executable");
        tokio::fs::write(instance_dir.join("server/server.jar"), b"fake jar")
            .await
            .expect("write fake server JAR");

        let now = Utc::now();
        let config = InstanceConfig {
            schema_version: 1,
            id,
            name: "Supervisor test".to_owned(),
            provider: "paper".to_owned(),
            project: "paper".to_owned(),
            game_version: "1.21.11".to_owned(),
            build_id: 1,
            java_path: java_path.clone(),
            min_memory_mb: 512,
            max_memory_mb: 1024,
            port,
            eula_accepted: true,
            created_at: now,
            updated_at: now,
        };
        tokio::fs::write(
            instance_dir.join("instance.toml"),
            toml::to_string_pretty(&config).expect("encode instance config"),
        )
        .await
        .expect("write instance config");
        let digest = format!("{:x}", Sha256::digest(b"fake jar"));
        let lock = InstanceLock {
            schema_version: 1,
            provider: "paper".to_owned(),
            project: "paper".to_owned(),
            game_version: "1.21.11".to_owned(),
            build_id: 1,
            channel: "stable".to_owned(),
            metadata_url: "https://fill.papermc.io/v3/projects/paper".to_owned(),
            artifact_name: "paper.jar".to_owned(),
            artifact_size: b"fake jar".len() as u64,
            upstream_sha256: digest.clone(),
            local_sha256: digest,
            resolved_at: now,
            installed_at: now,
        };
        tokio::fs::write(
            instance_dir.join("lock.json"),
            serde_json::to_vec_pretty(&lock).expect("encode instance lock"),
        )
        .await
        .expect("write instance lock");

        let instances = InstanceService::open(&root)
            .await
            .expect("open instance service fixture");
        let instance = instances
            .get_async(id)
            .await
            .expect("load fixture instance");
        Self {
            root,
            java_path,
            port_reservation: Some(port_reservation),
            instances,
            instance,
        }
    }

    async fn remove(self) {
        let _ = tokio::fs::remove_dir_all(self.root).await;
    }

    fn release_port(&mut self) {
        self.port_reservation.take();
    }
}
