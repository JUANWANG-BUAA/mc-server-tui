use mc_core::{ApiResponse, AppError, DoctorCheckStatus, InstanceSummary, TaskEvent, TaskSnapshot};

pub(crate) fn print_response(
    response: ApiResponse,
    include_experimental: bool,
) -> Result<(), AppError> {
    match response {
        ApiResponse::Pong => println!("daemon: running"),
        ApiResponse::Doctor(report) => {
            println!("Platform: {} ({})", report.platform, report.architecture);
            if let Some(directory) = report.data_directory {
                println!("Data directory: {directory}");
            }
            for check in report.checks {
                let status = match check.status {
                    DoctorCheckStatus::Ok => "OK",
                    DoctorCheckStatus::Warning => "WARN",
                    DoctorCheckStatus::Info => "INFO",
                };
                println!("[{status}] {}: {}", check.id, check.summary);
                if let Some(detail) = check.detail {
                    println!("  {detail}");
                }
                if let Some(action) = check.next_action {
                    println!("  Next: {action}");
                }
            }
            if !report.java_runtimes.is_empty() {
                println!("Java runtimes:");
                for runtime in report.java_runtimes {
                    let major = runtime
                        .major_version
                        .map(|version| version.to_string())
                        .unwrap_or_else(|| "unknown".to_owned());
                    let architecture = runtime.architecture.unwrap_or_else(|| "unknown".to_owned());
                    let vendor = runtime
                        .vendor
                        .unwrap_or_else(|| "unknown vendor".to_owned());
                    println!(
                        "  Java {major} · {} · {vendor} · {}",
                        runtime.version, architecture
                    );
                    println!("    {}", runtime.executable);
                }
            }
        }
        ApiResponse::PaperVersions(versions) => {
            for version in versions {
                println!("{}\t{}", version.id, version.group);
            }
        }
        ApiResponse::PaperBuilds(builds) => {
            for build in builds
                .into_iter()
                .filter(|build| include_experimental || build.channel.is_stable())
            {
                let artifact = build
                    .artifact_name
                    .unwrap_or_else(|| "no server artifact".to_owned());
                let size = build
                    .artifact_size
                    .map(format_bytes)
                    .unwrap_or_else(|| "unknown size".to_owned());
                println!(
                    "{}\t{}\t{}\t{} ({})",
                    build.id,
                    build.channel.0,
                    artifact,
                    size,
                    build
                        .released_at
                        .map(|time| time.to_rfc3339())
                        .unwrap_or_else(|| "release time unknown".to_owned())
                );
            }
        }
        ApiResponse::ArtifactPlan(plan) => {
            println!("Provider: {}", plan.provider);
            println!("Minecraft: {}", plan.game_version);
            println!("Build: {} ({})", plan.build_id, plan.channel.0);
            println!(
                "Artifact: {} ({})",
                plan.artifact_name,
                format_bytes(plan.expected_size)
            );
            println!("SHA-256: {}", plan.checksum.value);
            println!("Download: {}", plan.download_url);
            println!("Resolved: {}", plan.resolved_at.to_rfc3339());
        }
        ApiResponse::Instances(instances) => {
            if instances.is_empty() {
                println!("No instances yet. Create one with `instance create`.");
            } else {
                for instance in instances {
                    print_instance(&instance);
                }
            }
        }
        ApiResponse::InstanceDetail(detail) => {
            print_instance(&detail.instance);
            print_server(&detail.server);
            println!("Java: {}", detail.instance.java_path.display());
            println!(
                "Memory: {}–{} MiB",
                detail.instance.min_memory_mb, detail.instance.max_memory_mb
            );
        }
        ApiResponse::ServerStatus(server) => print_server(&server),
        ApiResponse::ServerLogs(logs) => {
            print!("{}", logs.content);
            if logs.truncated {
                eprintln!("[earlier server log output was rotated]");
            }
            eprintln!("[log byte cursor: {}]", logs.next_offset);
        }
        ApiResponse::ConsoleCommandAccepted => println!("Console command accepted."),
        ApiResponse::TaskQueued(task) => {
            println!("Task queued: {}", task.id);
            print_task(&task);
        }
        ApiResponse::Tasks(tasks) => {
            if tasks.is_empty() {
                println!("No tasks recorded yet.");
            } else {
                for task in tasks {
                    print_task(&task);
                }
            }
        }
        ApiResponse::Task(task) => print_task(&task),
        ApiResponse::TaskEvents(events) => {
            if events.is_empty() {
                println!("No new task events.");
            } else {
                for event in events {
                    print_task_event(&event);
                }
            }
        }
        ApiResponse::Error(error) => return Err(error),
    }
    Ok(())
}

