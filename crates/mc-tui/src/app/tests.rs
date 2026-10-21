use chrono::Utc;
use mc_core::{
    ErrorCode, InstanceSummary, TaskEvent, TaskKind, TaskPhase, TaskProgress, TaskSnapshot,
    TaskStatus,
};
use uuid::Uuid;

use crate::instance_detail::{InstanceCommand, InstanceCommandResult};
use crate::task_detail::TaskDetailUpdate;

use super::{Action, AppState, DashboardData};

#[test]
fn failed_refresh_keeps_the_last_successful_frame_data() {
    let mut state = AppState::default();
    state.apply_refresh(Ok(DashboardData {
        servers: Vec::new(),
        instances: vec![InstanceSummary {
            id: Uuid::new_v4(),
            name: "生存服".to_owned(),
            provider: "Paper".to_owned(),
            game_version: "1.21.11".to_owned(),
            build_id: 17,
            java_path: "/opt/java/bin/java".into(),
            min_memory_mb: 1024,
            max_memory_mb: 4096,
            port: 25565,
            directory: "/tmp/server".into(),
            created_at: Utc::now(),
        }],
        tasks: Vec::new(),
    }));
    state.update(Action::Next);
    state.apply_refresh(Err(mc_core::AppError::new(
        ErrorCode::SourceUnavailable,
        "daemon temporarily unavailable",
    )));

    assert_eq!(state.instances.len(), 1);
    assert_eq!(state.instances[0].name, "生存服");
    assert_eq!(state.selected_instance, 0);
    assert!(state.status.starts_with("连接失败"));
}

#[test]
fn create_failure_keeps_wizard_for_correction_and_retry() {
    let mut state = AppState::default();
    state.update(Action::OpenCreate);
    state.apply_create_result(Err(mc_core::AppError::new(
        ErrorCode::SourceUnavailable,
        "daemon temporarily unavailable",
    )));

    assert!(state.wizard.is_some());
    assert_eq!(
        state
            .wizard
            .as_ref()
            .and_then(|wizard| wizard.error.as_deref()),
        Some("daemon temporarily unavailable")
    );
}

#[test]
fn successful_create_returns_to_dashboard_and_requests_refresh() {
    let mut state = AppState::default();
    state.update(Action::OpenCreate);
    state.apply_create_result(Ok(TaskSnapshot {
        id: Uuid::new_v4(),
        kind: TaskKind::CreatePaperInstance,
        instance_id: None,
        status: TaskStatus::Queued,
        phase: TaskPhase::Queued,
        message: "Instance creation queued".to_owned(),
        progress: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        finished_at: None,
        result: None,
        error: None,
        last_event_sequence: 0,
    }));

    assert!(state.wizard.is_none());
    assert_eq!(state.tasks.len(), 1);
    assert!(state.take_refresh_request());
    assert!(state.status.contains("后台队列"));
}

#[test]
fn reopening_task_detail_resumes_from_the_last_event_sequence() {
    let task = task_snapshot();
    let task_id = task.id;
    let mut state = AppState {
        focused_panel: super::Panel::Tasks,
        tasks: vec![task.clone()],
        ..AppState::default()
    };
    state.update(Action::OpenTaskDetail);
    assert!(state.task_detail_visible);
    assert_eq!(state.take_task_detail_query().unwrap().after_sequence, 0);

    state.apply_task_detail_result(Ok(TaskDetailUpdate {
        task,
        events: vec![TaskEvent {
            sequence: 1,
            task_id,
            occurred_at: Utc::now(),
            status: TaskStatus::Running,
            phase: TaskPhase::DownloadingArtifact,
            message: "download started".to_owned(),
            progress: Some(TaskProgress {
                completed_steps: 1,
                total_steps: 3,
                transfer: None,
            }),
            error: None,
        }],
    }));
    state.update(Action::BackFromTaskDetail);
    state.update(Action::OpenTaskDetail);

    assert_eq!(state.take_task_detail_query().unwrap().after_sequence, 1);
}

#[test]
fn opening_instance_detail_starts_with_an_incremental_status_query() {
    let instance_id = Uuid::new_v4();
    let mut state = AppState {
        instances: vec![InstanceSummary {
            id: instance_id,
            name: "生存服".to_owned(),
            provider: "Paper".to_owned(),
            game_version: "1.21.11".to_owned(),
            build_id: 17,
            java_path: "/opt/java/bin/java".into(),
            min_memory_mb: 1024,
            max_memory_mb: 4096,
            port: 25565,
            directory: "/tmp/server".into(),
            created_at: Utc::now(),
        }],
        ..AppState::default()
    };
    state.update(Action::OpenInstanceDetail);
    let query = state.take_instance_detail_query().unwrap();

    assert!(state.instance_detail_visible);
    assert_eq!(query.instance_id, instance_id);
    assert_eq!(query.after_offset, 0);
}

