use mc_core::{AppError, InstanceDetail, ServerLogChunk, ServerSnapshot, TaskSnapshot};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InstanceDetailQuery {
    pub instance_id: Uuid,
    pub after_offset: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ProcessStats {
    pub cpu_percent: f64,
    pub rss_bytes: u64,
}

#[derive(Debug)]
pub(crate) struct InstanceDetailUpdate {
    pub detail: InstanceDetail,
    pub logs: ServerLogChunk,
    pub stats: Option<ProcessStats>,
}

#[derive(Debug)]
pub(crate) enum InstanceCommand {
    Manage {
        instance_id: Uuid,
        action: mc_core::InstanceOperation,
    },
    Start(Uuid),
    Stop(Uuid),
    Backup(Uuid),
    Console {
        instance_id: Uuid,
        command: String,
    },
}

#[derive(Debug)]
pub(crate) enum InstanceCommandResult {
    Management(Result<Box<mc_core::ApiResponse>, AppError>),
    Status(Result<ServerSnapshot, AppError>),
    Console(Result<(), AppError>),
    Task(Result<Box<TaskSnapshot>, AppError>),
}

#[derive(Debug)]
pub(crate) struct InstanceDetailState {
    pub instance_id: Uuid,
    pub detail: Option<InstanceDetail>,
    pub logs: String,
    pub log_offset: u64,
    pub stats: Option<ProcessStats>,
    pub error: Option<String>,
    pub notice: Option<String>,
    pub console_input: String,
    pub editing_console: bool,
    /// Number of log lines scrolled up from the tail. Zero means follow.
    pub log_scroll: usize,
    pub log_max_scroll: std::cell::Cell<usize>,
    pub log_horizontal: usize,
    pub log_filter: String,
    pub editing_filter: bool,
    pub cpu_high_samples: u8,
    pub rss_high_samples: u8,
    pub recent_error: Option<std::time::Instant>,
    pub cpu_history: std::collections::VecDeque<u64>,
    pub rss_history: std::collections::VecDeque<u64>,
}

impl InstanceDetailState {
    pub fn loading(instance_id: Uuid) -> Self {
        Self {
            instance_id,
            detail: None,
            logs: String::new(),
            log_offset: 0,
            stats: None,
            error: None,
            notice: None,
            console_input: String::new(),
            editing_console: false,
            log_scroll: 0,
            log_max_scroll: Default::default(),
            log_horizontal: 0,
            log_filter: String::new(),
            editing_filter: false,
            cpu_high_samples: 0,
            rss_high_samples: 0,
            recent_error: None,
            cpu_history: Default::default(),
            rss_history: Default::default(),
        }
    }

    pub fn alerts(&self) -> Vec<String> {
        let mut alerts = Vec::new();
        if self
            .detail
            .as_ref()
            .is_some_and(|v| v.server.state == mc_core::ServerState::Failed)
        {
            alerts.push("服务器异常退出；检查下方 ERROR 日志后重试。".to_owned());
        }
        let running = self
            .detail
            .as_ref()
            .is_some_and(|v| v.server.state == mc_core::ServerState::Running);
        if running && self.cpu_high_samples >= 6 {
            alerts.push(
                "CPU 持续达到单核 90%（进程占用，约 6 次采样）；检查 TPS / spark。".to_owned(),
            );
        }
        if running && self.rss_high_samples >= 3 {
            alerts.push(
                "进程 RSS 已超出配置的堆上限；RSS 含 JVM 堆外内存，请关注系统内存。".to_owned(),
            );
        }
        if self
            .recent_error
            .is_some_and(|at| at.elapsed().as_secs() < 60)
        {
            alerts.push(
                "最近 60 秒出现 ERROR / Exception；按 f 搜索或 /rg -e ERROR|Exception。".to_owned(),
            );
        }
        alerts
    }

    pub fn scroll_logs(&mut self, delta: isize) {
        let next = self.log_scroll as isize + delta;
        self.log_scroll = (next.max(0) as usize).min(self.log_max_scroll.get());
    }

    pub fn scroll_to_oldest(&mut self) {
        self.log_scroll = self.log_max_scroll.get();
    }

    pub fn scroll_to_latest(&mut self) {
        self.log_scroll = 0;
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
        if self
            .detail
            .as_ref()
            .is_some_and(|old| old.server.state != update.detail.server.state)
        {
            self.notice = None;
        }
        self.detail = Some(update.detail);
        if self.log_scroll > 0
            && !update.logs.truncated
            && update.logs.start_offset == self.log_offset
        {
            self.log_scroll = self
                .log_scroll
                .saturating_add(update.logs.content.lines().count());
        }
        if self.log_offset > 0
            && (update.logs.content.contains("ERROR") || update.logs.content.contains("Exception"))
        {
            self.recent_error = Some(std::time::Instant::now());
        }
        if update.logs.truncated || update.logs.start_offset != self.log_offset {
            self.logs = update.logs.content;
        } else {
            self.logs.push_str(&update.logs.content);
        }
        self.log_offset = update.logs.next_offset;
        self.stats = update.stats;
        if let Some(stats) = self.stats {
            self.cpu_high_samples = if stats.cpu_percent >= 90.0 {
                self.cpu_high_samples.saturating_add(1)
            } else {
                0
            };
            let memory = self
                .detail
                .as_ref()
                .map(|v| u64::from(v.instance.max_memory_mb) * 1_048_576)
                .unwrap_or(u64::MAX);
            self.rss_high_samples = if stats.rss_bytes > memory {
                self.rss_high_samples.saturating_add(1)
            } else {
                0
            };
            self.cpu_history
                .push_back(stats.cpu_percent.max(0.0) as u64);
            self.rss_history.push_back(stats.rss_bytes / 1_048_576);
            while self.cpu_history.len() > 60 {
                self.cpu_history.pop_front();
                self.rss_history.pop_front();
            }
        }
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
        if self.editing_filter {
            if !character.is_control() && self.log_filter.len() < 256 {
                self.log_filter.push(character);
                self.log_scroll = 0;
            }
            return;
        }
        if !character.is_control() && self.console_input.len() + character.len_utf8() <= 512 {
            self.console_input.push(character);
        }
    }

    pub fn backspace_console(&mut self) {
        if self.editing_filter {
            self.log_filter.pop();
            return;
        }
        self.console_input.pop();
    }

    pub fn cancel_console_input(&mut self) {
        self.console_input.clear();
        self.editing_console = false;
        if self.editing_filter {
            self.log_filter.clear();
            self.editing_filter = false;
        }
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
mod tests;
