use super::{CommandId, CommandPaletteState};
use crate::app::{AppState, Panel};

#[test]
fn dashboard_palette_supports_search_and_guidance_commands() {
    let state = AppState::default();
    let mut palette = CommandPaletteState::for_state(&state);
    assert!(palette
        .filtered_entries()
        .any(|entry| entry.id == CommandId::Shell("create")));
    assert!(palette
        .filtered_entries()
        .any(|entry| entry.id == CommandId::Shell("import")));
    assert!(palette
        .filtered_entries()
        .any(|entry| entry.id == CommandId::Quit));
    for character in "新建".chars() {
        palette.insert(character);
    }
    assert_eq!(
        palette.selected_entry().map(|entry| entry.id),
        Some(CommandId::Shell("create")),
    );
}

#[test]
fn instance_palette_exposes_lifecycle_and_backup_commands() {
    let mut state = AppState::default();
    state.instance_detail_visible = true;
    state.instance_detail_id = Some(uuid::Uuid::new_v4());
    state.focused_panel = Panel::Instances;
    let palette = CommandPaletteState::for_state(&state);
    let ids: Vec<_> = palette.filtered_entries().map(|entry| entry.id).collect();
    assert!(ids.contains(&CommandId::StartServer));
    assert!(ids.contains(&CommandId::StopServer));
    assert!(ids.contains(&CommandId::RestartServer));
    assert!(ids.contains(&CommandId::OpenBackupManager));
    assert!(ids.contains(&CommandId::CreateBackup));
}

#[test]
fn fuzzy_english_queries_reach_backup_commands() {
    let mut state = AppState::default();
    state.instance_detail_visible = true;
    state.instance_detail_id = Some(uuid::Uuid::new_v4());
    state.focused_panel = Panel::Instances;
    let mut palette = CommandPaletteState::for_state(&state);
    for character in "bkp".chars() {
        palette.insert(character);
    }
    assert!(palette
        .filtered_entries()
        .any(|entry| entry.id == CommandId::OpenBackupManager));
}

#[test]
fn english_import_query_reaches_the_modpack_flow() {
    let state = AppState::default();
    let mut palette = CommandPaletteState::for_state(&state);
    for character in "import".chars() {
        palette.insert(character);
    }
    assert_eq!(
        palette.selected_entry().map(|entry| entry.id),
        Some(CommandId::Shell("import")),
    );
}

#[test]
fn shell_palette_exposes_session_navigation_and_classic_switch() {
    let state = AppState::default();
    let palette = CommandPaletteState::for_state(&state);
    let ids: Vec<_> = palette.filtered_entries().map(|entry| entry.id).collect();
    assert!(ids.contains(&CommandId::Shell("create")));
    assert!(ids.contains(&CommandId::Shell("open")));
    assert!(ids.contains(&CommandId::Shell("classic")));
    assert!(ids.contains(&CommandId::Theme));
}

#[test]
fn shell_search_finds_stop_and_player_management() {
    for (query, expected) in [("stop", "stop"), ("封禁", "players"), ("日志", "rg")] {
        let mut palette = CommandPaletteState::for_state(&AppState::default());
        for c in query.chars() {
            palette.insert(c);
        }
        assert!(palette
            .filtered_entries()
            .any(|entry| entry.id == CommandId::Shell(expected)));
    }
}

#[test]
fn detailed_views_have_a_searchable_alternative_to_function_keys() {
    let mut state = AppState::default();
    state.instance_detail_visible = true;
    let mut palette = CommandPaletteState::for_state(&state);
    for ch in "展开".chars() {
        palette.insert(ch);
    }
    assert_eq!(
        palette.selected_entry().map(|entry| entry.id),
        Some(CommandId::ToggleDetails)
    );
}
