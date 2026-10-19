use super::{CommandId, CommandPaletteState};
use crate::app::{AppState, Panel};

#[test]
fn dashboard_palette_supports_search_and_guidance_commands() {
    let state = AppState::default();
    let mut palette = CommandPaletteState::for_state(&state);
    assert!(palette
        .filtered_entries()
        .any(|entry| entry.id == CommandId::OpenCreate));
    assert!(palette
        .filtered_entries()
        .any(|entry| entry.id == CommandId::ImportModpack));
    assert!(palette
        .filtered_entries()
        .any(|entry| entry.id == CommandId::Quit));
    for character in "新建".chars() {
        palette.insert(character);
    }
    assert_eq!(
        palette.selected_entry().map(|entry| entry.id),
        Some(CommandId::OpenCreate),
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
        Some(CommandId::ImportModpack),
    );
}

#[test]
fn shell_palette_exposes_session_navigation_and_classic_switch() {
    let state = AppState::default();
    let palette = CommandPaletteState::for_state(&state);
    let ids: Vec<_> = palette.filtered_entries().map(|entry| entry.id).collect();
    assert!(ids.contains(&CommandId::OpenCreate));
    assert!(ids.contains(&CommandId::OpenSelectedInstance));
    assert!(ids.contains(&CommandId::ShowClassic));
    assert!(ids.contains(&CommandId::Theme));
}
