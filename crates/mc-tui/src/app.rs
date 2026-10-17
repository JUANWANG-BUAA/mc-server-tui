use mc_core::{AppError, InstanceSummary, TaskSnapshot};

use crate::wizard::{CreateInstanceInput, CreateWizard, WizardAction, WizardOutcome};

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
    OpenCreate,
    Wizard(WizardAction),
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
    pub wizard: Option<CreateWizard>,
    refresh_requested: bool,
    create_requested: Option<CreateInstanceInput>,
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
            wizard: None,
            refresh_requested: false,
            create_requested: None,
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
            Action::OpenCreate => {
                self.wizard = Some(CreateWizard::default());
            }
            Action::Wizard(action) => {
                let outcome = self
                    .wizard
                    .as_mut()
                    .map(|wizard| wizard.handle(action))
                    .unwrap_or(WizardOutcome::None);
                match outcome {
                    WizardOutcome::None => {}
                    WizardOutcome::Cancelled => {
                        self.wizard = None;
                    }
                    WizardOutcome::Submit(input) => {
                        self.create_requested = Some(input);
                        self.status = "正在提交创建任务…".to_owned();
                    }
                }
            }
            Action::Ignore => {}
        }
    }

    pub fn apply_create_result(&mut self, result: Result<TaskSnapshot, AppError>) {
        let Some(wizard) = self.wizard.as_mut() else {
            return;
        };
        match result {
            Ok(task) => {
                self.tasks.insert(0, task);
                self.selected_task = 0;
                self.wizard = None;
                self.status = "创建任务已加入后台队列。".to_owned();
                self.refresh_requested = true;
            }
            Err(error) => {
                let message = match error.next_action {
                    Some(action) => format!("{} · 下一步：{}", error.reason, action),
                    None => error.reason.clone(),
                };
                wizard.set_submit_error(message);
                self.status = format!("提交失败 [{}]：{}", error.code, error.reason);
            }
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

    pub fn take_create_request(&mut self) -> Option<CreateInstanceInput> {
        self.create_requested.take()
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
    use mc_core::{ErrorCode, InstanceSummary, TaskKind, TaskPhase, TaskSnapshot, TaskStatus};
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

    #[test]
    fn create_failure_keeps_wizard_for_correction_and_retry() {
        let mut state = AppState::default();
        state.update(Action::OpenCreate);
        state.apply_create_result(Err(mc_core::AppError::new(
            ErrorCode::SourceUnavailable,
            "daemon temporarily unavailable",
        )));

        assert!(state.wizard.is_some());
        assert_eq!(
            state
                .wizard
                .as_ref()
                .and_then(|wizard| wizard.error.as_deref()),
            Some("daemon temporarily unavailable")
        );
    }

    #[test]
    fn successful_create_returns_to_dashboard_and_requests_refresh() {
        let mut state = AppState::default();
        state.update(Action::OpenCreate);
        state.apply_create_result(Ok(TaskSnapshot {
            id: Uuid::new_v4(),
            kind: TaskKind::CreatePaperInstance,
            instance_id: None,
            status: TaskStatus::Queued,
            phase: TaskPhase::Queued,
            message: "Instance creation queued".to_owned(),
            progress: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            finished_at: None,
            result: None,
            error: None,
            last_event_sequence: 0,
        }));

        assert!(state.wizard.is_none());
        assert_eq!(state.tasks.len(), 1);
        assert!(state.take_refresh_request());
        assert!(state.status.contains("后台队列"));
    }
}
