use std::fs;

use super::{discover_candidates_from, ModpackImportState};
use tempfile::TempDir;

#[test]
fn discovers_mrpack_files_recursively_but_ignores_hidden_directories() {
    let temp = TempDir::new().expect("tempdir");
    let nested = temp.path().join("exports");
    let hidden = temp.path().join(".cache");
    fs::create_dir_all(&nested).expect("nested");
    fs::create_dir_all(&hidden).expect("hidden");
    fs::write(nested.join("server.mrpack"), b"fixture").expect("pack");
    fs::write(hidden.join("ignored.mrpack"), b"fixture").expect("hidden pack");
    let files = discover_candidates_from([temp.path().to_path_buf()]);
    assert!(files.iter().any(|path| path.ends_with("exports/server.mrpack")));
    assert!(!files.iter().any(|path| path.ends_with(".cache/ignored.mrpack")));
}

#[test]
fn import_state_requires_a_candidate_before_analysis() {
    let mut state = ModpackImportState {
        candidates: Vec::new(),
        selected: 0,
        report: None,
        error: Some("none".to_owned()),
        loading: false,
    };
    assert_eq!(state.begin_analysis(), None);
    state
        .candidates
        .push(std::path::PathBuf::from("/tmp/server.mrpack"));
    assert_eq!(
        state.begin_analysis(),
        Some(std::path::PathBuf::from("/tmp/server.mrpack")),
    );
    assert!(state.loading);
}
