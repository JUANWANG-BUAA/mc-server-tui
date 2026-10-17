use std::{io, sync::Arc, time::Duration};

use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use futures_util::StreamExt;
use mc_core::AppError;
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc;

use crate::{
    app::{Action, AppState, DashboardData, Panel},
    instance_detail::{
        InstanceCommand, InstanceCommandResult, InstanceDetailQuery, InstanceDetailUpdate,
    },
    source::ManagementSource,
    task_detail::{TaskDetailQuery, TaskDetailUpdate},
    view::render,
    wizard::{CreateInstanceInput, WizardAction, WizardStep},
};

const REFRESH_INTERVAL: Duration = Duration::from_secs(2);

pub(crate) async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    source: Arc<dyn ManagementSource>,
) -> io::Result<()> {
    let mut state = AppState::default();
    let mut events = EventStream::new();
    let mut refresh_timer = tokio::time::interval_at(
        tokio::time::Instant::now() + REFRESH_INTERVAL,
        REFRESH_INTERVAL,
    );
    refresh_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let (refresh_tx, mut refresh_rx) = mpsc::channel::<Result<DashboardData, AppError>>(1);
    let (create_tx, mut create_rx) = mpsc::channel::<Result<mc_core::TaskSnapshot, AppError>>(1);
    let (detail_tx, mut detail_rx) = mpsc::channel::<Result<TaskDetailUpdate, AppError>>(1);
    let (instance_detail_tx, mut instance_detail_rx) =
        mpsc::channel::<Result<InstanceDetailUpdate, AppError>>(1);
    let (instance_command_tx, mut instance_command_rx) = mpsc::channel::<InstanceCommandResult>(1);
    let mut refresh_in_flight = true;
    let mut create_in_flight = false;
    let mut detail_in_flight = false;
    let mut instance_detail_in_flight = false;
    let mut instance_command_in_flight = false;
    let mut refresh_pending = false;
    let mut detail_pending = false;
    let mut instance_detail_pending = false;
    start_refresh(Arc::clone(&source), refresh_tx.clone());

    while !state.should_quit() {
        terminal.draw(|frame| render(frame, &state))?;
        tokio::select! {
            event = events.next() => match event {
                Some(Ok(Event::Key(key))) if key.kind != KeyEventKind::Release => {
                    let mode = if let Some(wizard) = &state.wizard {
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
                    };
                    let action = action_for_key(key.code, key.modifiers, mode);
                    state.update(action);
                }
                Some(Ok(Event::Resize(_, _))) => {}
                Some(Ok(_)) => {}
                Some(Err(error)) => state.apply_refresh(Err(AppError::new(
                    mc_core::ErrorCode::SourceUnavailable,
                    "Could not read terminal input.",
                ).with_evidence(error.to_string()))),
                None => break,
            },
            _ = refresh_timer.tick(), if (state.task_detail_visible && !detail_in_flight)
                || (!state.task_detail_visible && state.instance_detail_visible && !instance_detail_in_flight
                    && !state.instance_detail.as_ref().is_some_and(|detail| detail.editing_console))
                || (!state.task_detail_visible && !state.instance_detail_visible && !refresh_in_flight) => {
                if state.task_detail_visible {
                    state.update(Action::RefreshTaskDetail);
                } else if state.instance_detail_visible {
                    state.update(Action::RefreshInstanceDetail);
                } else {
                    state.update(Action::Refresh);
                }
            }
            result = refresh_rx.recv(), if refresh_in_flight => {
                refresh_in_flight = false;
                if let Some(result) = result {
                    state.apply_refresh(result);
                }
            }
            result = create_rx.recv(), if create_in_flight => {
                create_in_flight = false;
                if let Some(result) = result {
                    state.apply_create_result(result);
                }
            }
            result = detail_rx.recv(), if detail_in_flight => {
                detail_in_flight = false;
                if let Some(result) = result {
                    state.apply_task_detail_result(result);
                }
            }
            result = instance_detail_rx.recv(), if instance_detail_in_flight => {
                instance_detail_in_flight = false;
                if let Some(result) = result {
                    state.apply_instance_detail_result(result);
                }
            }
            result = instance_command_rx.recv(), if instance_command_in_flight => {
                instance_command_in_flight = false;
                if let Some(result) = result {
                    state.apply_instance_command_result(result);
                }
            }
        }

        if !state.task_detail_visible {
            detail_pending = false;
        }
        if !state.instance_detail_visible {
            instance_detail_pending = false;
        }

        if let Some(input) = state.take_create_request() {
            if !create_in_flight {
                create_in_flight = true;
                start_create(Arc::clone(&source), create_tx.clone(), input);
            }
        }

        if let Some(query) = state.take_task_detail_query() {
            if detail_in_flight {
                detail_pending = true;
            } else {
                detail_in_flight = true;
                start_task_detail(Arc::clone(&source), detail_tx.clone(), query);
            }
        }

        if let Some(query) = state.take_instance_detail_query() {
            if instance_detail_in_flight {
                instance_detail_pending = true;
            } else {
                instance_detail_in_flight = true;
                start_instance_detail(Arc::clone(&source), instance_detail_tx.clone(), query);
            }
        }

        if let Some(command) = state.take_instance_command() {
            if !instance_command_in_flight {
                instance_command_in_flight = true;
                start_instance_command(Arc::clone(&source), instance_command_tx.clone(), command);
            }
        }

        if state.take_refresh_request() {
            if refresh_in_flight {
                refresh_pending = true;
            } else {
                refresh_in_flight = true;
                start_refresh(Arc::clone(&source), refresh_tx.clone());
            }
        }
        if refresh_pending && !refresh_in_flight {
            refresh_pending = false;
            refresh_in_flight = true;
            start_refresh(Arc::clone(&source), refresh_tx.clone());
        }
        if detail_pending && !detail_in_flight && state.task_detail_visible {
            detail_pending = false;
            state.update(Action::RefreshTaskDetail);
        }
        if instance_detail_pending && !instance_detail_in_flight && state.instance_detail_visible {
            instance_detail_pending = false;
            state.update(Action::RefreshInstanceDetail);
        }
    }
    Ok(())
}

