use std::{io, sync::Arc, time::Duration};

use crossterm::event::{Event, EventStream, KeyEventKind, MouseButton, MouseEventKind};
use futures_util::StreamExt;
use mc_core::AppError;
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc;

mod input;

use crate::{
    app::{Action, AppState, DashboardData},
    backup_manager::BackupCommand,
    hit::HitTarget,
    instance_detail::{
        InstanceCommand, InstanceCommandResult, InstanceDetailQuery, InstanceDetailUpdate,
    },
    server_properties::{ServerPropertiesOperation, ServerPropertiesUpdate},
    source::ManagementSource,
    task_detail::{TaskDetailQuery, TaskDetailUpdate},
    view::render,
    wizard::CreateInstanceInput,
};

const REFRESH_INTERVAL: Duration = Duration::from_secs(2);
const ANIMATION_INTERVAL: Duration = Duration::from_millis(160);
const BACKUP_TASK_POLL_INTERVAL: Duration = Duration::from_millis(500);

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
    let mut animation_timer = tokio::time::interval_at(
        tokio::time::Instant::now() + ANIMATION_INTERVAL,
        ANIMATION_INTERVAL,
    );
    animation_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut backup_poll_timer = tokio::time::interval_at(
        tokio::time::Instant::now() + BACKUP_TASK_POLL_INTERVAL,
        BACKUP_TASK_POLL_INTERVAL,
    );
    backup_poll_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut task_detail_poll_timer = tokio::time::interval_at(
        tokio::time::Instant::now() + BACKUP_TASK_POLL_INTERVAL,
        BACKUP_TASK_POLL_INTERVAL,
    );
    task_detail_poll_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut shell_detail_timer = tokio::time::interval_at(
        tokio::time::Instant::now() + REFRESH_INTERVAL,
        REFRESH_INTERVAL,
    );
    shell_detail_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut activity_timer = tokio::time::interval(Duration::from_millis(200));
    activity_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let (tasks_tx, mut tasks_rx) = mpsc::channel(1);
    let mut tasks_in_flight = false;
    let mut animation_frame = 0u8;
    let mut scroll_drag_column = None;
    let (refresh_tx, mut refresh_rx) = mpsc::channel::<Result<DashboardData, AppError>>(1);
    let (create_tx, mut create_rx) = mpsc::channel::<Result<mc_core::TaskSnapshot, AppError>>(1);
    let (create_options_tx, mut create_options_rx) =
        mpsc::channel::<Result<crate::wizard::CreateOptions, AppError>>(1);
    let (detail_tx, mut detail_rx) = mpsc::channel::<Result<TaskDetailUpdate, AppError>>(1);
    let (instance_detail_tx, mut instance_detail_rx) =
        mpsc::channel::<Result<InstanceDetailUpdate, AppError>>(1);
    let (instance_command_tx, mut instance_command_rx) = mpsc::channel::<InstanceCommandResult>(1);
    let (backup_command_tx, mut backup_command_rx) =
        mpsc::channel::<Result<mc_core::TaskSnapshot, AppError>>(1);
    let (backup_task_tx, mut backup_task_rx) =
        mpsc::channel::<Result<mc_core::TaskSnapshot, AppError>>(1);
    let (task_cancel_tx, mut task_cancel_rx) =
        mpsc::channel::<Result<mc_core::TaskSnapshot, AppError>>(1);
    let (modpack_tx, mut modpack_rx) =
        mpsc::channel::<Result<mc_core::ModpackAnalysisReport, AppError>>(1);
    let (server_properties_tx, mut server_properties_rx) =
        mpsc::channel::<ServerPropertiesUpdate>(1);
    let mut refresh_in_flight = true;
    let mut create_in_flight = false;
    let mut create_options_in_flight = false;
    let mut detail_in_flight = false;
    let mut instance_detail_in_flight = false;
    let mut instance_command_in_flight = false;
    let mut backup_command_in_flight = false;
    let mut backup_task_in_flight = false;
    let mut task_cancel_in_flight = false;
    let mut modpack_in_flight = false;
    let mut server_properties_in_flight = false;
    let mut refresh_pending = false;
    let mut detail_pending = false;
    let mut instance_detail_pending = false;
    start_refresh(Arc::clone(&source), refresh_tx.clone());

    while !state.should_quit() {
        state.sync_pages();
        terminal.draw(|frame| render(frame, &state, animation_frame))?;
        tokio::select! {
            event = events.next() => match event {
                Some(Ok(Event::Key(key))) if key.kind != KeyEventKind::Release => {
                    let action = if state.workspace_prefix {
                        state.workspace_prefix = false;
                        input::workspace_key(key.code)
                    } else {
                        input::action_for_key(key.code, key.modifiers, input::mode_for_state(&state))
                    };
                    state.update(action);
                }
                Some(Ok(Event::Paste(text))) => state.paste_text(&text),
                Some(Ok(Event::Mouse(mouse))) => match mouse.kind {
                    MouseEventKind::ScrollUp => state.mouse_wheel(mouse.column, mouse.row, false),
                    MouseEventKind::ScrollDown => state.mouse_wheel(mouse.column, mouse.row, true),
                    MouseEventKind::Down(MouseButton::Left) => {
                        if let Some(target) = state.hit_target_at(mouse.column, mouse.row) {
                            if matches!(target, HitTarget::Scrollbar(..)) {
                                scroll_drag_column = Some(mouse.column);
                            }
                            state.apply_click(target);
                        }
                    }
                    // Dragging follows the scrollbar column even when the pointer
                    // drifts sideways, so the thumb keeps tracking the row.
                    MouseEventKind::Drag(MouseButton::Left) => {
                        if let Some(column) = scroll_drag_column {
                            if let Some(target @ HitTarget::Scrollbar(..)) =
                                state.hit_target_at(column, mouse.row)
                            {
                                state.apply_click(target);
                            }
                        }
                    }
                    MouseEventKind::Up(MouseButton::Left) => scroll_drag_column = None,
                    _ => {}
                },
                Some(Ok(Event::Resize(_, _))) => {}
                Some(Ok(_)) => {}
                Some(Err(error)) => state.apply_refresh(Err(AppError::new(
                    mc_core::ErrorCode::SourceUnavailable,
                    "Could not read terminal input.",
                ).with_evidence(error.to_string()))),
                None => break,
            },
            _ = refresh_timer.tick(), if (state.task_detail_visible && !detail_in_flight)
                || (!state.task_detail_visible && !state.backup_manager_visible && !state.server_properties_visible && state.instance_detail_visible && !instance_detail_in_flight
                    && !state.instance_detail.as_ref().is_some_and(|detail| detail.editing_console))
                || (!state.task_detail_visible && !state.backup_manager_visible && !state.server_properties_visible && !state.instance_detail_visible && !refresh_in_flight) => {
                if state.task_detail_visible {
                    state.update(Action::RefreshTaskDetail);
                } else if state.instance_detail_visible {
                    state.update(Action::RefreshInstanceDetail);
                } else {
                    state.update(Action::Refresh);
                }
            }
            // Active tasks are polled every 200 ms so progress bars move between
            // the two-second dashboard refreshes; one request is in flight at most.
            _ = activity_timer.tick(), if !tasks_in_flight && state.has_active_tasks() => {
                tasks_in_flight = true;
                let source = Arc::clone(&source);
                let sender = tasks_tx.clone();
                tokio::spawn(async move {
                    let _ = sender.send(source.load_tasks().await).await;
                });
            }
            result = tasks_rx.recv(), if tasks_in_flight => {
                tasks_in_flight = false;
                if let Some(Ok(tasks)) = result {
                    state.apply_task_list(tasks);
                }
            }
            _ = animation_timer.tick(), if state.has_animated_work() || state.shell_animation_active() => {
                animation_frame = animation_frame.wrapping_add(1);
            }
            _ = backup_poll_timer.tick(), if !backup_task_in_flight && state.backup_task_to_poll().is_some() => {
                if let Some(task_id) = state.backup_task_to_poll() {
                    backup_task_in_flight = true;
                    start_backup_task_poll(Arc::clone(&source), backup_task_tx.clone(), task_id);
                }
            }
            _ = task_detail_poll_timer.tick(), if !detail_in_flight && state.task_detail_should_poll_fast() => {
                state.update(Action::RefreshTaskDetail);
            }
            _ = shell_detail_timer.tick(), if state.shell_visible
                && !state.server_properties_visible
                && !state.instance_detail_visible
                && !instance_detail_in_flight
                && state.instance_detail_id.is_some() => {
                state.update(Action::RefreshInstanceDetail);
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
            result = create_options_rx.recv(), if create_options_in_flight => {
                create_options_in_flight = false;
                if let Some(result) = result {
                    state.apply_create_options(result);
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
            result = backup_command_rx.recv(), if backup_command_in_flight => {
                backup_command_in_flight = false;
                if let Some(result) = result {
                    state.apply_backup_command_result(result);
                }
            }
            result = backup_task_rx.recv(), if backup_task_in_flight => {
                backup_task_in_flight = false;
                if let Some(result) = result {
                    state.apply_backup_task_update(result);
                }
            }

            result = task_cancel_rx.recv(), if task_cancel_in_flight => {
                task_cancel_in_flight = false;
                if let Some(result) = result {
                    state.apply_task_cancel_result(result);
                }
            }
            result = modpack_rx.recv(), if modpack_in_flight => {
                modpack_in_flight = false;
                if let Some(result) = result {
                    state.apply_modpack_analysis_result(result);
                }
            }
            result = server_properties_rx.recv(), if server_properties_in_flight => {
                server_properties_in_flight = false;
                if let Some(update) = result {
                    state.apply_server_properties_result(update.request_id, update.result);
                }
            }
        }

        if !state.task_detail_visible {
            detail_pending = false;
        }
        if !state.instance_detail_visible && !state.shell_visible {
            instance_detail_pending = false;
        }

        if state.take_create_options_request() && !create_options_in_flight {
            create_options_in_flight = true;
            start_create_options(Arc::clone(&source), create_options_tx.clone());
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

        if let Some(command) = state.take_backup_command() {
            if !backup_command_in_flight {
                backup_command_in_flight = true;
                start_backup_command(Arc::clone(&source), backup_command_tx.clone(), command);
            }
        }

        if let Some(task_id) = state.take_task_cancel_request() {
            if !task_cancel_in_flight {
                task_cancel_in_flight = true;
                start_task_cancel(Arc::clone(&source), task_cancel_tx.clone(), task_id);
            }
        }

        if let Some(path) = state.take_modpack_analysis_request() {
            if !modpack_in_flight {
                modpack_in_flight = true;
                start_modpack_analysis(Arc::clone(&source), modpack_tx.clone(), path);
            }
        }

        if !server_properties_in_flight {
            if let Some(request) = state.take_server_properties_request() {
                server_properties_in_flight = true;
                start_server_properties(
                    Arc::clone(&source),
                    server_properties_tx.clone(),
                    request.request_id,
                    request.operation,
                );
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
        if instance_detail_pending
            && !instance_detail_in_flight
            && (state.instance_detail_visible || state.shell_visible)
        {
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
            InstanceCommand::Manage {
                instance_id,
                action,
            } => InstanceCommandResult::Management(
                source
                    .manage_instance(instance_id, action)
                    .await
                    .map(Box::new),
            ),
            InstanceCommand::Start(instance_id) => {
                InstanceCommandResult::Status(source.start_server(instance_id).await)
            }
            InstanceCommand::Stop(instance_id) => {
                InstanceCommandResult::Status(source.stop_server(instance_id).await)
            }
            InstanceCommand::Restart(instance_id) => {
                InstanceCommandResult::Status(source.restart_server(instance_id).await)
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

fn start_server_properties(
    source: Arc<dyn ManagementSource>,
    sender: mpsc::Sender<ServerPropertiesUpdate>,
    request_id: uuid::Uuid,
    operation: ServerPropertiesOperation,
) {
    tokio::spawn(async move {
        let result = match operation {
            ServerPropertiesOperation::Read { instance_id } => {
                source.load_server_properties(instance_id).await
            }
            ServerPropertiesOperation::Save {
                instance_id,
                expected_revision,
                values,
            } => {
                source
                    .save_server_properties(instance_id, expected_revision, values)
                    .await
            }
        };
        let _ = sender
            .send(ServerPropertiesUpdate { request_id, result })
            .await;
    });
}

fn start_backup_command(
    source: Arc<dyn ManagementSource>,
    sender: mpsc::Sender<Result<mc_core::TaskSnapshot, AppError>>,
    command: BackupCommand,
) {
    tokio::spawn(async move {
        let result = match command {
            BackupCommand::List {
                instance_id,
                cursor,
            } => source.list_backups(instance_id, cursor).await,
            BackupCommand::Create { instance_id } => source.create_backup(instance_id).await,
            BackupCommand::Verify {
                instance_id,
                backup_id,
            } => source.verify_backup(instance_id, backup_id).await,
            BackupCommand::PlanRestore {
                instance_id,
                backup_id,
            } => source.plan_restore(instance_id, backup_id).await,
            BackupCommand::Restore {
                instance_id,
                backup_id,
                confirmation_token,
            } => {
                source
                    .restore_backup(instance_id, backup_id, confirmation_token)
                    .await
            }
        };
        let _ = sender.send(result).await;
    });
}

fn start_backup_task_poll(
    source: Arc<dyn ManagementSource>,
    sender: mpsc::Sender<Result<mc_core::TaskSnapshot, AppError>>,
    task_id: uuid::Uuid,
) {
    tokio::spawn(async move {
        let _ = sender.send(source.load_task(task_id).await).await;
    });
}

fn start_task_cancel(
    source: Arc<dyn ManagementSource>,
    sender: mpsc::Sender<Result<mc_core::TaskSnapshot, AppError>>,
    task_id: uuid::Uuid,
) {
    tokio::spawn(async move {
        let _ = sender.send(source.cancel_task(task_id).await).await;
    });
}

fn start_create_options(
    source: Arc<dyn ManagementSource>,
    sender: mpsc::Sender<Result<crate::wizard::CreateOptions, AppError>>,
) {
    tokio::spawn(async move {
        let _ = sender.send(source.load_create_options().await).await;
    });
}

fn start_modpack_analysis(
    source: Arc<dyn ManagementSource>,
    sender: mpsc::Sender<Result<mc_core::ModpackAnalysisReport, AppError>>,
    path: std::path::PathBuf,
) {
    tokio::spawn(async move {
        let _ = sender.send(source.analyze_modpack(path).await).await;
    });
}
