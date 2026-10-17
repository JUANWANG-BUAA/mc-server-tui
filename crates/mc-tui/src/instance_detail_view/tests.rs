use chrono::Utc;
use mc_core::{InstanceDetail, InstanceSummary, ServerSnapshot, ServerState};
use ratatui::{backend::TestBackend, Terminal};
use uuid::Uuid;

use crate::instance_detail::InstanceDetailState;

use super::render;

#[test]
fn renders_instance_state_and_controls_at_reference_and_compact_sizes() {
    let instance_id = Uuid::new_v4();
    let mut state = InstanceDetailState::loading(instance_id);
    state.detail = Some(InstanceDetail {
        instance: InstanceSummary {
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
        },
        server: ServerSnapshot {
            instance_id,
            state: ServerState::Running,
            pid: Some(1234),
            port: 25565,
            address: "127.0.0.1:25565".to_owned(),
            started_at: Some(Utc::now()),
            last_exit_code: None,
            message: "服务端已就绪".to_owned(),
        },
    });
    state.logs = "Done (0.4s)!\n玩家加入\n".to_owned();

    let standard = render_to_text(&state, 80, 24);
    assert!(standard.contains("生存服"));
    assert!(standard.contains("运行中"));
    assert!(standard.contains("控制台日志"));
    assert!(standard.contains("启动"));
    assert!(standard.contains("备份"));

    let compact = render_to_text(&state, 40, 12);
    assert!(compact.contains("运行中"));
    assert!(compact.contains("已就绪"));
    assert!(compact.contains("备份"));
}

fn render_to_text(state: &InstanceDetailState, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    terminal
        .draw(|frame| render(frame, state, 0))
        .expect("render instance detail");
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .filter(|symbol| !symbol.chars().all(char::is_whitespace))
        .collect()
}