fn start_refresh(
    source: Arc<dyn ManagementSource>,
    sender: mpsc::Sender<Result<DashboardData, AppError>>,
) {
    tokio::spawn(async move {
        let _ = sender.send(source.load().await).await;
    });
}

fn start_create(
    source: Arc<dyn ManagementSource>,
    sender: mpsc::Sender<Result<mc_core::TaskSnapshot, AppError>>,
    input: CreateInstanceInput,
) {
    tokio::spawn(async move {
        let _ = sender.send(source.create_instance(input).await).await;
    });
}

fn start_task_detail(
    source: Arc<dyn ManagementSource>,
    sender: mpsc::Sender<Result<TaskDetailUpdate, AppError>>,
    query: TaskDetailQuery,
) {
    tokio::spawn(async move {
        let _ = sender
            .send(
                source
                    .load_task_detail(query.task_id, query.after_sequence)
                    .await,
            )
            .await;
    });
}

fn start_instance_detail(
    source: Arc<dyn ManagementSource>,
    sender: mpsc::Sender<Result<InstanceDetailUpdate, AppError>>,
    query: InstanceDetailQuery,
) {
    tokio::spawn(async move {
        let _ = sender.send(source.load_instance_detail(query).await).await;
    });
}

fn start_instance_command(
    source: Arc<dyn ManagementSource>,
    sender: mpsc::Sender<InstanceCommandResult>,
    command: InstanceCommand,
) {
    tokio::spawn(async move {
        let result = match command {
            InstanceCommand::Start(instance_id) => {
                InstanceCommandResult::Status(source.start_server(instance_id).await)
            }
            InstanceCommand::Stop(instance_id) => {
                InstanceCommandResult::Status(source.stop_server(instance_id).await)
            }
            InstanceCommand::Backup(instance_id) => {
                InstanceCommandResult::Task(source.create_backup(instance_id).await.map(Box::new))
            }
            InstanceCommand::Console {
                instance_id,
                command,
            } => InstanceCommandResult::Console(
                source.send_console_command(instance_id, command).await,
            ),
        };
        let _ = sender.send(result).await;
    });
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UiMode {
    Dashboard(Panel),
    Wizard(WizardStep),
    TaskDetail,
    InstanceDetail { editing_console: bool },
}

fn action_for_key(code: KeyCode, modifiers: KeyModifiers, mode: UiMode) -> Action {
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
mod tests {
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
}