#[test]
fn backup_action_queues_a_task_and_surfaces_its_result() {
    let instance_id = Uuid::new_v4();
    let mut state = AppState {
        instances: vec![InstanceSummary {
            id: instance_id,
            name: "生存服".to_owned(),
            provider: "Paper".to_owned(),
            game_version: "1.21.11".to_owned(),
            build_id: 17,
            java_path: "/opt/java/bin/java".into(),
            min_memory_mb: 1024,
            max_memory_mb: 4096,
            port: 25565,
            directory: "/tmp/server".into(),
            created_at: Utc::now(),
        }],
        ..AppState::default()
    };
    state.update(Action::OpenInstanceDetail);
    state.update(Action::CreateBackup);
    assert!(matches!(
        state.take_instance_command(),
        Some(InstanceCommand::Backup(id)) if id == instance_id
    ));

    let task = TaskSnapshot {
        id: Uuid::new_v4(),
        kind: TaskKind::CreateWorldBackup,
        instance_id: Some(instance_id),
        status: TaskStatus::Queued,
        phase: TaskPhase::Queued,
        message: "World backup queued".to_owned(),
        progress: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        finished_at: None,
        result: None,
        error: None,
        last_event_sequence: 0,
    };
    state.apply_instance_command_result(InstanceCommandResult::Task(Ok(Box::new(task.clone()))));

    assert_eq!(state.tasks, vec![task]);
    assert!(state.status.contains("后台队列"));
    assert!(state.take_refresh_request());
}

#[test]
fn opening_backup_manager_queues_a_bounded_first_page_for_the_open_instance() {
    let instance_id = Uuid::new_v4();
    let mut state = AppState {
        instance_detail_id: Some(instance_id),
        instance_detail_visible: true,
        ..AppState::default()
    };
    state.update(Action::OpenBackupManager);

    assert!(state.backup_manager_visible);
    assert!(state.instance_operation_pending);
    assert!(matches!(
        state.take_backup_command(),
        Some(crate::backup_manager::BackupCommand::List {
            instance_id: requested,
            cursor: None,
        }) if requested == instance_id
    ));
}

#[test]
fn visible_backup_task_keeps_animation_active_until_the_terminal_result() {
    let instance_id = Uuid::new_v4();
    let mut manager =
        crate::backup_manager::BackupManagerState::new(instance_id, "生存服".to_owned());
    manager.begin_list(None);
    let mut state = AppState {
        backup_manager: Some(manager),
        backup_manager_visible: true,
        ..AppState::default()
    };
    assert!(state.has_animated_work());

    state.apply_backup_command_result(Ok(backup_task(
        instance_id,
        TaskKind::ListWorldBackups,
        mc_core::TaskResult::WorldBackupsListed {
            instance_id,
            page: mc_core::BackupPage {
                backups: Vec::new(),
                next_cursor: None,
            },
        },
    )));
    assert!(!state.has_animated_work());
}

#[test]
fn visible_running_task_detail_keeps_animation_active_without_dashboard_copy() {
    let mut state = AppState {
        task_detail: Some(crate::task_detail::TaskDetailState::new(task_snapshot())),
        task_detail_visible: true,
        ..AppState::default()
    };

    assert!(state.has_animated_work());

    state.task_detail.as_mut().expect("task detail").task.status = TaskStatus::Succeeded;
    assert!(!state.has_animated_work());
}

#[test]
fn visible_starting_server_keeps_animation_active_after_start_request_returns() {
    use mc_core::{InstanceDetail, ServerSnapshot, ServerState};

    let instance_id = Uuid::new_v4();
    let instance = InstanceSummary {
        id: instance_id,
        name: "生存服".to_owned(),
        provider: "Paper".to_owned(),
        game_version: "1.21.11".to_owned(),
        build_id: 17,
        java_path: "/opt/java/bin/java".into(),
        min_memory_mb: 1024,
        max_memory_mb: 4096,
        port: 25565,
        directory: "/tmp/server".into(),
        created_at: Utc::now(),
    };
    let mut detail = crate::instance_detail::InstanceDetailState::loading(instance_id);
    detail.detail = Some(InstanceDetail {
        instance,
        server: ServerSnapshot {
            instance_id,
            state: ServerState::Starting,
            pid: None,
            port: 25565,
            address: "127.0.0.1:25565".to_owned(),
            started_at: None,
            last_exit_code: None,
            message: "waiting for readiness".to_owned(),
        },
    });
    let mut state = AppState {
        instance_detail_visible: true,
        instance_detail: Some(detail),
        ..AppState::default()
    };

    assert!(state.has_animated_work());

    state
        .instance_detail
        .as_mut()
        .and_then(|detail| detail.detail.as_mut())
        .expect("loaded instance detail")
        .server
        .state = ServerState::Running;
    assert!(!state.has_animated_work());
}

