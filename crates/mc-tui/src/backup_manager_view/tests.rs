use chrono::Utc;
use mc_core::{
    BackupPage, BackupRestorePlan, BackupSummary, TaskKind, TaskPhase, TaskProgress, TaskResult,
    TaskSnapshot, TaskStatus,
};
use ratatui::{backend::TestBackend, Terminal};

use crate::hit::HitMap;
use uuid::Uuid;

use super::render;
use crate::backup_manager::BackupManagerState;

#[test]
fn renders_backup_catalog_and_restore_actions_at_reference_and_compact_sizes() {
    let state = loaded_state();
    let standard = render_to_text(&state, 80, 24, 0);
    assert!(standard.contains("世界与备份"));
    assert!(standard.contains("备份快照"));
    assert!(standard.contains("12个文件"));
    assert!(standard.contains("2.0MiB"));
    assert!(standard.contains("Enter恢复预览"));
    assert!(standard.contains("Esc返回"));

    let compact = render_to_text(&state, 40, 12, 0);
    assert!(compact.contains("世界与备份"));
    assert!(compact.contains("备份快照"));
    assert!(compact.contains("Enter恢复"));
}

#[test]
fn renders_backup_task_loading_at_both_sizes() {
    let mut state = BackupManagerState::new(Uuid::new_v4(), "生存服".to_owned());
    state.begin_list(None);
    let standard = render_to_text(&state, 80, 24, 1);
    assert!(standard.contains("读取备份目录"));
    let compact = render_to_text(&state, 40, 12, 1);
    assert!(compact.contains("读取备份目录"));
}

#[test]
fn renders_restore_impact_and_explicit_confirmation_prompt() {
    let mut state = loaded_state();
    let plan = restore_plan(state.instance_id, state.backups[0].id);
    state.plan = Some(plan);
    state.editing_confirmation = true;
    state.confirmation_input = "RESTORE".to_owned();

    let text = render_to_text(&state, 80, 24, 0);
    assert!(text.contains("恢复影响预览"));
    assert!(text.contains("替换"));
    assert!(text.contains("world_nether"));
    assert!(text.contains("新增"));
    assert!(text.contains("world_the_end"));
    assert!(text.contains("RESTORE"));
    assert!(text.contains("失败自动回滚"));
}

#[test]
fn renders_running_and_completed_restore_states() {
    let mut state = loaded_state();
    state.plan = Some(restore_plan(state.instance_id, state.backups[0].id));
    state.begin_restore();
    let running = render_to_text(&state, 80, 24, 1);
    assert!(running.contains("正在执行恢复事务"));
    assert!(running.contains("持久任务会在后台继续执行"));

    let backup_id = state.backups[0].id;
    state.track_task(&TaskSnapshot {
        id: Uuid::new_v4(),
        kind: TaskKind::RestoreWorldBackup,
        instance_id: Some(state.instance_id),
        status: TaskStatus::Succeeded,
        phase: TaskPhase::Completed,
        message: "World restore committed".to_owned(),
        progress: Some(TaskProgress {
            completed_steps: 1,
            total_steps: 1,
            transfer: None,
        }),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        finished_at: Some(Utc::now()),
        result: Some(TaskResult::WorldBackupRestored {
            instance_id: state.instance_id,
            backup_id,
        }),
        error: None,
        last_event_sequence: 2,
    });
    let complete = render_to_text(&state, 40, 12, 0);
    assert!(complete.contains("世界恢复完成"), "{complete}");
    assert!(complete.contains("Esc返回"));
}

fn loaded_state() -> BackupManagerState {
    let instance_id = Uuid::new_v4();
    let backup_id = Uuid::new_v4();
    let mut state = BackupManagerState::new(instance_id, "视觉验收服".to_owned());
    state.begin_list(None);
    state.track_task(&TaskSnapshot {
        id: Uuid::new_v4(),
        kind: TaskKind::ListWorldBackups,
        instance_id: Some(instance_id),
        status: TaskStatus::Succeeded,
        phase: TaskPhase::Completed,
        message: "Backup page listed".to_owned(),
        progress: Some(TaskProgress {
            completed_steps: 1,
            total_steps: 1,
            transfer: None,
        }),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        finished_at: Some(Utc::now()),
        result: Some(TaskResult::WorldBackupsListed {
            instance_id,
            page: BackupPage {
                backups: vec![BackupSummary {
                    id: backup_id,
                    created_at: Utc::now(),
                    consistency: mc_core::BackupConsistency::ServerStopped,
                    file_count: 12,
                    total_bytes: 2 * 1024 * 1024,
                }],
                next_cursor: None,
            },
        }),
        error: None,
        last_event_sequence: 2,
    });
    state
}

fn restore_plan(instance_id: Uuid, backup_id: Uuid) -> BackupRestorePlan {
    BackupRestorePlan {
        instance_id,
        backup_id,
        created_at: Utc::now(),
        current_worlds: vec!["world".to_owned(), "world_nether".to_owned()],
        target_worlds: vec!["world".to_owned(), "world_the_end".to_owned()],
        replaced_worlds: vec!["world".to_owned()],
        added_worlds: vec!["world_the_end".to_owned()],
        removed_worlds: vec!["world_nether".to_owned()],
        required_space_bytes: 2 * 1024 * 1024,
        available_space_bytes: 10 * 1024 * 1024 * 1024,
        confirmation_token: format!("v1:{}", "a".repeat(64)),
    }
}

fn render_to_text(
    state: &BackupManagerState,
    width: u16,
    height: u16,
    animation_frame: u8,
) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    terminal
        .draw(|frame| render(frame, state, animation_frame, &mut HitMap::default()))
        .expect("render backup manager");
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .filter(|symbol| !symbol.chars().all(char::is_whitespace))
        .collect()
}
