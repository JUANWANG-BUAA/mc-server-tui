use std::path::PathBuf;

use mc_core::{ModpackAnalysisReport, ModpackIssue, ModpackSeverity, ModpackStatus};
use ratatui::{backend::TestBackend, Terminal};

use crate::hit::HitMap;

use crate::modpack_import::ModpackImportState;

use super::render;

#[test]
fn renders_local_modpack_picker_at_reference_and_compact_sizes() {
    let state = ModpackImportState {
        candidates: vec![PathBuf::from("/tmp/Example Server.mrpack")],
        selected: 0,
        report: None,
        error: None,
        loading: false,
    };
    let standard = render_to_text(&state, 80, 24);
    assert!(standard.contains("整合包导入"));
    assert!(standard.contains("ExampleServer.mrpack"));
    assert!(standard.contains("Enter检查"));
    let compact = render_to_text(&state, 40, 12);
    assert!(compact.contains("整合包导入"));
}

#[test]
fn renders_blocked_compatibility_report_and_guidance() {
    let state = ModpackImportState {
        candidates: vec![PathBuf::from("/tmp/Example.mrpack")],
        selected: 0,
        report: Some(ModpackAnalysisReport {
            path: PathBuf::from("/tmp/Example.mrpack"),
            name: "Example Pack".to_owned(),
            version_id: "2.1.0".to_owned(),
            minecraft_version: Some("1.21.1".to_owned()),
            loader: Some(mc_core::ModpackLoader {
                dependency: "fabric-loader".to_owned(),
                version: "0.16.9".to_owned(),
            }),
            status: ModpackStatus::Blocked,
            install_files: 4,
            review_files: 2,
            excluded_files: 3,
            mod_files: 5,
            total_install_bytes: 64 * 1024 * 1024,
            server_override_files: 1,
            issues: vec![ModpackIssue {
                severity: ModpackSeverity::Blocker,
                code: "unsafe_path".to_owned(),
                title: "整合包包含不安全路径".to_owned(),
                detail: "../escape.jar".to_owned(),
                action: Some("删除异常条目后重新导出。".to_owned()),
            }],
            files: Vec::new(),
            files_truncated: false,
        }),
        error: None,
        loading: false,
    };
    let text = render_to_text(&state, 80, 24);
    assert!(text.contains("整合包适配报告"));
    assert!(text.contains("禁止导入"));
    assert!(text.contains("fabric-loader0.16.9"));
    assert!(text.contains("不安全路径"));
    assert!(text.contains("不会下载Mod"));
}

fn render_to_text(state: &ModpackImportState, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    terminal
        .draw(|frame| render(frame, state, 0, &mut HitMap::default()))
        .expect("render");
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .filter(|symbol| !symbol.chars().all(char::is_whitespace))
        .collect()
}