#[test]
fn backup_manager_load_more_sends_the_cursor_and_deduplicates_results() {
    let instance_id = Uuid::new_v4();
    let first_id = Uuid::new_v4();
    let second_id = Uuid::new_v4();
    let cursor = mc_core::BackupCursor {
        created_at: Utc::now(),
        id: first_id,
    };
    let mut manager =
        crate::backup_manager::BackupManagerState::new(instance_id, "生存服".to_owned());
    manager.begin_list(None);
    let mut state = AppState {
        backup_manager: Some(manager),
        backup_manager_visible: true,
        instance_detail_id: Some(instance_id),
        instance_detail_visible: true,
        ..AppState::default()
    };
    state.apply_backup_command_result(Ok(backup_task(
        instance_id,
        TaskKind::ListWorldBackups,
        mc_core::TaskResult::WorldBackupsListed {
            instance_id,
            page: mc_core::BackupPage {
                backups: vec![mc_core::BackupSummary {
                    id: first_id,
                    created_at: Utc::now(),
                    consistency: mc_core::BackupConsistency::ServerStopped,
                    file_count: 12,
                    total_bytes: 2 * 1024 * 1024,
                }],
                next_cursor: Some(cursor.clone()),
            },
        },
    )));

    state.update(Action::LoadMoreBackups);
    assert!(matches!(
        state.take_backup_command(),
        Some(crate::backup_manager::BackupCommand::List {
            instance_id: requested_instance,
            cursor: Some(requested_cursor),
        }) if requested_instance == instance_id && requested_cursor == cursor
    ));

    state.apply_backup_command_result(Ok(backup_task(
        instance_id,
        TaskKind::ListWorldBackups,
        mc_core::TaskResult::WorldBackupsListed {
            instance_id,
            page: mc_core::BackupPage {
                backups: vec![
                    mc_core::BackupSummary {
                        id: first_id,
                        created_at: Utc::now(),
                        consistency: mc_core::BackupConsistency::ServerStopped,
                        file_count: 12,
                        total_bytes: 2 * 1024 * 1024,
                    },
                    mc_core::BackupSummary {
                        id: second_id,
                        created_at: Utc::now(),
                        consistency: mc_core::BackupConsistency::ServerStopped,
                        file_count: 34,
                        total_bytes: 70 * 1024 * 1024,
                    },
                ],
                next_cursor: None,
            },
        },
    )));
    assert_eq!(
        state
            .backup_manager
            .as_ref()
            .map(|manager| manager.backups.len()),
        Some(2)
    );
    assert!(state
        .backup_manager
        .as_ref()
        .is_none_or(|manager| manager.next_cursor.is_none()));
}

