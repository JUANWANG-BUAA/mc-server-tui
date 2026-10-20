use chrono::Utc;
use mc_core::{InstanceSummary, TaskKind, TaskPhase, TaskProgress, TaskSnapshot, TaskStatus};
use ratatui::{backend::TestBackend, style::Color, Terminal};
use uuid::Uuid;

use crate::app::AppState;
use crate::hit::HitMap;

use super::render;

#[test]
fn renders_shell_transcript_composer_and_status_dock() {
    let state = populated_state();
    let text = render_to_text(80, 24, &state, 0);
    assert!(text.contains("连接中"));
    assert!(text.contains("控制台→视觉验收服"));
    assert!(text.contains("视觉验收服"));
    assert!(text.contains("下载校验"));
    assert!(text.contains("F1帮助"));
    assert!(text.contains("/refresh"));
    assert_eq!(text.matches("下载校验").count(), 1);

    let compact = render_to_text(40, 12, &state, 0);
    assert!(compact.contains("下载校验"));
    assert!(compact.contains("控制台"));
}

#[test]
fn mascot_is_reserved_for_the_empty_welcome_screen() {
    let state = AppState::default();
    let first = render_to_text(80, 24, &state, 2);
    assert!(first.contains("▀"));
    assert!(!render_to_text(80, 24, &populated_state(), 2).contains("▀"));

    let styles = render_to_styles(80, 24, &state, 2);
    for expected in [
        Color::Rgb(54, 37, 25),    // hair
        Color::Rgb(181, 136, 108), // skin
        Color::Rgb(75, 65, 145),   // pupil
        Color::Rgb(67, 43, 29),    // mouth
    ] {
        assert!(
            styles
                .iter()
                .any(|(symbol, fg, bg)| symbol == "▀" && (*fg == expected || *bg == expected)),
            "missing Steve pixel color {expected:?}"
        );
    }
}

#[test]
fn long_composer_input_scrolls_to_keep_the_caret_visible() {
    let mut state = populated_state();
    state.shell_input = "x".repeat(120);
    state.shell_cursor = state.shell_input.len();
    let text = render_to_text(80, 24, &state, 0);
    assert!(text.contains(&"x".repeat(40)));
}

#[test]
fn slash_input_opens_command_completion() {
    let mut state = populated_state();
    state.shell_input = "/sta".to_owned();
    let text = render_to_text(80, 24, &state, 0);
    assert!(text.contains("命令补全"));
    assert!(text.contains("/refresh"));
    assert!(text.contains("Tab填入"));
    assert!(text.contains("先补全；再次Enter执行"));
}

fn populated_state() -> AppState {
    let mut state = AppState::default();
    state.instances = vec![InstanceSummary {
        id: Uuid::new_v4(),
        name: "视觉验收服".to_owned(),
        provider: "Paper".to_owned(),
        game_version: "1.21.11".to_owned(),
        build_id: 17,
        java_path: "/opt/java/bin/java".into(),
        min_memory_mb: 1024,
        max_memory_mb: 2048,
        port: 25565,
        directory: "/tmp/server".into(),
        created_at: Utc::now(),
    }];
    state.tasks = vec![TaskSnapshot {
        id: Uuid::new_v4(),
        kind: TaskKind::CreatePaperInstance,
        instance_id: None,
        status: TaskStatus::Running,
        phase: TaskPhase::DownloadingArtifact,
        message: "正在下载并校验 Paper".to_owned(),
        progress: Some(TaskProgress {
            completed_steps: 1,
            total_steps: 3,
            transfer: None,
        }),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        finished_at: None,
        result: None,
        error: None,
        last_event_sequence: 2,
    }];
    state.status = "已连接 · 自动刷新中".to_owned();
    state
}

fn render_to_text(width: u16, height: u16, state: &AppState, animation_frame: u8) -> String {
    render_to_styles(width, height, state, animation_frame)
        .into_iter()
        .map(|(symbol, _, _)| symbol)
        .filter(|symbol| !symbol.chars().all(char::is_whitespace))
        .collect()
}

fn render_to_styles(
    width: u16,
    height: u16,
    state: &AppState,
    animation_frame: u8,
) -> Vec<(String, Color, Color)> {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    terminal
        .draw(|frame| {
            render(
                frame,
                frame.area(),
                state,
                animation_frame,
                &mut HitMap::default(),
            )
        })
        .expect("render shell");
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| (cell.symbol().to_owned(), cell.fg, cell.bg))
        .collect()
}

#[test]
fn header_uses_selected_instance_even_when_detail_belongs_to_another() {
    let mut state = populated_state();
    let id = state.instances[0].id;
    state.servers.insert(
        id,
        mc_core::ServerSnapshot {
            instance_id: id,
            state: mc_core::ServerState::Installed,
            pid: None,
            port: 25565,
            address: "127.0.0.1:25565".into(),
            started_at: None,
            last_exit_code: None,
            message: "Installed".into(),
        },
    );
    state.instance_detail = Some(crate::instance_detail::InstanceDetailState::loading(
        Uuid::new_v4(),
    ));
    let text = render_to_text(80, 24, &state, 0);
    assert!(text.contains("/start启动"));
    assert!(text.find("视觉验收服").expect("instance") < text.find("下载校验").expect("task"));
    state.connected = Some(false);
    assert!(render_to_text(80, 24, &state, 0).contains("显示上次状态"));
}

#[test]
fn composer_distinguishes_management_from_console_target() {
    let mut state = populated_state();
    state.shell_input = "say hello".into();
    state.shell_cursor = state.shell_input.len();
    assert!(render_to_text(80, 24, &state, 0).contains("控制台→视觉验收服"));
    state.shell_input = "/help".into();
    state.shell_cursor = state.shell_input.len();
    assert!(render_to_text(80, 24, &state, 0).contains("管理命令"));
}
