use chrono::Utc;
use mc_core::{InstanceSummary, TaskKind, TaskPhase, TaskProgress, TaskSnapshot, TaskStatus};
use ratatui::{backend::TestBackend, style::Color, Terminal};
use uuid::Uuid;

use crate::app::{AppState, DashboardData};

use super::render;

#[test]
fn renders_empty_dashboard_at_reference_width() {
    let mut state = AppState::default();
    state.shell_visible = false;
    let text = render_to_text(80, 24, &state, 0);

    assert!(text.contains("MC"));
    assert!(text.contains("还没有服务器"));
    assert!(text.contains("创建Paper"));
    assert!(text.contains("Fabric"));
    assert!(text.contains("q退出"));
}

#[test]
fn renders_instance_and_task_state() {
    let mut state = AppState::default();
    state.shell_visible = false;
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
        tasks: vec![TaskSnapshot {
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
        }],
    }));
    assert!(state.has_animated_work());
    let text = render_to_text(100, 30, &state, 0);

    assert!(text.contains("生存服"));
    assert!(text.contains("paper1.21.11"));
    assert!(text.contains("下载校验"));
    assert!(text.contains("1/3"));
    assert!(text.contains("◐"));
    assert!(render_to_text(100, 30, &state, 1).contains("◓"));

    let styles = render_to_styles(100, 30, &state, 0);
    let brand = styles
        .iter()
        .find(|(symbol, _, _)| symbol == "◆")
        .expect("colored brand mark");
    assert_eq!(brand.1, Color::Rgb(217, 119, 87));
    assert_eq!(brand.2, Color::Rgb(24, 24, 24));
}

#[test]
fn compact_terminal_render_does_not_panic() {
    let mut state = AppState::default();
    state.shell_visible = false;
    let text = render_to_text(40, 12, &state, 0);
    assert!(text.contains("MC"));
}

fn render_to_text(width: u16, height: u16, state: &AppState, animation_frame: u8) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    terminal
        .draw(|frame| render(frame, state, animation_frame))
        .expect("render dashboard");
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
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
        .draw(|frame| render(frame, state, animation_frame))
        .expect("render dashboard styles");
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| (cell.symbol().to_owned(), cell.fg, cell.bg))
        .collect()
}

#[test]
fn default_session_welcome_has_three_clickable_entries_at_both_sizes() {
    use crate::hit::HitTarget;
    for (width, height) in [(80, 24), (40, 16)] {
        let mut state = AppState::default();
        state.apply_refresh(Ok(DashboardData::default()));
        let text = render_to_text(width, height, &state, 0);
        assert!(text.contains("/create"));
        assert!(text.contains("/fabric"));
        assert!(text.contains("/import"));
        assert!(text.contains("Enter发送"));
        for target in [
            HitTarget::WelcomeCreatePaper,
            HitTarget::WelcomeCreateFabric,
            HitTarget::WelcomeImportModpack,
        ] {
            let found = (0..height).any(|row| {
                (0..width).any(|column| state.hit_target_at(column, row) == Some(target))
            });
            assert!(found, "missing {target:?} at {width}x{height}");
        }
        state.paste_text("/help");
        let _ = render_to_text(width, height, &state, 0);
        assert!(!(0..height).any(|row| (0..width).any(
            |column| state.hit_target_at(column, row) == Some(HitTarget::WelcomeCreateFabric)
        )));
    }
}

#[test]
fn error_dialog_renders_recovery_and_details_without_underlying_mouse_targets() {
    for (width, height) in [(80, 24), (40, 12)] {
        let mut state = AppState::default();
        state.show_error(
            &mc_core::AppError::new(mc_core::ErrorCode::InvalidInput, "bad input")
                .with_action("Correct the input")
                .with_evidence("proof=42"),
        );
        let text = render_to_text(width, height, &state, 0);
        assert!(text.contains("badinput"));
        assert!(text.contains("Correcttheinput"));
        assert!(text.contains("Esc关闭"));
        assert!(!text.contains("proof=42"));
        assert!((0..height)
            .all(|row| (0..width).all(|column| state.hit_target_at(column, row).is_none())));
        state.update(crate::app::Action::ToggleErrorDetails);
        let _ = render_to_text(width, height, &state, 0);
        for _ in 0..5 {
            state.update(crate::app::Action::ErrorScrollDown);
        }
        assert!(render_to_text(width, height, &state, 0).contains("proof=42"));
    }
}