#[test]
fn backup_manager_requires_exact_restore_confirmation_and_submits_verified_plan() {
    let instance_id = Uuid::new_v4();
    let backup_id = Uuid::new_v4();
    let mut state = AppState {
        instance_detail_id: Some(instance_id),
        instance_detail_visible: true,
        ..AppState::default()
    };

    state.update(Action::OpenBackupManager);
    assert!(matches!(
        state.take_backup_command(),
        Some(crate::backup_manager::BackupCommand::List {
            instance_id: requested,
            cursor: None,
        }) if requested == instance_id
    ));

    state.apply_backup_command_result(Ok(backup_task(
        instance_id,
        TaskKind::ListWorldBackups,
        mc_core::TaskResult::WorldBackupsListed {
            instance_id,
            page: mc_core::BackupPage {
                backups: vec![mc_core::BackupSummary {
                    id: backup_id,
                    created_at: Utc::now(),
                    consistency: mc_core::BackupConsistency::ServerStopped,
                    file_count: 12,
                    total_bytes: 2 * 1024 * 1024,
                }],
                next_cursor: None,
            },
        },
    )));
    assert_eq!(
        state
            .backup_manager
            .as_ref()
            .map(|manager| manager.backups.len()),
        Some(1)
    );
    assert!(!state.instance_operation_pending);

    state.update(Action::PreviewRestore);
    assert!(matches!(
        state.take_backup_command(),
        Some(crate::backup_manager::BackupCommand::PlanRestore {
            instance_id: requested_instance,
            backup_id: requested_backup,
        }) if requested_instance == instance_id && requested_backup == backup_id
    ));

    let token = format!("v1:{}", "a".repeat(64));
    state.apply_backup_command_result(Ok(backup_task(
        instance_id,
        TaskKind::PlanWorldBackupRestore,
        mc_core::TaskResult::WorldBackupRestorePlanned {
            instance_id,
            plan: mc_core::BackupRestorePlan {
                instance_id,
                backup_id,
                created_at: Utc::now(),
                current_worlds: vec!["world".to_owned()],
                target_worlds: vec!["world".to_owned(), "world_the_end".to_owned()],
                replaced_worlds: vec!["world".to_owned()],
                added_worlds: vec!["world_the_end".to_owned()],
                removed_worlds: Vec::new(),
                required_space_bytes: 2 * 1024 * 1024,
                available_space_bytes: 10 * 1024 * 1024 * 1024,
                confirmation_token: token.clone(),
            },
        },
    )));
    assert!(state
        .backup_manager
        .as_ref()
        .is_some_and(|manager| manager.is_reviewing_plan()));

    state.update(Action::BeginRestoreConfirmation);
    for character in "RESTOR".chars() {
        state.update(Action::RestoreConfirmInsert(character));
    }
    state.update(Action::RestoreConfirmSubmit);
    assert!(state.take_backup_command().is_none());
    assert!(state
        .backup_manager
        .as_ref()
        .and_then(|manager| manager.error.as_deref())
        .is_some_and(|error| error.contains("RESTORE")));

    state.update(Action::RestoreConfirmInsert('E'));
    state.update(Action::RestoreConfirmSubmit);
    assert!(matches!(
        state.take_backup_command(),
        Some(crate::backup_manager::BackupCommand::Restore {
            instance_id: requested_instance,
            backup_id: requested_backup,
            confirmation_token: requested_token,
        }) if requested_instance == instance_id
            && requested_backup == backup_id
            && requested_token == token
    ));
}

#[test]
fn opening_create_loads_versions_and_java_before_editing() {
    let mut state = AppState::default();
    state.update(Action::OpenCreate);
    assert!(state.take_create_options_request());
    assert!(state
        .wizard
        .as_ref()
        .is_some_and(|wizard| wizard.options_loading));
    state.apply_create_options(Ok(crate::wizard::CreateOptions {
        versions: vec![mc_core::ServerVersion {
            id: "1.21.11".to_owned(),
            group: "1.21".to_owned(),
        }],
        java_runtimes: vec![mc_core::JavaRuntime {
            executable: "/opt/java21/bin/java".to_owned(),
            version: "21.0.2".to_owned(),
            major_version: Some(21),
            architecture: Some("aarch64".to_owned()),
            vendor: Some("Eclipse Adoptium".to_owned()),
        }],
        suggested_port: 25565,
    }));
    let wizard = state.wizard.as_ref().expect("wizard remains open");
    assert_eq!(wizard.versions.len(), 1);
    assert_eq!(wizard.java_runtimes.len(), 1);
    assert_eq!(wizard.port, "25565");
    assert!(!wizard.options_loading);
}

#[test]
fn command_palette_can_start_the_guided_create_flow() {
    let mut state = AppState::default();
    state.update(Action::OpenCommandPalette);
    assert!(state.command_palette.is_some());
    for character in "新建".chars() {
        state.update(Action::PaletteInsert(character));
    }
    state.update(Action::PaletteSubmit);
    assert!(state.command_palette.is_none());
    assert!(state.wizard.is_some());
    assert!(state.take_create_options_request());
}

fn backup_task(instance_id: Uuid, kind: TaskKind, result: mc_core::TaskResult) -> TaskSnapshot {
    TaskSnapshot {
        id: Uuid::new_v4(),
        kind,
        instance_id: Some(instance_id),
        status: TaskStatus::Succeeded,
        phase: TaskPhase::Completed,
        message: "backup task completed".to_owned(),
        progress: Some(TaskProgress {
            completed_steps: 1,
            total_steps: 1,
            transfer: None,
        }),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        finished_at: Some(Utc::now()),
        result: Some(result),
        error: None,
        last_event_sequence: 2,
    }
}

