use crossterm::event::{KeyCode, KeyModifiers};

use crate::{
    app::{Action, AppState, Panel},
    wizard::{WizardAction, WizardStep},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum UiMode {
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
}

pub(super) fn mode_for_state(state: &AppState) -> UiMode {
    if state.command_palette.is_some() {
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
    } else if state.instance_detail_visible {
        UiMode::InstanceDetail {
            editing_console: state
                .instance_detail
                .as_ref()
                .is_some_and(|detail| detail.editing_console),
        }
    } else {
        UiMode::Dashboard(state.focused_panel)
    }
}

pub(super) fn action_for_key(code: KeyCode, modifiers: KeyModifiers, mode: UiMode) -> Action {
    if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
        return Action::Quit;
    }
    if code == KeyCode::Char('k') && modifiers.contains(KeyModifiers::CONTROL) {
        return Action::OpenCommandPalette;
    }
    match mode {
        UiMode::ModpackImport { report: true, .. } => match code {
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Esc | KeyCode::Backspace => Action::BackFromModpack,
            KeyCode::Char('r') => Action::OpenModpackImport,
            _ => Action::Ignore,
        },
        UiMode::ModpackImport { loading: true, .. } => match code {
            KeyCode::Char('q') => Action::Quit,
            _ => Action::Ignore,
        },
        UiMode::ModpackImport { .. } => match code {
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Esc | KeyCode::Backspace => Action::BackFromModpack,
            KeyCode::Up | KeyCode::Char('k') => Action::Previous,
            KeyCode::Down | KeyCode::Char('j') => Action::Next,
            KeyCode::Enter => Action::AnalyzeModpack,
            KeyCode::Char('r') => Action::RefreshModpack,
            _ => Action::Ignore,
        },
        UiMode::CommandPalette => match code {
            KeyCode::Esc => Action::BackFromCommandPalette,
            KeyCode::Up | KeyCode::Char('k') => Action::PalettePrevious,
            KeyCode::Down | KeyCode::Char('j') => Action::PaletteNext,
            KeyCode::Enter => Action::PaletteSubmit,
            KeyCode::Backspace => Action::PaletteBackspace,
            KeyCode::Char(character) => Action::PaletteInsert(character),
            _ => Action::Ignore,
        },
        UiMode::Wizard(step) => Action::Wizard(match code {
            KeyCode::Esc => WizardAction::Back,
            KeyCode::Enter => WizardAction::Advance,
            KeyCode::Tab | KeyCode::Down => WizardAction::FocusNext,
            KeyCode::BackTab | KeyCode::Up => WizardAction::FocusPrevious,
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
            KeyCode::Char('/') => Action::OpenCommandPalette,
            _ => Action::Ignore,
        },
        UiMode::BackupManager { .. } => match code {
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Esc | KeyCode::Backspace => Action::BackFromBackups,
            KeyCode::Up | KeyCode::Char('k') => Action::Previous,
            KeyCode::Down | KeyCode::Char('j') => Action::Next,
            KeyCode::Char('r') => Action::RefreshBackups,
            KeyCode::Char('v') => Action::VerifyBackup,
            KeyCode::Char('p') | KeyCode::Enter => Action::PreviewRestore,
            KeyCode::Char('m') => Action::LoadMoreBackups,
            KeyCode::Char('b') => Action::CreateBackup,
            KeyCode::Char('/') => Action::OpenCommandPalette,
            _ => Action::Ignore,
        },
        UiMode::TaskDetail => match code {
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Esc | KeyCode::Backspace => Action::BackFromTaskDetail,
            KeyCode::Up | KeyCode::Char('k') => Action::Previous,
            KeyCode::Down | KeyCode::Char('j') => Action::Next,
            KeyCode::Char('r') => Action::RefreshTaskDetail,
            KeyCode::Char('x') => Action::CancelTask,
            KeyCode::Char('/') => Action::OpenCommandPalette,
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
                    KeyCode::Char('r') => Action::RefreshInstanceDetail,
                    KeyCode::Char('s') => Action::StartServer,
                    KeyCode::Char('x') => Action::StopServer,
                    KeyCode::Char('b') => Action::CreateBackup,
                    KeyCode::Char('l') => Action::OpenBackupManager,
                    KeyCode::Char(':') => Action::BeginConsoleInput,
                    KeyCode::Char('/') => Action::OpenCommandPalette,
                    _ => Action::Ignore,
                }
            }
        }
        UiMode::Dashboard(panel) => match code {
            KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
            KeyCode::Enter if panel == Panel::Tasks => Action::OpenTaskDetail,
            KeyCode::Enter if panel == Panel::Instances => Action::OpenInstanceDetail,
            KeyCode::Char('n') => Action::OpenCreate,
            KeyCode::Up | KeyCode::Char('k') => Action::Previous,
            KeyCode::Down | KeyCode::Char('j') => Action::Next,
            KeyCode::Tab => Action::TogglePanel,
            KeyCode::Char('r') => Action::Refresh,
            KeyCode::Char('/') => Action::OpenCommandPalette,
            _ => Action::Ignore,
        },
    }
}

#[cfg(test)]
mod tests;
