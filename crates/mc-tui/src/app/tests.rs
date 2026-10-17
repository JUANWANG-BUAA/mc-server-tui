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
fn list_backups_action_queues_a_task_for_the_open_instance() {
    let instance_id = Uuid::new_v4();
    let mut state = AppState {
        instance_detail_id: Some(instance_id),
        instance_detail_visible: true,
        ..AppState::default()
    };
    state.update(Action::ListBackups);
    assert!(matches!(
        state.take_instance_command(),
        Some(InstanceCommand::ListBackups(id)) if id == instance_id
    ));

    state.apply_instance_command_result(InstanceCommandResult::Task(Ok(Box::new(TaskSnapshot {
        id: Uuid::new_v4(),
        kind: TaskKind::ListWorldBackups,
        instance_id: Some(instance_id),
        status: TaskStatus::Queued,
        phase: TaskPhase::Queued,
        message: "Backup list queued".to_owned(),
        progress: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        finished_at: None,
        result: None,
        error: None,
        last_event_sequence: 0,
    }))));
    assert_eq!(state.tasks[0].kind, TaskKind::ListWorldBackups);
    assert!(state.status.contains("备份列表任务"));
    assert!(state.take_refresh_request());
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
