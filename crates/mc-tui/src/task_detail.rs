use uuid::Uuid;

use mc_core::{AppError, TaskEvent, TaskSnapshot};

const MAX_RETAINED_EVENTS: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TaskDetailQuery {
    pub task_id: Uuid,
    pub after_sequence: i64,
}

#[derive(Debug)]
pub(crate) struct TaskDetailUpdate {
    pub task: TaskSnapshot,
    pub events: Vec<TaskEvent>,
}

#[derive(Debug)]
pub(crate) struct TaskDetailState {
    pub task: TaskSnapshot,
    pub events: Vec<TaskEvent>,
    pub last_event_sequence: i64,
    pub scroll_offset: usize,
    pub error: Option<String>,
}

impl TaskDetailState {
    pub fn new(task: TaskSnapshot) -> Self {
        Self {
            task,
            events: Vec::new(),
            last_event_sequence: 0,
            scroll_offset: 0,
            error: None,
        }
    }

    pub fn query(&self) -> TaskDetailQuery {
        TaskDetailQuery {
            task_id: self.task.id,
            after_sequence: self.last_event_sequence,
        }
    }

    pub fn apply_result(&mut self, result: Result<TaskDetailUpdate, AppError>) {
        let update = match result {
            Ok(update) => update,
            Err(error) => {
                self.error = Some(match error.next_action {
                    Some(action) => format!("{} · 下一步：{}", error.reason, action),
                    None => error.reason,
                });
                return;
            }
        };
        if update.task.id != self.task.id {
            self.error = Some("daemon returned a different task".to_owned());
            return;
        }

        self.task = update.task;
        let mut incoming = update.events;
        incoming.sort_by_key(|event| event.sequence);
        for event in incoming {
            if event.task_id == self.task.id && event.sequence > self.last_event_sequence {
                self.last_event_sequence = event.sequence;
                self.events.push(event);
            }
        }
        self.events.sort_by_key(|event| event.sequence);
        if self.events.len() > MAX_RETAINED_EVENTS {
            let discard = self.events.len() - MAX_RETAINED_EVENTS;
            self.events.drain(..discard);
        }
        self.scroll_offset = self.scroll_offset.min(self.events.len().saturating_sub(1));
        self.error = None;
    }

    pub fn scroll_older(&mut self) {
        self.scroll_offset = (self.scroll_offset + 1).min(self.events.len().saturating_sub(1));
    }

    pub fn scroll_newer(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }
}

#[cfg(test)]
mod tests;
