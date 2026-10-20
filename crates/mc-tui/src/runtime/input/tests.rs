use crossterm::event::{KeyCode, KeyModifiers};

use crate::app::{Action, Panel};
use crate::wizard::WizardStep;

use super::{action_for_key, mode_for_state, UiMode};

#[test]
fn maps_common_keyboard_actions_without_terminal_io() {
    assert_eq!(
        action_for_key(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
            UiMode::Dashboard(Panel::Instances),
        ),
        Action::Quit
    );
    assert_eq!(
        action_for_key(
            KeyCode::Down,
            KeyModifiers::NONE,
            UiMode::Dashboard(Panel::Instances),
        ),
        Action::Next
    );
    assert_eq!(
        action_for_key(
            KeyCode::Tab,
            KeyModifiers::NONE,
            UiMode::Dashboard(Panel::Instances),
        ),
        Action::TogglePanel
    );
    assert_eq!(
        action_for_key(
            KeyCode::Char('r'),
            KeyModifiers::NONE,
            UiMode::Dashboard(Panel::Instances),
        ),
        Action::Refresh
    );
    assert_eq!(
        action_for_key(
            KeyCode::Enter,
            KeyModifiers::NONE,
            UiMode::Dashboard(Panel::Instances),
        ),
        Action::OpenInstanceDetail
    );
    assert_eq!(
        action_for_key(
            KeyCode::Char('s'),
            KeyModifiers::NONE,
            UiMode::InstanceDetail {
                editing_console: false,
            },
        ),
        Action::StartServer
    );
    assert_eq!(
        action_for_key(
            KeyCode::Char('b'),
            KeyModifiers::NONE,
            UiMode::InstanceDetail {
                editing_console: false,
            },
        ),
        Action::CreateBackup
    );
    assert_eq!(
        action_for_key(
            KeyCode::Char('l'),
            KeyModifiers::NONE,
            UiMode::InstanceDetail {
                editing_console: false,
            },
        ),
        Action::OpenBackupManager
    );
    assert_eq!(
        action_for_key(
            KeyCode::Enter,
            KeyModifiers::NONE,
            UiMode::InstanceDetail {
                editing_console: true,
            },
        ),
        Action::ConsoleSubmit
    );
    assert_eq!(
        action_for_key(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
            UiMode::Dashboard(Panel::Instances),
        ),
        Action::Quit
    );
}

#[test]
fn backup_manager_maps_browse_review_and_confirmation_keys() {
    let browse = UiMode::BackupManager {
        editing_confirmation: false,
        reviewing_plan: false,
    };
    assert_eq!(
        action_for_key(KeyCode::Char('v'), KeyModifiers::NONE, browse),
        Action::VerifyBackup
    );
    assert_eq!(
        action_for_key(KeyCode::Enter, KeyModifiers::NONE, browse),
        Action::PreviewRestore
    );
    assert_eq!(
        action_for_key(KeyCode::Char('m'), KeyModifiers::NONE, browse),
        Action::LoadMoreBackups
    );

    let review = UiMode::BackupManager {
        editing_confirmation: false,
        reviewing_plan: true,
    };
    assert_eq!(
        action_for_key(KeyCode::Char('c'), KeyModifiers::NONE, review),
        Action::BeginRestoreConfirmation
    );
    assert_eq!(
        action_for_key(KeyCode::Esc, KeyModifiers::NONE, review),
        Action::BackFromBackups
    );

    let confirmation = UiMode::BackupManager {
        editing_confirmation: true,
        reviewing_plan: true,
    };
    assert_eq!(
        action_for_key(KeyCode::Char('r'), KeyModifiers::NONE, confirmation),
        Action::RestoreConfirmInsert('r')
    );
    assert_eq!(
        action_for_key(KeyCode::Enter, KeyModifiers::NONE, confirmation),
        Action::RestoreConfirmSubmit
    );
}

#[test]
fn wizard_keeps_text_keys_and_uses_escape_to_go_back() {
    assert_eq!(
        action_for_key(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
            UiMode::Wizard(WizardStep::Server)
        ),
        Action::Wizard(crate::wizard::WizardAction::Insert('q'))
    );
    assert_eq!(
        action_for_key(
            KeyCode::Esc,
            KeyModifiers::NONE,
            UiMode::Wizard(WizardStep::Server)
        ),
        Action::Wizard(crate::wizard::WizardAction::Back)
    );
    assert_eq!(
        action_for_key(
            KeyCode::Char('n'),
            KeyModifiers::NONE,
            UiMode::Dashboard(Panel::Instances),
        ),
        Action::OpenCreate
    );
    assert_eq!(
        action_for_key(
            KeyCode::Char(' '),
            KeyModifiers::NONE,
            UiMode::Wizard(WizardStep::Server)
        ),
        Action::Wizard(crate::wizard::WizardAction::Insert(' '))
    );
    assert_eq!(
        action_for_key(
            KeyCode::Char(' '),
            KeyModifiers::NONE,
            UiMode::Wizard(WizardStep::Review)
        ),
        Action::Wizard(crate::wizard::WizardAction::ToggleAcceptance)
    );
    assert_eq!(
        action_for_key(
            KeyCode::Enter,
            KeyModifiers::NONE,
            UiMode::Dashboard(Panel::Tasks),
        ),
        Action::OpenTaskDetail
    );
    assert_eq!(
        action_for_key(KeyCode::Esc, KeyModifiers::NONE, UiMode::TaskDetail),
        Action::BackFromTaskDetail
    );
}

