use mc_core::{AppError, InstanceSummary, TaskSnapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Panel {
    Instances,
    Tasks,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    Quit,
    Previous,
    Next,
    TogglePanel,
    Refresh,
    Ignore,
}

#[derive(Debug, Default)]
pub(crate) struct DashboardData {
    pub instances: Vec<InstanceSummary>,
    pub tasks: Vec<TaskSnapshot>,
}

#[derive(Debug)]
pub(crate) struct AppState {
    pub instances: Vec<InstanceSummary>,
    pub tasks: Vec<TaskSnapshot>,
    pub focused_panel: Panel,
    pub selected_instance: usize,
    pub selected_task: usize,
    pub status: String,
    refresh_requested: bool,
    should_quit: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            instances: Vec::new(),
            tasks: Vec::new(),
            focused_panel: Panel::Instances,
            selected_instance: 0,
            selected_task: 0,
            status: "正在连接后台服务…".to_owned(),
            refresh_requested: false,
            should_quit: false,
        }
    }
}

impl AppState {
    pub fn update(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_quit = true,
            Action::Previous => self.move_selection(false),
            Action::Next => self.move_selection(true),
            Action::TogglePanel => {
                self.focused_panel = match self.focused_panel {
                    Panel::Instances => Panel::Tasks,
                    Panel::Tasks => Panel::Instances,
                };
            }
            Action::Refresh => {
                self.refresh_requested = true;
                self.status = "正在刷新…".to_owned();
            }
            Action::Ignore => {}
        }
    }

    pub fn apply_refresh(&mut self, result: Result<DashboardData, AppError>) {
        match result {
            Ok(data) => {
                self.instances = data.instances;
                self.tasks = data.tasks;
                self.selected_instance = self
                    .selected_instance
                    .min(self.instances.len().saturating_sub(1));
                self.selected_task = self.selected_task.min(self.tasks.len().saturating_sub(1));
                self.status = "已连接 · 自动刷新中".to_owned();
            }
            Err(error) => {
                self.status = format!("连接失败：{}", error.reason);
            }
        }
    }

    pub fn take_refresh_request(&mut self) -> bool {
        std::mem::take(&mut self.refresh_requested)
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    fn move_selection(&mut self, forward: bool) {
        let (selection, length) = match self.focused_panel {
            Panel::Instances => (&mut self.selected_instance, self.instances.len()),
            Panel::Tasks => (&mut self.selected_task, self.tasks.len()),
        };
        if length == 0 {
            *selection = 0;
        } else if forward {
            *selection = (*selection + 1).min(length - 1);
        } else {
            *selection = selection.saturating_sub(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use mc_core::{ErrorCode, InstanceSummary};
    use uuid::Uuid;

    use super::{Action, AppState, DashboardData};

    #[test]
    fn failed_refresh_keeps_the_last_successful_frame_data() {
        let mut state = AppState::default();
        state.apply_refresh(Ok(DashboardData {
            instances: vec![InstanceSummary {
                id: Uuid::new_v4(),
                name: "生存服".to_owned(),
                provider: "Paper".to_owned(),
                game_version: "1.21.11".to_owned(),
                build_id: 17,
                java_path: "/opt/java/bin/java".into(),
                min_memory_mb: 1024,
                max_memory_mb: 4096,
                port: 25565,
                directory: "/tmp/server".into(),
                created_at: Utc::now(),
            }],
            tasks: Vec::new(),
        }));
        state.update(Action::Next);
        state.apply_refresh(Err(mc_core::AppError::new(
            ErrorCode::SourceUnavailable,
            "daemon temporarily unavailable",
        )));

        assert_eq!(state.instances.len(), 1);
        assert_eq!(state.instances[0].name, "生存服");
        assert_eq!(state.selected_instance, 0);
        assert!(state.status.starts_with("连接失败"));
    }
}
