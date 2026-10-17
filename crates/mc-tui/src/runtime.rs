use std::{io, sync::Arc, time::Duration};

use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use futures_util::StreamExt;
use mc_core::AppError;
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc;

use crate::{
    app::{Action, AppState, DashboardData},
    source::DashboardSource,
    view::render,
};

const REFRESH_INTERVAL: Duration = Duration::from_secs(2);

pub(crate) async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    source: Arc<dyn DashboardSource>,
) -> io::Result<()> {
    let mut state = AppState::default();
    let mut events = EventStream::new();
    let mut refresh_timer = tokio::time::interval_at(
        tokio::time::Instant::now() + REFRESH_INTERVAL,
        REFRESH_INTERVAL,
    );
    refresh_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let (refresh_tx, mut refresh_rx) = mpsc::channel::<Result<DashboardData, AppError>>(1);
    let mut refresh_in_flight = true;
    let mut refresh_pending = false;
    start_refresh(Arc::clone(&source), refresh_tx.clone());

    while !state.should_quit() {
        terminal.draw(|frame| render(frame, &state))?;
        tokio::select! {
            event = events.next() => match event {
                Some(Ok(Event::Key(key))) if key.kind != KeyEventKind::Release => {
                    state.update(action_for_key(key.code, key.modifiers));
                }
                Some(Ok(Event::Resize(_, _))) => {}
                Some(Ok(_)) => {}
                Some(Err(error)) => state.apply_refresh(Err(AppError::new(
                    mc_core::ErrorCode::SourceUnavailable,
                    "Could not read terminal input.",
                ).with_evidence(error.to_string()))),
                None => break,
            },
            _ = refresh_timer.tick(), if !refresh_in_flight => {
                state.update(Action::Refresh);
            }
            result = refresh_rx.recv(), if refresh_in_flight => {
                refresh_in_flight = false;
                if let Some(result) = result {
                    state.apply_refresh(result);
                }
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
    }
    Ok(())
}

fn start_refresh(
    source: Arc<dyn DashboardSource>,
    sender: mpsc::Sender<Result<DashboardData, AppError>>,
) {
    tokio::spawn(async move {
        let _ = sender.send(source.load().await).await;
    });
}

fn action_for_key(code: KeyCode, modifiers: KeyModifiers) -> Action {
    if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
        return Action::Quit;
    }
    match code {
        KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
        KeyCode::Up | KeyCode::Char('k') => Action::Previous,
        KeyCode::Down | KeyCode::Char('j') => Action::Next,
        KeyCode::Tab => Action::TogglePanel,
        KeyCode::Char('r') => Action::Refresh,
        _ => Action::Ignore,
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyModifiers};

    use crate::app::Action;

    use super::action_for_key;

    #[test]
    fn maps_common_keyboard_actions_without_terminal_io() {
        assert_eq!(
            action_for_key(KeyCode::Char('q'), KeyModifiers::NONE),
            Action::Quit
        );
        assert_eq!(
            action_for_key(KeyCode::Down, KeyModifiers::NONE),
            Action::Next
        );
        assert_eq!(
            action_for_key(KeyCode::Tab, KeyModifiers::NONE),
            Action::TogglePanel
        );
        assert_eq!(
            action_for_key(KeyCode::Char('r'), KeyModifiers::NONE),
            Action::Refresh
        );
        assert_eq!(
            action_for_key(KeyCode::Char('c'), KeyModifiers::CONTROL),
            Action::Quit
        );
    }
}
