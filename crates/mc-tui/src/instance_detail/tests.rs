use chrono::Utc;
use mc_core::{InstanceSummary, ServerLogChunk, ServerSnapshot, ServerState};

use super::{InstanceDetailState, InstanceDetailUpdate};

#[test]
fn log_cursor_appends_incrementally_and_replaces_after_truncation() {
    let instance_id = uuid::Uuid::new_v4();
    let mut state = InstanceDetailState::loading(instance_id);
    state.apply_update(Ok(update(instance_id, 0, 3, "one\n", false)));
    assert_eq!(state.log_offset, 3);
    state.apply_update(Ok(update(instance_id, 3, 7, "two\n", false)));
    assert_eq!(state.logs, "one\ntwo\n");
    state.apply_update(Ok(update(instance_id, 10, 13, "new\n", true)));
    assert_eq!(state.logs, "new\n");
    assert_eq!(state.log_offset, 13);
}

#[test]
fn console_input_rejects_controls_and_caps_length() {
    let mut state = InstanceDetailState::loading(uuid::Uuid::new_v4());
    state.begin_console_input();
    state.insert_console_char('a');
    state.insert_console_char('\n');
    for _ in 0..600 {
        state.insert_console_char('x');
    }
    assert_eq!(state.console_input.len(), 512);
    assert_eq!(state.take_console_command().unwrap().len(), 512);
    assert!(!state.editing_console);
}

fn update(
    instance_id: uuid::Uuid,
    start_offset: u64,
    next_offset: u64,
    content: &str,
    truncated: bool,
) -> InstanceDetailUpdate {
    let instance = InstanceSummary {
        id: instance_id,
        name: "test".to_owned(),
        provider: "Paper".to_owned(),
        game_version: "1.21.11".to_owned(),
        build_id: 1,
        java_path: "/usr/bin/java".into(),
        min_memory_mb: 512,
        max_memory_mb: 1024,
        port: 25565,
        directory: "/tmp/test".into(),
        created_at: Utc::now(),
    };
    InstanceDetailUpdate {
        stats: None,
        detail: mc_core::InstanceDetail {
            instance,
            server: ServerSnapshot {
                instance_id,
                state: ServerState::Running,
                pid: Some(123),
                port: 25565,
                address: "127.0.0.1:25565".to_owned(),
                started_at: Some(Utc::now()),
                last_exit_code: None,
                message: "Ready".to_owned(),
            },
        },
        logs: ServerLogChunk {
            instance_id,
            start_offset,
            next_offset,
            content: content.to_owned(),
            truncated,
        },
    }
}

#[test]
fn scrolling_clamps_and_new_lines_preserve_reading_position() {
    let id = uuid::Uuid::new_v4();
    let mut state = InstanceDetailState::loading(id);
    state.apply_update(Ok(update(id, 0, 4, "a\nb\n", false)));
    state.log_max_scroll.set(10);
    state.scroll_to_oldest();
    assert_eq!(state.log_scroll, 10);
    state.scroll_logs(-1);
    assert_eq!(state.log_scroll, 9);
    state.apply_update(Ok(update(id, 4, 6, "c\n", false)));
    assert_eq!(state.log_scroll, 10);
}
#[test]
fn alerts_require_sustained_cpu_and_surface_new_log_errors() {
    let id = uuid::Uuid::new_v4();
    let mut state = InstanceDetailState::loading(id);
    for i in 0..6 {
        let mut data = update(id, i, i + 1, "[ERROR] failed\n", false);
        data.stats = Some(super::ProcessStats {
            cpu_percent: 95.0,
            rss_bytes: 32,
        });
        state.apply_update(Ok(data));
    }
    assert!(state.alerts().iter().any(|a| a.contains("CPU")));
    assert!(state.alerts().iter().any(|a| a.contains("ERROR")));
}
