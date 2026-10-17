use crossterm::event::{KeyCode, KeyModifiers};

use crate::app::{Action, Panel};
use crate::wizard::WizardStep;

use super::{action_for_key, UiMode};

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
        Action::ListBackups
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