fn task_snapshot() -> TaskSnapshot {
    TaskSnapshot {
        id: Uuid::new_v4(),
        kind: TaskKind::CreatePaperInstance,
        instance_id: None,
        status: TaskStatus::Running,
        phase: TaskPhase::ResolvingArtifact,
        message: "Resolving Paper build".to_owned(),
        progress: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        finished_at: None,
        result: None,
        error: None,
        last_event_sequence: 0,
    }
}

#[test]
fn modpack_import_scan_selects_a_pack_and_tracks_analysis_result() {
    let mut state = AppState::default();
    state.update(Action::OpenModpackImport);
    assert!(state.modpack_import_visible);
    let import = state.modpack_import.as_mut().expect("import state");
    import.candidates = vec![std::path::PathBuf::from("/tmp/Example.mrpack")];
    import.error = None;
    state.update(Action::AnalyzeModpack);
    assert_eq!(
        state.take_modpack_analysis_request(),
        Some(std::path::PathBuf::from("/tmp/Example.mrpack")),
    );
    state.apply_modpack_analysis_result(Ok(mc_core::ModpackAnalysisReport {
        path: std::path::PathBuf::from("/tmp/Example.mrpack"),
        name: "Example Pack".to_owned(),
        version_id: "1.0.0".to_owned(),
        minecraft_version: Some("1.21.1".to_owned()),
        loader: Some(mc_core::ModpackLoader {
            dependency: "fabric-loader".to_owned(),
            version: "0.16.9".to_owned(),
        }),
        status: mc_core::ModpackStatus::Ready,
        install_files: 2,
        review_files: 0,
        excluded_files: 1,
        mod_files: 1,
        total_install_bytes: 1024,
        server_override_files: 0,
        issues: Vec::new(),
        files: Vec::new(),
        files_truncated: false,
    }));
    assert!(state
        .modpack_import
        .as_ref()
        .and_then(|import| import.report.as_ref())
        .is_some_and(|report| report.name == "Example Pack"));
}

#[test]
fn shell_use_selects_an_instance_and_requests_its_status() {
    let instance_id = Uuid::new_v4();
    let mut state = AppState {
        instances: vec![InstanceSummary {
            id: instance_id,
            name: "生存服".to_owned(),
            provider: "Paper".to_owned(),
            game_version: "1.21.11".to_owned(),
            build_id: 17,
            java_path: "/opt/java/bin/java".into(),
            min_memory_mb: 1024,
            max_memory_mb: 2048,
            port: 25565,
            directory: "/tmp/server".into(),
            created_at: Utc::now(),
        }],
        ..AppState::default()
    };
    state.shell_input = "/use 1".to_owned();
    state.update(Action::ShellSubmit);

    assert_eq!(state.selected_instance, 0);
    assert!(state
        .transcript
        .iter()
        .any(|entry| entry.text.contains("已选择 #1")));
    assert_eq!(
        state
            .take_instance_detail_query()
            .map(|query| query.instance_id),
        Some(instance_id)
    );
}

#[test]
fn shell_status_and_tasks_render_structured_output() {
    let instance_id = Uuid::new_v4();
    let mut state = AppState {
        instances: vec![InstanceSummary {
            id: instance_id,
            name: "生存服".to_owned(),
            provider: "Paper".to_owned(),
            game_version: "1.21.11".to_owned(),
            build_id: 17,
            java_path: "/opt/java/bin/java".into(),
            min_memory_mb: 1024,
            max_memory_mb: 2048,
            port: 25565,
            directory: "/tmp/server".into(),
            created_at: Utc::now(),
        }],
        tasks: vec![task_snapshot()],
        ..AppState::default()
    };

    state.shell_input = "/status".to_owned();
    state.update(Action::ShellSubmit);
    assert!(state
        .transcript
        .iter()
        .any(|entry| entry.text.contains("当前实例：生存服")));
    assert!(state
        .transcript
        .iter()
        .any(|entry| entry.text.contains("后台任务：1 个")));

    state.shell_input = "/tasks".to_owned();
    state.update(Action::ShellSubmit);
    assert!(state
        .transcript
        .iter()
        .any(|entry| entry.text.contains("后台任务 · 1")));
}