#[test]
fn slash_and_ctrl_k_open_the_discoverable_command_palette() {
    assert_eq!(
        action_for_key(
            KeyCode::Char('/'),
            KeyModifiers::NONE,
            UiMode::Dashboard(Panel::Instances),
        ),
        Action::OpenCommandPalette
    );
    assert_eq!(
        action_for_key(
            KeyCode::Char('k'),
            KeyModifiers::CONTROL,
            UiMode::Wizard(WizardStep::Server),
        ),
        Action::OpenCommandPalette
    );
    assert_eq!(
        action_for_key(
            KeyCode::Backspace,
            KeyModifiers::NONE,
            UiMode::CommandPalette,
        ),
        Action::PaletteBackspace
    );
    assert_eq!(
        action_for_key(
            KeyCode::Char('备'),
            KeyModifiers::NONE,
            UiMode::CommandPalette,
        ),
        Action::PaletteInsert('备')
    );
}

#[test]
fn command_palette_keeps_j_and_k_for_search_and_uses_arrows_for_navigation() {
    assert_eq!(
        action_for_key(
            KeyCode::Char('j'),
            KeyModifiers::NONE,
            UiMode::CommandPalette,
        ),
        Action::PaletteInsert('j')
    );
    assert_eq!(
        action_for_key(
            KeyCode::Char('k'),
            KeyModifiers::NONE,
            UiMode::CommandPalette,
        ),
        Action::PaletteInsert('k')
    );
    assert_eq!(
        action_for_key(KeyCode::Up, KeyModifiers::NONE, UiMode::CommandPalette),
        Action::PalettePrevious
    );
    assert_eq!(
        action_for_key(KeyCode::Down, KeyModifiers::NONE, UiMode::CommandPalette),
        Action::PaletteNext
    );
}

#[test]
fn modpack_import_maps_selection_analysis_and_report_navigation() {
    let browsing = UiMode::ModpackImport {
        report: false,
        loading: false,
    };
    assert_eq!(
        action_for_key(KeyCode::Down, KeyModifiers::NONE, browsing),
        Action::Next
    );
    assert_eq!(
        action_for_key(KeyCode::Enter, KeyModifiers::NONE, browsing),
        Action::AnalyzeModpack
    );
    assert_eq!(
        action_for_key(KeyCode::Char('r'), KeyModifiers::NONE, browsing),
        Action::RefreshModpack
    );
    let review = UiMode::ModpackImport {
        report: true,
        loading: false,
    };
    assert_eq!(
        action_for_key(KeyCode::Esc, KeyModifiers::NONE, review),
        Action::BackFromModpack
    );
    let loading = UiMode::ModpackImport {
        report: false,
        loading: true,
    };
    assert_eq!(
        action_for_key(KeyCode::Enter, KeyModifiers::NONE, loading),
        Action::Ignore
    );
}

#[test]
fn shell_mode_maps_composer_navigation_and_theme_keys() {
    assert_eq!(
        action_for_key(KeyCode::Enter, KeyModifiers::NONE, UiMode::Shell),
        Action::ShellSubmit
    );
    assert_eq!(
        action_for_key(KeyCode::Tab, KeyModifiers::NONE, UiMode::Shell),
        Action::ShellComplete
    );
    assert_eq!(
        action_for_key(KeyCode::Up, KeyModifiers::NONE, UiMode::Shell),
        Action::ShellUp
    );
    assert_eq!(
        action_for_key(KeyCode::Left, KeyModifiers::NONE, UiMode::Shell),
        Action::ShellCursorLeft
    );
    assert_eq!(
        action_for_key(KeyCode::Home, KeyModifiers::NONE, UiMode::Shell),
        Action::ShellCursorHome
    );
    assert_eq!(
        action_for_key(KeyCode::Home, KeyModifiers::CONTROL, UiMode::Shell),
        Action::First
    );
    assert_eq!(
        action_for_key(KeyCode::Char('a'), KeyModifiers::CONTROL, UiMode::Shell),
        Action::ShellCursorHome
    );
    assert_eq!(
        action_for_key(KeyCode::Delete, KeyModifiers::NONE, UiMode::Shell),
        Action::ShellDelete
    );
    assert_eq!(
        action_for_key(KeyCode::Char('/'), KeyModifiers::NONE, UiMode::Shell),
        Action::ShellInsert('/')
    );
    assert_eq!(
        action_for_key(KeyCode::PageUp, KeyModifiers::NONE, UiMode::Shell),
        Action::PagePrevious
    );
    assert_eq!(
        action_for_key(KeyCode::F(2), KeyModifiers::NONE, UiMode::Shell),
        Action::CycleTheme
    );
    assert_eq!(
        mode_for_state(&crate::app::AppState::default()),
        UiMode::Shell
    );
}
