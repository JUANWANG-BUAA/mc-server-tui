use crossterm::event::{KeyCode, KeyModifiers};

use crate::{
    app::{Action, AppState, Panel},
    server_properties::EditorPhase,
    wizard::{WizardAction, WizardStep},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum UiMode {
    Help,
    CommandPalette,
    ModpackImport {
        report: bool,
        loading: bool,
    },
    Dashboard(Panel),
    Wizard(WizardStep),
    TaskDetail,
    BackupManager {
        editing_confirmation: bool,
        reviewing_plan: bool,
    },
    InstanceDetail {
        editing_console: bool,
    },
    ServerProperties(EditorPhase),
    Shell,
}

pub(super) fn mode_for_state(state: &AppState) -> UiMode {
    if state.help_visible {
        UiMode::Help
    } else if state.command_palette.is_some() {
        UiMode::CommandPalette
    } else if state.modpack_import_visible {
        let import = state.modpack_import.as_ref();
        UiMode::ModpackImport {
            report: import.is_some_and(|import| import.report.is_some()),
            loading: import.is_some_and(|import| import.loading),
        }
    } else if let Some(wizard) = &state.wizard {
        UiMode::Wizard(wizard.step)
    } else if state.task_detail_visible {
        UiMode::TaskDetail
    } else if state.backup_manager_visible {
        let manager = state.backup_manager.as_ref();
        UiMode::BackupManager {
            editing_confirmation: manager.is_some_and(|manager| manager.editing_confirmation),
            reviewing_plan: manager.is_some_and(|manager| manager.is_reviewing_plan()),
        }
    } else if state.server_properties_visible {
        UiMode::ServerProperties(
            state
                .server_properties
                .as_ref()
                .map(|editor| editor.phase)
                .unwrap_or(EditorPhase::Loading),
        )
    } else if state.instance_detail_visible {
        UiMode::InstanceDetail {
            editing_console: state
                .instance_detail
                .as_ref()
                .is_some_and(|detail| detail.editing_console || detail.editing_filter),
        }
    } else if state.shell_visible {
        UiMode::Shell
    } else {
        UiMode::Dashboard(state.focused_panel)
    }
}

pub(super) fn action_for_key(code: KeyCode, modifiers: KeyModifiers, mode: UiMode) -> Action {
    if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
        return Action::Quit;
    }
    if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('b') {
        return Action::WorkspacePrefix;
    }
    if modifiers.contains(KeyModifiers::ALT) {
        match code {
            KeyCode::Left | KeyCode::Char('b') => return Action::PagePreviousWorkspace,
            KeyCode::Right | KeyCode::Char('f') => return Action::PageNextWorkspace,
            KeyCode::Char('w') => return Action::CloseWorkspacePage,
            _ => return Action::Ignore,
        }
    }
    if code == KeyCode::F(2) {
        return Action::CycleTheme;
    }
    if code == KeyCode::F(1) {
        return if mode == UiMode::Help {
            Action::CloseHelp
        } else {
            Action::ToggleHelp
        };
    }
    if code == KeyCode::Char('k') && modifiers.contains(KeyModifiers::CONTROL) {
        return if mode == UiMode::CommandPalette {
            Action::Ignore
        } else {
            Action::OpenCommandPalette
        };
    }
    if code == KeyCode::Char('l') && modifiers.contains(KeyModifiers::CONTROL) {
        return Action::ShellClear;
    }
    match mode {
        UiMode::Shell => match code {
            KeyCode::Esc => Action::ShellEscape,
            KeyCode::Enter => Action::ShellSubmit,
            KeyCode::Tab => Action::ShellComplete,
            KeyCode::Up => Action::ShellUp,
            KeyCode::Down => Action::ShellDown,
            KeyCode::Backspace => Action::ShellBackspace,
            KeyCode::Delete => Action::ShellDelete,
            KeyCode::Left => Action::ShellCursorLeft,
            KeyCode::Right => Action::ShellCursorRight,
            KeyCode::Home if modifiers.contains(KeyModifiers::CONTROL) => Action::First,
            KeyCode::End if modifiers.contains(KeyModifiers::CONTROL) => Action::Last,
            KeyCode::Home => Action::ShellCursorHome,
            KeyCode::End => Action::ShellCursorEnd,
            KeyCode::Char('a') if modifiers.contains(KeyModifiers::CONTROL) => {
                Action::ShellCursorHome
            }
            KeyCode::Char('e') if modifiers.contains(KeyModifiers::CONTROL) => {
                Action::ShellCursorEnd
            }
            KeyCode::PageUp => Action::PagePrevious,
            KeyCode::PageDown => Action::PageNext,
            KeyCode::Char(character) => Action::ShellInsert(character),
            _ => Action::Ignore,
        },
        UiMode::Help => match code {
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => Action::CloseHelp,
            KeyCode::Up | KeyCode::Char('k') => Action::ScrollUp,
            KeyCode::Down | KeyCode::Char('j') => Action::ScrollDown,
            KeyCode::PageUp => Action::PagePrevious,
            KeyCode::PageDown => Action::PageNext,
            KeyCode::Home | KeyCode::Char('g') => Action::First,
            KeyCode::End | KeyCode::Char('G') => Action::Last,
            _ => Action::Ignore,
        },
        UiMode::ModpackImport { report: true, .. } => match code {
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Esc | KeyCode::Backspace => Action::BackFromModpack,
            KeyCode::Char('r') => Action::RefreshModpack,
            KeyCode::Char('d') => Action::GoDashboard,
            KeyCode::Char('/') => Action::OpenCommandPalette,
            KeyCode::Char('?') => Action::ToggleHelp,
            _ => Action::Ignore,
        },
        UiMode::ModpackImport { loading: true, .. } => match code {
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Char('?') => Action::ToggleHelp,
            _ => Action::Ignore,
        },
        UiMode::ModpackImport { .. } => match code {
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Esc | KeyCode::Backspace => Action::BackFromModpack,
            KeyCode::Up | KeyCode::Char('k') => Action::Previous,
            KeyCode::Down | KeyCode::Char('j') => Action::Next,
            KeyCode::PageUp => Action::PagePrevious,
            KeyCode::PageDown => Action::PageNext,
            KeyCode::Home => Action::First,
            KeyCode::End => Action::Last,
            KeyCode::Enter => Action::AnalyzeModpack,
            KeyCode::Char('r') => Action::RefreshModpack,
            KeyCode::Char('d') => Action::GoDashboard,
            KeyCode::Char('/') => Action::OpenCommandPalette,
            KeyCode::Char('?') => Action::ToggleHelp,
            _ => Action::Ignore,
        },
        UiMode::CommandPalette => match code {
            KeyCode::Esc => Action::BackFromCommandPalette,
            KeyCode::Up => Action::PalettePrevious,
            KeyCode::Down => Action::PaletteNext,
            KeyCode::PageUp => Action::PagePrevious,
            KeyCode::PageDown => Action::PageNext,
            KeyCode::Home => Action::First,
            KeyCode::End => Action::Last,
            KeyCode::Enter => Action::PaletteSubmit,
            KeyCode::Backspace => Action::PaletteBackspace,
            KeyCode::Char(character) => Action::PaletteInsert(character),
            _ => Action::Ignore,
        },
        UiMode::Wizard(step) => Action::Wizard(match code {
            KeyCode::Esc => WizardAction::Back,
            KeyCode::Enter => WizardAction::Advance,
            KeyCode::Tab => WizardAction::FocusNext,
            KeyCode::BackTab => WizardAction::FocusPrevious,
            KeyCode::Up => WizardAction::Up,
            KeyCode::Down => WizardAction::Down,
            KeyCode::PageUp => WizardAction::PagePrevious,
            KeyCode::PageDown => WizardAction::PageNext,
            KeyCode::Home => WizardAction::FirstChoice,
            KeyCode::End => WizardAction::LastChoice,
            KeyCode::Left => WizardAction::PreviousChoice,
            KeyCode::Right => WizardAction::NextChoice,
            KeyCode::Backspace => WizardAction::Backspace,
            KeyCode::Char(' ') if step == WizardStep::Review => WizardAction::ToggleAcceptance,
            KeyCode::Char(character) => WizardAction::Insert(character),
            _ => return Action::Ignore,
        }),
        UiMode::BackupManager {
            editing_confirmation: true,
            ..
        } => match code {
            KeyCode::Esc => Action::RestoreConfirmCancel,
            KeyCode::Enter => Action::RestoreConfirmSubmit,
            KeyCode::Backspace => Action::RestoreConfirmBackspace,
            KeyCode::Char(character) if character.is_ascii_alphabetic() => {
                Action::RestoreConfirmInsert(character)
            }
            _ => Action::Ignore,
        },
        UiMode::BackupManager {
            editing_confirmation: false,
            reviewing_plan: true,
        } => match code {
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Esc | KeyCode::Backspace => Action::BackFromBackups,
            KeyCode::Char('c') => Action::BeginRestoreConfirmation,
            KeyCode::Char('d') => Action::GoDashboard,
            KeyCode::Char('/') => Action::OpenCommandPalette,
            KeyCode::Char('?') => Action::ToggleHelp,
            _ => Action::Ignore,
        },
        UiMode::BackupManager { .. } => match code {
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Esc | KeyCode::Backspace => Action::BackFromBackups,
            KeyCode::Up | KeyCode::Char('k') => Action::Previous,
            KeyCode::Down | KeyCode::Char('j') => Action::Next,
            KeyCode::PageUp => Action::PagePrevious,
            KeyCode::PageDown => Action::PageNext,
            KeyCode::Home => Action::First,
            KeyCode::End => Action::Last,
            KeyCode::Char('r') => Action::RefreshBackups,
            KeyCode::Char('v') => Action::VerifyBackup,
            KeyCode::Char('p') | KeyCode::Enter => Action::PreviewRestore,
            KeyCode::Char('m') => Action::LoadMoreBackups,
            KeyCode::Char('b') => Action::CreateBackup,
            KeyCode::Char('d') => Action::GoDashboard,
            KeyCode::Char('/') => Action::OpenCommandPalette,
            KeyCode::Char('?') => Action::ToggleHelp,
            _ => Action::Ignore,
        },
        UiMode::TaskDetail => match code {
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Esc | KeyCode::Backspace => Action::BackFromTaskDetail,
            KeyCode::Up | KeyCode::Char('k') => Action::Previous,
            KeyCode::Down | KeyCode::Char('j') => Action::Next,
            KeyCode::PageUp => Action::PagePrevious,
            KeyCode::PageDown => Action::PageNext,
            KeyCode::Home => Action::First,
            KeyCode::End => Action::Last,
            KeyCode::Char('r') => Action::RefreshTaskDetail,
            KeyCode::Char('x') => Action::CancelTask,
            KeyCode::Char('d') => Action::GoDashboard,
            KeyCode::Char('/') => Action::OpenCommandPalette,
            KeyCode::Char('?') => Action::ToggleHelp,
            _ => Action::Ignore,
        },
        UiMode::InstanceDetail { editing_console } => {
            if editing_console {
                match code {
                    KeyCode::Esc => Action::ConsoleCancel,
                    KeyCode::Enter => Action::ConsoleSubmit,
                    KeyCode::Backspace => Action::ConsoleBackspace,
                    KeyCode::Char(character) => Action::ConsoleInsert(character),
                    _ => Action::Ignore,
                }
            } else {
                match code {
                    KeyCode::Esc | KeyCode::Backspace => Action::BackFromInstanceDetail,
                    KeyCode::Up | KeyCode::Char('k') => Action::ScrollUp,
                    KeyCode::Down | KeyCode::Char('j') => Action::ScrollDown,
                    KeyCode::PageUp => Action::PagePrevious,
                    KeyCode::PageDown => Action::PageNext,
                    KeyCode::Home => Action::First,
                    KeyCode::End => Action::Last,
                    KeyCode::Char('r') => Action::RefreshInstanceDetail,
                    KeyCode::Char('f') => Action::BeginLogSearch,
                    KeyCode::Left => Action::LogLeft,
                    KeyCode::Right => Action::LogRight,
                    KeyCode::Char('s') => Action::StartServer,
                    KeyCode::Char('x') => Action::StopServer,
                    KeyCode::Char('c') => Action::OpenServerProperties,
                    KeyCode::Char('b') => Action::CreateBackup,
                    KeyCode::Char('l') => Action::OpenBackupManager,
                    KeyCode::Char(':') => Action::BeginConsoleInput,
                    KeyCode::Char('d') => Action::GoDashboard,
                    KeyCode::Char('/') => Action::OpenCommandPalette,
                    KeyCode::Char('?') => Action::ToggleHelp,
                    _ => Action::Ignore,
                }
            }
        }
        UiMode::ServerProperties(EditorPhase::Loading | EditorPhase::Saving) => match code {
            KeyCode::Esc => Action::BackFromServerProperties,
            _ => Action::Ignore,
        },
        UiMode::ServerProperties(EditorPhase::LoadFailed) => match code {
            KeyCode::Esc | KeyCode::Backspace => Action::BackFromServerProperties,
            KeyCode::F(5) | KeyCode::Enter => Action::ServerPropertiesReload,
            _ => Action::Ignore,
        },
        UiMode::ServerProperties(EditorPhase::Review) => match code {
            KeyCode::Esc | KeyCode::Backspace => Action::ServerPropertiesEdit,
            KeyCode::Enter => Action::ServerPropertiesSubmit,
            _ => Action::Ignore,
        },
        UiMode::ServerProperties(EditorPhase::Saved) => match code {
            KeyCode::Esc | KeyCode::Backspace | KeyCode::Enter => Action::BackFromServerProperties,
            _ => Action::Ignore,
        },
        UiMode::ServerProperties(EditorPhase::Edit) => match code {
            KeyCode::Esc => Action::BackFromServerProperties,
            KeyCode::Enter => Action::ServerPropertiesSubmit,
            KeyCode::Tab => Action::ServerPropertiesFocusNext,
            KeyCode::BackTab => Action::ServerPropertiesFocusPrevious,
            KeyCode::Backspace => Action::ServerPropertiesBackspace,
            KeyCode::Delete => Action::ServerPropertiesDelete,
            KeyCode::Left => Action::ServerPropertiesLeft,
            KeyCode::Right => Action::ServerPropertiesRight,
            KeyCode::Home => Action::ServerPropertiesHome,
            KeyCode::End => Action::ServerPropertiesEnd,
            KeyCode::Up => Action::ServerPropertiesAdjust(true),
            KeyCode::Down => Action::ServerPropertiesAdjust(false),
            KeyCode::F(5) => Action::ServerPropertiesReload,
            KeyCode::Char(character) => Action::ServerPropertiesInsert(character),
            _ => Action::Ignore,
        },
        UiMode::Dashboard(panel) => match code {
            KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
            KeyCode::Enter if panel == Panel::Tasks => Action::OpenTaskDetail,
            KeyCode::Enter if panel == Panel::Instances => Action::OpenInstanceDetail,
            KeyCode::Char('n') => Action::OpenCreate,
            KeyCode::Char('i') => Action::OpenModpackImport,
            KeyCode::Up | KeyCode::Char('k') => Action::Previous,
            KeyCode::Down | KeyCode::Char('j') => Action::Next,
            KeyCode::PageUp => Action::PagePrevious,
            KeyCode::PageDown => Action::PageNext,
            KeyCode::Home => Action::First,
            KeyCode::End => Action::Last,
            KeyCode::Tab | KeyCode::BackTab => Action::TogglePanel,
            KeyCode::Char('1') => Action::FocusInstances,
            KeyCode::Char('2') => Action::FocusTasks,
            KeyCode::Char('r') => Action::Refresh,
            KeyCode::Char('t') => Action::ShowShell,
            KeyCode::Char('/') => Action::OpenCommandPalette,
            KeyCode::Char('?') => Action::ToggleHelp,
            _ => Action::Ignore,
        },
    }
}

pub(super) fn workspace_key(code: KeyCode) -> Action {
    match code {
        KeyCode::Char('n') => Action::PageNextWorkspace,
        KeyCode::Char('p') => Action::PagePreviousWorkspace,
        KeyCode::Char('w') => Action::WorkspaceList,
        KeyCode::Char('x') => Action::CloseWorkspacePage,
        KeyCode::Char('0') => Action::WorkspaceSwitch(1),
        KeyCode::Char(c) if c.is_ascii_digit() => {
            Action::WorkspaceSwitch(c.to_digit(10).unwrap_or(1))
        }
        _ => Action::Ignore,
    }
}

#[cfg(test)]
mod tests;
