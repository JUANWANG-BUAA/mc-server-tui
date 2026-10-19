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
    pub notice: Option<String>,
    pub cancel_armed: bool,
    pub cancel_requested: bool,
}

impl TaskDetailState {
    pub fn new(task: TaskSnapshot) -> Self {
        Self {
            task,
            events: Vec::new(),
            last_event_sequence: 0,
            scroll_offset: 0,
            error: None,
            notice: None,
            cancel_armed: false,
            cancel_requested: false,
        }
    }

    pub fn can_cancel(&self) -> bool {
        crate::task_presentation::is_cancelable_download(&self.task)
    }

    pub fn request_cancel(&mut self) -> Option<Uuid> {
        if !self.can_cancel() || self.cancel_requested {
            return None;
        }
        if !self.cancel_armed {
            self.cancel_armed = true;
            self.notice = Some("再次按 x 确认取消；下载已完成的部分会保留用于续传。".to_owned());
            return None;
        }
        self.cancel_armed = false;
        self.cancel_requested = true;
        self.notice = Some("正在向后端发送取消请求…".to_owned());
        Some(self.task.id)
    }

    pub fn apply_cancel_result(&mut self, result: Result<mc_core::TaskSnapshot, AppError>) {
        match result {
            Ok(task) if task.id == self.task.id => {
                self.task = task;
                self.notice = Some("取消请求已发送；正在等待任务安全结束。".to_owned());
                self.error = None;
            }
            Ok(_) => {
                self.cancel_requested = false;
                self.error = Some("后台返回了其他任务的取消结果。".to_owned());
            }
            Err(error) => {
                self.cancel_requested = false;
                self.error = Some(match error.next_action {
                    Some(action) => format!("{} · 下一步：{}", error.reason, action),
                    None => error.reason,
                });
            }
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
        if !self.can_cancel() {
            self.cancel_armed = false;
            self.cancel_requested = false;
        }
        let old_len = self.events.len();
        let mut incoming = update.events;
        incoming.sort_by_key(|event| event.sequence);
        for event in incoming {
            if event.task_id == self.task.id && event.sequence > self.last_event_sequence {
                self.last_event_sequence = event.sequence;
                self.events.push(event);
            }
        }
        if self.scroll_offset > 0 {
            self.scroll_offset += self.events.len().saturating_sub(old_len);
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
