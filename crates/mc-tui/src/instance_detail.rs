use mc_core::{AppError, InstanceDetail, ServerLogChunk, ServerSnapshot};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InstanceDetailQuery {
    pub instance_id: Uuid,
    pub after_offset: u64,
}

#[derive(Debug)]
pub(crate) struct InstanceDetailUpdate {
    pub detail: InstanceDetail,
    pub logs: ServerLogChunk,
}

#[derive(Debug)]
pub(crate) enum InstanceCommand {
    Start(Uuid),
    Stop(Uuid),
    Console { instance_id: Uuid, command: String },
}

#[derive(Debug)]
pub(crate) enum InstanceCommandResult {
    Status(Result<ServerSnapshot, AppError>),
    Console(Result<(), AppError>),
}

#[derive(Debug)]
pub(crate) struct InstanceDetailState {
    pub instance_id: Uuid,
    pub detail: Option<InstanceDetail>,
    pub logs: String,
    pub log_offset: u64,
    pub error: Option<String>,
    pub notice: Option<String>,
    pub console_input: String,
    pub editing_console: bool,
}

impl InstanceDetailState {
    pub fn loading(instance_id: Uuid) -> Self {
        Self {
            instance_id,
            detail: None,
            logs: String::new(),
            log_offset: 0,
            error: None,
            notice: None,
            console_input: String::new(),
            editing_console: false,
        }
    }

    pub fn query(&self) -> InstanceDetailQuery {
        InstanceDetailQuery {
            instance_id: self.instance_id,
            after_offset: self.log_offset,
        }
    }

    pub fn apply_update(&mut self, result: Result<InstanceDetailUpdate, AppError>) {
        let update = match result {
            Ok(update) => update,
            Err(error) => {
                self.error = Some(error_message(error));
                return;
            }
        };
        if update.detail.instance.id != self.instance_id
            || update.detail.server.instance_id != self.instance_id
            || update.logs.instance_id != self.instance_id
        {
            self.error = Some("后台返回了其他实例的数据。".to_owned());
            return;
        }
        self.detail = Some(update.detail);
        if update.logs.truncated || update.logs.start_offset != self.log_offset {
            self.logs = update.logs.content;
        } else {
            self.logs.push_str(&update.logs.content);
        }
        self.log_offset = update.logs.next_offset;
        trim_log_buffer(&mut self.logs);
        self.error = None;
    }

    pub fn apply_status(&mut self, result: Result<ServerSnapshot, AppError>) {
        match result {
            Ok(server) if server.instance_id == self.instance_id => {
                if let Some(detail) = &mut self.detail {
                    detail.server = server;
                }
                self.error = None;
            }
            Ok(_) => self.error = Some("后台返回了其他实例的状态。".to_owned()),
            Err(error) => self.error = Some(error_message(error)),
        }
    }

    pub fn begin_console_input(&mut self) {
        self.editing_console = true;
        self.console_input.clear();
    }

    pub fn insert_console_char(&mut self, character: char) {
        if !character.is_control() && self.console_input.len() + character.len_utf8() <= 512 {
            self.console_input.push(character);
        }
    }

    pub fn backspace_console(&mut self) {
        self.console_input.pop();
    }

    pub fn cancel_console_input(&mut self) {
        self.console_input.clear();
        self.editing_console = false;
    }

    pub fn take_console_command(&mut self) -> Option<String> {
        let command = self.console_input.trim().to_owned();
        self.cancel_console_input();
        (!command.is_empty()).then_some(command)
    }
}

fn error_message(error: AppError) -> String {
    match error.next_action {
        Some(action) => format!("{} · 下一步：{action}", error.reason),
        None => error.reason,
    }
}

fn trim_log_buffer(logs: &mut String) {
    const MAX_RETAINED_LOG_BYTES: usize = 64 * 1024;
    if logs.len() <= MAX_RETAINED_LOG_BYTES {
        return;
    }
    let mut start = logs.len() - MAX_RETAINED_LOG_BYTES;
    while !logs.is_char_boundary(start) {
        start += 1;
    }
    if let Some(line_end) = logs[start..].find('\n') {
        start += line_end + 1;
    }
    logs.drain(..start);
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use mc_core::{InstanceSummary, ServerLogChunk, ServerSnapshot, ServerState};

    use super::{InstanceDetailState, InstanceDetailUpdate};

    #[test]
    fn log_cursor_appends_incrementally_and_replaces_after_truncation() {
        let instance_id = uuid::Uuid::new_v4();
        let mut state = InstanceDetailState::loading(instance_id);
        state.apply_update(Ok(update(instance_id, 0, 3, "one\n", false)));
        assert_eq!(state.log_offset, 3);
        state.apply_update(Ok(update(instance_id, 3, 7, "two\n", false)));
        assert_eq!(state.logs, "one\ntwo\n");
        state.apply_update(Ok(update(instance_id, 10, 13, "new\n", true)));
        assert_eq!(state.logs, "new\n");
        assert_eq!(state.log_offset, 13);
    }

    #[test]
    fn console_input_rejects_controls_and_caps_length() {
        let mut state = InstanceDetailState::loading(uuid::Uuid::new_v4());
        state.begin_console_input();
        state.insert_console_char('a');
        state.insert_console_char('\n');
        for _ in 0..600 {
            state.insert_console_char('x');
        }
        assert_eq!(state.console_input.len(), 512);
        assert_eq!(state.take_console_command().unwrap().len(), 512);
        assert!(!state.editing_console);
    }

    fn update(
        instance_id: uuid::Uuid,
        start_offset: u64,
        next_offset: u64,
        content: &str,
        truncated: bool,
    ) -> InstanceDetailUpdate {
        let instance = InstanceSummary {
            id: instance_id,
            name: "test".to_owned(),
            provider: "Paper".to_owned(),
            game_version: "1.21.11".to_owned(),
            build_id: 1,
            java_path: "/usr/bin/java".into(),
            min_memory_mb: 512,
            max_memory_mb: 1024,
            port: 25565,
            directory: "/tmp/test".into(),
            created_at: Utc::now(),
        };
        InstanceDetailUpdate {
            detail: mc_core::InstanceDetail {
                instance,
                server: ServerSnapshot {
                    instance_id,
                    state: ServerState::Running,
                    pid: Some(123),
                    port: 25565,
                    address: "127.0.0.1:25565".to_owned(),
                    started_at: Some(Utc::now()),
                    last_exit_code: None,
                    message: "Ready".to_owned(),
                },
            },
            logs: ServerLogChunk {
                instance_id,
                start_offset,
                next_offset,
                content: content.to_owned(),
                truncated,
            },
        }
    }
}
