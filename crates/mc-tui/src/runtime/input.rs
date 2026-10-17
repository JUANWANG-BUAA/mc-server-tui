use crossterm::event::{KeyCode, KeyModifiers};

use crate::{
    app::{Action, AppState, Panel},
    wizard::{WizardAction, WizardStep},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum UiMode {
    Dashboard(Panel),
    Wizard(WizardStep),
    TaskDetail,
    InstanceDetail { editing_console: bool },
}

pub(super) fn mode_for_state(state: &AppState) -> UiMode {
    if let Some(wizard) = &state.wizard {
        UiMode::Wizard(wizard.step)
    } else if state.task_detail_visible {
        UiMode::TaskDetail
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
    match mode {
        UiMode::Wizard(step) => Action::Wizard(match code {
            KeyCode::Esc => WizardAction::Back,
            KeyCode::Enter => WizardAction::Advance,
            KeyCode::Tab | KeyCode::Down => WizardAction::FocusNext,
            KeyCode::BackTab | KeyCode::Up => WizardAction::FocusPrevious,
            KeyCode::Backspace => WizardAction::Backspace,
            KeyCode::Char(' ') if step == WizardStep::Review => WizardAction::ToggleAcceptance,
            KeyCode::Char(character) => WizardAction::Insert(character),
            _ => return Action::Ignore,
        }),
        UiMode::TaskDetail => match code {
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Esc | KeyCode::Backspace => Action::BackFromTaskDetail,
            KeyCode::Up | KeyCode::Char('k') => Action::Previous,
            KeyCode::Down | KeyCode::Char('j') => Action::Next,
            KeyCode::Char('r') => Action::RefreshTaskDetail,
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
                    KeyCode::Char('l') => Action::ListBackups,
                    KeyCode::Char(':') => Action::BeginConsoleInput,
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
            _ => Action::Ignore,
        },
    }
}

#[cfg(test)]
mod tests;