#[test]
fn shell_free_text_queues_a_console_command() {
    let instance_id = Uuid::new_v4();
    let mut state = AppState {
        instances: vec![InstanceSummary {
            id: instance_id,
            name: "生存服".to_owned(),
            provider: "Paper".to_owned(),
            game_version: "1.21.11".to_owned(),
            build_id: 17,
            java_path: "/opt/java/bin/java".into(),
            min_memory_mb: 1024,
            max_memory_mb: 2048,
            port: 25565,
            directory: "/tmp/server".into(),
            created_at: Utc::now(),
        }],
        ..AppState::default()
    };
    state.shell_input = "say hello".to_owned();
    state.update(Action::ShellSubmit);

    assert!(state
        .transcript
        .iter()
        .any(|entry| entry.text.contains("发送到「生存服」控制台")));
    match state.take_instance_command() {
        Some(InstanceCommand::Console {
            instance_id: id,
            command,
        }) => {
            assert_eq!(id, instance_id);
            assert_eq!(command, "say hello");
        }
        other => panic!("expected console command, got {other:?}"),
    }
    assert!(state.instance_operation_pending);
}

#[test]
fn shell_restart_queues_the_controlled_restart_command() {
    let instance_id = Uuid::new_v4();
    let mut state = AppState {
        instances: vec![InstanceSummary {
            id: instance_id,
            name: "生存服".to_owned(),
            provider: "Paper".to_owned(),
            game_version: "1.21.11".to_owned(),
            build_id: 17,
            java_path: "/opt/java/bin/java".into(),
            min_memory_mb: 1024,
            max_memory_mb: 2048,
            port: 25565,
            directory: "/tmp/server".into(),
            created_at: Utc::now(),
        }],
        ..AppState::default()
    };
    state.shell_input = "/restart".to_owned();
    state.update(Action::ShellSubmit);

    assert!(state
        .transcript
        .iter()
        .any(|entry| entry.text.contains("先优雅停止")));
    assert!(matches!(
        state.take_instance_command(),
        Some(InstanceCommand::Restart(id)) if id == instance_id
    ));
    assert!(state.instance_operation_pending);
    assert!(state.status.contains("自动重新启动"));
}

#[test]
fn shell_history_completion_and_unknown_commands_are_actionable() {
    let mut state = AppState {
        shell_history: vec!["/status".to_owned()],
        ..AppState::default()
    };
    state.update(Action::ShellUp);
    assert_eq!(state.shell_input, "/status");
    state.update(Action::ShellEscape);
    assert!(state.shell_input.is_empty());

    state.shell_input = "/sta".to_owned();
    state.update(Action::ShellComplete);
    assert_eq!(state.shell_input, "/status ");

    state.shell_input = "/nope".to_owned();
    state.update(Action::ShellSubmit);
    assert!(state
        .transcript
        .iter()
        .any(|entry| entry.text.contains("未知命令")));
}