fn print_task(task: &TaskSnapshot) {
    let progress = task
        .progress
        .map(|progress| {
            format!(
                "{}/{} steps",
                progress.completed_steps, progress.total_steps
            )
        })
        .unwrap_or_else(|| "progress pending".to_owned());
    println!(
        "{}\t{:?}\t{:?}\t{}\t{}",
        task.id, task.status, task.phase, progress, task.message
    );
    if let Some(instance_id) = task.instance_id {
        println!("  Instance: {instance_id}");
    }
    if let Some(result) = &task.result {
        match result {
            mc_core::TaskResult::InstanceCreated(instance) => print_instance(instance),
            mc_core::TaskResult::WorldBackupCreated {
                instance_id,
                backup_id,
            } => {
                println!("  Backup: {backup_id}");
                println!("  Instance: {instance_id}");
            }
            mc_core::TaskResult::WorldBackupsListed { instance_id, page } => {
                println!("  Instance: {instance_id}");
                for backup in &page.backups {
                    println!(
                        "  Backup {}\t{}\t{} files\t{}",
                        backup.id,
                        backup.created_at.to_rfc3339(),
                        backup.file_count,
                        format_bytes(backup.total_bytes)
                    );
                }
                if let Some(cursor) = &page.next_cursor {
                    println!("  Next page: backup list {instance_id} --cursor '{cursor}'");
                } else {
                    println!("  No more backups.");
                }
            }
            mc_core::TaskResult::WorldBackupVerified {
                instance_id,
                backup,
            } => {
                println!("  Instance: {instance_id}");
                println!("  Verified backup: {}", backup.id);
                println!("  Created: {}", backup.created_at.to_rfc3339());
                println!(
                    "  Content: {} files, {}",
                    backup.file_count,
                    format_bytes(backup.total_bytes)
                );
            }
        }
    }
    if let Some(error) = &task.error {
        println!("  Error [{}]: {}", error.code, error.reason);
        if let Some(action) = &error.next_action {
            println!("  Next: {action}");
        }
    }
    println!("  Updated: {}", task.updated_at.to_rfc3339());
}

fn print_task_event(event: &TaskEvent) {
    let progress = event
        .progress
        .map(|progress| format!("{}/{}", progress.completed_steps, progress.total_steps))
        .unwrap_or_else(|| "indeterminate".to_owned());
    println!(
        "#{} {} {:?} {:?} [{}] {}",
        event.sequence,
        event.occurred_at.to_rfc3339(),
        event.status,
        event.phase,
        progress,
        event.message
    );
}

fn print_instance(instance: &InstanceSummary) {
    println!(
        "{}\t{}\tPaper {} build {}\tport {}\t{}",
        instance.id,
        instance.name,
        instance.game_version,
        instance.build_id,
        instance.port,
        instance.directory.display()
    );
}

fn print_server(server: &mc_core::ServerSnapshot) {
    println!(
        "Server: {} · {} · pid {}",
        server_state_label(server.state),
        server.address,
        server
            .pid
            .map(|pid| pid.to_string())
            .unwrap_or_else(|| "—".to_owned())
    );
    println!("{}", server.message);
    if let Some(code) = server.last_exit_code {
        println!("Last exit code: {code}");
    }
}

fn server_state_label(state: mc_core::ServerState) -> &'static str {
    match state {
        mc_core::ServerState::Installed => "installed",
        mc_core::ServerState::Starting => "starting",
        mc_core::ServerState::Running => "running",
        mc_core::ServerState::Stopping => "stopping",
        mc_core::ServerState::Stopped => "stopped",
        mc_core::ServerState::Failed => "failed",
        mc_core::ServerState::Unknown => "unknown",
    }
}

fn format_bytes(bytes: u64) -> String {
    const MIB: u64 = 1024 * 1024;
    if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else {
        format!("{bytes} bytes")
    }
}