#[test]
fn shell_theme_command_lists_all_themes() {
    let mut state = AppState {
        shell_input: "/theme".to_owned(),
        ..AppState::default()
    };
    state.update(Action::ShellSubmit);
    let text = state
        .transcript
        .iter()
        .map(|entry| entry.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    for name in [
        "Claude", "DeepSeek", "Codex", "Kimi", "PCL2", "Mono", "Ocean",
    ] {
        assert!(text.contains(name), "missing theme {name}");
    }
}

#[test]
fn shell_logs_command_tails_console_output_inline() {
    let instance_id = Uuid::new_v4();
    let mut detail = crate::instance_detail::InstanceDetailState::loading(instance_id);
    detail.logs = "Starting minecraft server\nDone (2.4s)!\nPlayer Steve joined\n".to_owned();
    let mut state = AppState {
        instances: vec![InstanceSummary {
            id: instance_id,
            name: "生存服".to_owned(),
            provider: "Paper".to_owned(),
            game_version: "1.21.11".to_owned(),
            build_id: 17,
            java_path: "/opt/java/bin/java".into(),
            min_memory_mb: 1024,
            max_memory_mb: 2048,
            port: 25565,
            directory: "/tmp/server".into(),
            created_at: Utc::now(),
        }],
        instance_detail: Some(detail),
        instance_detail_id: Some(instance_id),
        shell_input: "/logs".to_owned(),
        ..AppState::default()
    };
    state.update(Action::ShellSubmit);

    let text = state
        .transcript
        .iter()
        .map(|entry| entry.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("最近控制台日志"));
    assert!(text.contains("Player Steve joined"));
    assert!(!state.instance_detail_visible);
}

#[test]
fn shell_cancel_command_queues_cancellation_without_opening_task_detail() {
    let mut state = AppState::default();
    let mut task = task_snapshot();
    task.phase = TaskPhase::DownloadingArtifact;
    let task_id = task.id;
    state.tasks = vec![task];
    state.shell_input = "/cancel 1".to_owned();
    state.update(Action::ShellSubmit);

    assert_eq!(state.take_task_cancel_request(), Some(task_id));
    assert!(state
        .transcript
        .iter()
        .any(|entry| entry.text.contains("已请求取消任务 #1")));
}

#[test]
fn shell_cursor_edits_mid_line_and_home_end() {
    let mut state = AppState::default();
    for character in "abcd".chars() {
        state.shell_insert(character);
    }
    state.shell_cursor_left();
    state.shell_cursor_left();
    state.shell_insert('X');
    assert_eq!(state.shell_input, "abXcd");
    assert_eq!(state.shell_cursor, 3);

    state.shell_delete();
    assert_eq!(state.shell_input, "abXd");
    state.shell_backspace();
    assert_eq!(state.shell_input, "abd");
    state.shell_cursor_home();
    state.shell_insert('Z');
    assert_eq!(state.shell_input, "Zabd");
    state.shell_cursor_end();
    state.shell_backspace();
    assert_eq!(state.shell_input, "Zab");
}

#[test]
fn multiline_paste_stays_in_one_inspectable_shell_input() {
    let mut state = AppState::default();
    state.paste_text("say hello\r\n/stop\nsecond line");

    assert_eq!(state.shell_input, "say hello /stop second line");
    assert!(state
        .transcript
        .iter()
        .all(|entry| entry.role != crate::transcript::TranscriptRole::User));
    assert_eq!(
        state.status,
        "多行粘贴已合并为单行输入，请检查当前目标和内容后再提交。"
    );
}

#[test]
fn paste_text_is_applied_to_the_active_command_palette_or_wizard_field() {
    let mut palette_state = AppState::default();
    palette_state.update(Action::OpenCommandPalette);
    palette_state.paste_text("jk");
    assert_eq!(
        palette_state
            .command_palette
            .as_ref()
            .map(|palette| palette.query.as_str()),
        Some("jk")
    );

    let mut wizard_state = AppState::default();
    wizard_state.update(Action::OpenCreate);
    wizard_state.paste_text("Friend\nServer");
    assert_eq!(
        wizard_state
            .wizard
            .as_ref()
            .map(|wizard| wizard.name.as_str()),
        Some("Friend Server")
    );
}

#[test]
fn shell_enter_completes_a_partial_command_before_dispatching_it() {
    let mut state = AppState::default();
    state.shell_input = "/sto".to_owned();
    state.shell_cursor = state.shell_input.len();
    state.update(Action::ShellSubmit);
    assert_eq!(state.shell_input, "/stop");
    assert!(state
        .transcript
        .iter()
        .all(|entry| entry.role != crate::transcript::TranscriptRole::User));
    assert!(state.take_instance_command().is_none());

    state.update(Action::ShellSubmit);
    let user = state
        .transcript
        .iter()
        .rev()
        .find(|entry| entry.role == crate::transcript::TranscriptRole::User)
        .expect("user command entry");
    assert_eq!(user.text, "/stop");

    let mut exact = AppState::default();
    exact.shell_input = "/stop".to_owned();
    exact.shell_cursor = exact.shell_input.len();
    exact.shell_suggestion_index = 0;
    exact.update(Action::ShellSubmit);
    let user = exact
        .transcript
        .iter()
        .rev()
        .find(|entry| entry.role == crate::transcript::TranscriptRole::User)
        .expect("user command entry");
    assert_eq!(user.text, "/stop");
}

#[test]
fn shell_help_reveals_the_start_of_the_command_list() {
    let mut state = AppState::default();
    state.update(Action::ToggleHelp);
    state.update(Action::CloseHelp);
    state.shell_viewport.set(10);
    state.shell_input = "/help".to_owned();
    state.shell_cursor = state.shell_input.len();
    state.update(Action::ShellSubmit);

    assert!(state.shell_scroll > 0);
    assert!(state
        .transcript
        .iter()
        .any(|entry| entry.text.contains("可用命令") && entry.text.contains("/status")));
}

#[test]
fn shell_minecraft_slash_commands_get_a_console_hint() {
    let mut state = AppState::default();
    state.shell_input = "/say hello".to_owned();
    state.shell_cursor = state.shell_input.len();
    state.update(Action::ShellSubmit);
    let text = state
        .transcript
        .iter()
        .map(|entry| entry.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("未知命令 /say"));
    assert!(text.contains("Minecraft 控制台命令不加前导 /"));
}

#[test]
fn server_readiness_is_announced_once_with_join_guidance() {
    let instance = InstanceSummary {
        id: Uuid::new_v4(),
        name: "朋友服".to_owned(),
        provider: "paper".to_owned(),
        game_version: "1.21.11".to_owned(),
        build_id: 1,
        java_path: "/usr/bin/java".into(),
        min_memory_mb: 1024,
        max_memory_mb: 2048,
        port: 25601,
        directory: "/tmp/friends".into(),
        created_at: Utc::now(),
    };
    let server = |state| mc_core::ServerSnapshot {
        instance_id: instance.id,
        state,
        pid: None,
        port: 25601,
        address: "127.0.0.1:25601".to_owned(),
        started_at: None,
        last_exit_code: None,
        message: String::new(),
    };
    let data = |state| DashboardData {
        servers: vec![server(state)],
        instances: vec![instance.clone()],
        tasks: Vec::new(),
    };
    let mut state = AppState::default();
    let announced = |state: &AppState| {
        state
            .transcript
            .iter()
            .filter(|entry| entry.text.contains("已就绪"))
            .count()
    };
    // Already running when the TUI attaches: nothing to announce.
    state.apply_refresh(Ok(data(mc_core::ServerState::Running)));
    assert_eq!(announced(&state), 0);
    state.apply_refresh(Ok(data(mc_core::ServerState::Starting)));
    state.apply_refresh(Ok(data(mc_core::ServerState::Running)));
    state.apply_refresh(Ok(data(mc_core::ServerState::Running)));
    assert_eq!(announced(&state), 1);
    assert!(state
        .transcript
        .iter()
        .any(|entry| entry.text.contains("/connect") && entry.text.contains("127.0.0.1:25601")));
}

#[test]
fn console_draft_cannot_follow_a_changed_instance() {
    let mut state = AppState::default();
    let first = InstanceSummary {
        id: Uuid::new_v4(),
        name: "原服务器".into(),
        provider: "paper".into(),
        game_version: "1.21.11".into(),
        build_id: 17,
        java_path: "/usr/bin/java".into(),
        min_memory_mb: 1024,
        max_memory_mb: 2048,
        port: 25565,
        directory: "/tmp/server".into(),
        created_at: Utc::now(),
    };
    let mut second = first.clone();
    second.id = Uuid::new_v4();
    second.name = "另一台服务器".into();
    state.instances = vec![first, second];
    for command in ["say hello", "/console say hello", "/cmd say hello"] {
        state.selected_instance = 0;
        state.shell_escape();
        for ch in command.chars() {
            state.shell_insert(ch);
        }
        state.selected_instance = 1;
        state.shell_submit();
        assert!(state.take_instance_command().is_none());
        assert_eq!(state.shell_input, command);
        assert!(state.status.contains("之前的服务器"));
    }
    state.shell_escape();
    assert!(state.shell_draft_target.is_none());
}

#[test]
fn welcome_actions_open_real_flows_without_submitting_fabric() {
    use crate::hit::HitTarget;
    let mut state = AppState::default();
    assert!(state.should_show_welcome());
    state.apply_click(HitTarget::WelcomeCreatePaper);
    assert!(state.wizard.is_some());
    assert!(state.take_create_options_request());
    state.update(Action::CancelWizard);
    state.apply_click(HitTarget::WelcomeCreateFabric);
    assert!(state.shell_visible);
    assert_eq!(state.shell_input, "/fabric ");
    assert!(!state.should_show_welcome());
    assert!(state.take_instance_command().is_none());
    state.update(Action::ShellEscape);
    state.apply_click(HitTarget::WelcomeImportModpack);
    assert!(state.modpack_import_visible);
}

#[test]
fn operation_error_blocks_paste_and_clicks_until_dismissed() {
    use crate::hit::HitTarget;
    let mut state = AppState {
        shell_input: "say draft".to_owned(),
        ..AppState::default()
    };
    state.apply_instance_command_result(InstanceCommandResult::Management(Err(
        mc_core::AppError::new(ErrorCode::BackupFailed, "World is running")
            .with_evidence("instance=example")
            .with_action("Stop the server first"),
    )));
    let dialog = state
        .error_dialog
        .as_ref()
        .expect("operation failure dialog");
    assert_eq!(dialog.message.reason, "World is running");
    assert_eq!(dialog.message.actions[0], "Stop the server first");
    assert!(dialog
        .message
        .diagnostic_report()
        .contains("instance=example"));
    state.paste_text("unsafe pasted command");
    state.apply_click(HitTarget::WelcomeCreatePaper);
    assert_eq!(state.shell_input, "say draft");
    assert!(state.wizard.is_none());
    state.update(Action::ToggleErrorDetails);
    assert!(state.error_dialog.as_ref().expect("dialog").show_details);
    state.update(Action::DismissError);
    assert!(state.error_dialog.is_none());
    assert_eq!(state.shell_input, "say draft");
}
