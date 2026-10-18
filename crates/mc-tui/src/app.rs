mod backup;
mod instance;
mod modpack;

use std::path::PathBuf;

use mc_core::{AppError, InstanceSummary, TaskSnapshot};

use crate::{
    backup_manager::{BackupCommand, BackupManagerState},
    command_palette::{CommandId, CommandPaletteState},
    instance_detail::{InstanceCommand, InstanceDetailState},
    modpack_import::ModpackImportState,
    task_detail::{TaskDetailQuery, TaskDetailState, TaskDetailUpdate},
    wizard::{CreateInstanceInput, CreateWizard, WizardAction, WizardOutcome},
};

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
    OpenTaskDetail,
    BackFromTaskDetail,
    RefreshTaskDetail,
    CancelTask,
    OpenCommandPalette,
    BackFromCommandPalette,
    OpenModpackImport,
    AnalyzeModpack,
    RefreshModpack,
    BackFromModpack,
    PalettePrevious,
    PaletteNext,
    PaletteInsert(char),
    PaletteBackspace,
    PaletteSubmit,
    OpenInstanceDetail,
    BackFromInstanceDetail,
    RefreshInstanceDetail,
    StartServer,
    StopServer,
    CreateBackup,
    OpenBackupManager,
    BackFromBackups,
    RefreshBackups,
    VerifyBackup,
    PreviewRestore,
    LoadMoreBackups,
    BeginRestoreConfirmation,
    RestoreConfirmInsert(char),
    RestoreConfirmBackspace,
    RestoreConfirmSubmit,
    RestoreConfirmCancel,
    BeginConsoleInput,
    ConsoleInsert(char),
    ConsoleBackspace,
    ConsoleSubmit,
    ConsoleCancel,
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
    pub task_detail: Option<TaskDetailState>,
    pub task_detail_visible: bool,
    pub instance_detail: Option<InstanceDetailState>,
    pub instance_detail_id: Option<uuid::Uuid>,
    pub instance_detail_visible: bool,
    pub backup_manager: Option<BackupManagerState>,
    pub backup_manager_visible: bool,
    pub command_palette: Option<CommandPaletteState>,
    pub modpack_import: Option<ModpackImportState>,
    pub modpack_import_visible: bool,
    refresh_requested: bool,
    create_requested: Option<CreateInstanceInput>,
    create_options_requested: bool,
    task_detail_refresh_requested: bool,
    instance_detail_refresh_requested: bool,
    instance_command_requested: Option<InstanceCommand>,
    backup_command_requested: Option<BackupCommand>,
    task_cancel_requested: Option<uuid::Uuid>,
    modpack_analysis_requested: Option<PathBuf>,
    instance_operation_pending: bool,
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
            task_detail: None,
            task_detail_visible: false,
            instance_detail: None,
            instance_detail_id: None,
            instance_detail_visible: false,
            backup_manager: None,
            backup_manager_visible: false,
            command_palette: None,
            modpack_import: None,
            modpack_import_visible: false,
            refresh_requested: false,
            create_requested: None,
            create_options_requested: false,
            task_detail_refresh_requested: false,
            instance_detail_refresh_requested: false,
            instance_command_requested: None,
            backup_command_requested: None,
            task_cancel_requested: None,
            modpack_analysis_requested: None,
            instance_operation_pending: false,
            should_quit: false,
        }
    }
}

impl AppState {
    pub fn update(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_quit = true,
            Action::Previous => {
                if self.modpack_import_visible {
                    if let Some(import) = self.modpack_import.as_mut() {
                        import.select_previous();
                    }
                } else if self.backup_manager_visible {
                    if let Some(manager) = self.backup_manager.as_mut() {
                        manager.select_previous();
                    }
                } else if self.task_detail_visible {
                    if let Some(detail) = self.task_detail.as_mut() {
                        detail.scroll_older();
                    }
                } else if !self.instance_detail_visible {
                    self.move_selection(false);
                }
            }
            Action::Next => {
                if self.modpack_import_visible {
                    if let Some(import) = self.modpack_import.as_mut() {
                        import.select_next();
                    }
                } else if self.backup_manager_visible {
                    if let Some(manager) = self.backup_manager.as_mut() {
                        manager.select_next();
                    }
                } else if self.task_detail_visible {
                    if let Some(detail) = self.task_detail.as_mut() {
                        detail.scroll_newer();
                    }
                } else if !self.instance_detail_visible {
                    self.move_selection(true);
                }
            }
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
            Action::OpenTaskDetail => self.open_selected_task_detail(),
            Action::BackFromTaskDetail => {
                self.task_detail_visible = false;
                self.task_detail_refresh_requested = false;
            }
            Action::CancelTask => self.request_task_cancel(),
            Action::OpenCommandPalette => self.open_command_palette(),
            Action::BackFromCommandPalette => self.command_palette = None,
            Action::PalettePrevious => self.palette_previous(),
            Action::PaletteNext => self.palette_next(),
            Action::PaletteInsert(character) => self.palette_insert(character),
            Action::PaletteBackspace => self.palette_backspace(),
            Action::PaletteSubmit => self.submit_command_palette(),
            Action::OpenModpackImport => self.open_modpack_import(),
            Action::AnalyzeModpack => self.analyze_selected_modpack(),
            Action::RefreshModpack => self.refresh_modpack_import(),
            Action::BackFromModpack => self.close_modpack_import(),
            Action::RefreshTaskDetail => {
                if self.task_detail_visible {
                    self.task_detail_refresh_requested = true;
                }
            }
            Action::OpenInstanceDetail => self.open_selected_instance_detail(),
            Action::BackFromInstanceDetail => self.close_instance_detail(),
            Action::RefreshInstanceDetail => self.request_instance_detail_refresh(),
            Action::StartServer => self.queue_server_command(false),
            Action::StopServer => self.queue_server_command(true),
            Action::CreateBackup => self.queue_backup(),
            Action::OpenBackupManager => self.open_backup_manager(),
            Action::BackFromBackups => self.close_backup_manager(),
            Action::RefreshBackups => self.queue_backup_list(None),
            Action::VerifyBackup => self.queue_backup_verify(),
            Action::PreviewRestore => self.queue_backup_restore_plan(),
            Action::LoadMoreBackups => self.queue_backup_more(),
            Action::BeginRestoreConfirmation => self.begin_restore_confirmation(),
            Action::RestoreConfirmInsert(character) => self.insert_restore_confirmation(character),
            Action::RestoreConfirmBackspace => self.backspace_restore_confirmation(),
            Action::RestoreConfirmSubmit => self.submit_restore_confirmation(),
            Action::RestoreConfirmCancel => self.cancel_restore_confirmation(),
            Action::BeginConsoleInput => self.begin_console_input(),
            Action::ConsoleInsert(character) => self.insert_console_character(character),
            Action::ConsoleBackspace => self.backspace_console(),
            Action::ConsoleCancel => self.cancel_console_input(),
            Action::ConsoleSubmit => self.submit_console_command(),
            Action::OpenCreate => {
                self.wizard = Some(CreateWizard::default());
                self.create_options_requested = true;
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

    pub fn apply_task_detail_result(&mut self, result: Result<TaskDetailUpdate, AppError>) {
        let Some(detail) = self.task_detail.as_mut() else {
            return;
        };
        detail.apply_result(result);
        if let Some(error) = &detail.error {
            self.status = format!("任务详情刷新失败：{error}");
        } else {
            self.status = format!("任务事件游标：{}", detail.last_event_sequence);
        }
    }

    pub fn take_refresh_request(&mut self) -> bool {
        std::mem::take(&mut self.refresh_requested)
    }

    pub fn take_create_options_request(&mut self) -> bool {
        std::mem::take(&mut self.create_options_requested)
    }

    pub fn apply_create_options(&mut self, result: Result<crate::wizard::CreateOptions, AppError>) {
        let Some(wizard) = self.wizard.as_mut() else {
            return;
        };
        match result {
            Ok(options) => {
                wizard.apply_options(options);
                self.status = "已读取 Paper 稳定版本和本机 Java 运行时。".to_owned();
            }
            Err(error) => {
                let message = match error.next_action {
                    Some(action) => format!("{} · 下一步：{}", error.reason, action),
                    None => error.reason,
                };
                wizard.set_options_error(message.clone());
                self.status = format!("创建选项读取失败：{message}");
            }
        }
    }

    pub fn take_create_request(&mut self) -> Option<CreateInstanceInput> {
        self.create_requested.take()
    }

    pub fn take_task_detail_query(&mut self) -> Option<TaskDetailQuery> {
        if !self.task_detail_visible || !std::mem::take(&mut self.task_detail_refresh_requested) {
            return None;
        }
        self.task_detail.as_ref().map(TaskDetailState::query)
    }

    fn open_command_palette(&mut self) {
        self.command_palette = Some(CommandPaletteState::for_state(self));
    }

    fn palette_previous(&mut self) {
        if let Some(palette) = self.command_palette.as_mut() {
            palette.select_previous();
        }
    }

    fn palette_next(&mut self) {
        if let Some(palette) = self.command_palette.as_mut() {
            palette.select_next();
        }
    }

    fn palette_insert(&mut self, character: char) {
        if let Some(palette) = self.command_palette.as_mut() {
            palette.insert(character);
        }
    }

    fn palette_backspace(&mut self) {
        if let Some(palette) = self.command_palette.as_mut() {
            palette.backspace();
        }
    }

    fn submit_command_palette(&mut self) {
        let Some(command) = self
            .command_palette
            .as_ref()
            .and_then(CommandPaletteState::selected_entry)
            .map(|entry| entry.id)
        else {
            return;
        };
        self.command_palette = None;
        match command {
            CommandId::OpenCreate => self.update(Action::OpenCreate),
            CommandId::ImportModpack => self.update(Action::OpenModpackImport),
            CommandId::Refresh => {
                if self.task_detail_visible {
                    self.update(Action::RefreshTaskDetail);
                } else if self.backup_manager_visible {
                    self.update(Action::RefreshBackups);
                } else if self.instance_detail_visible {
                    self.update(Action::RefreshInstanceDetail);
                } else {
                    self.update(Action::Refresh);
                }
            }
            CommandId::TogglePanel => self.update(Action::TogglePanel),
            CommandId::OpenBackupManager => self.update(Action::OpenBackupManager),
            CommandId::StartServer => self.update(Action::StartServer),
            CommandId::StopServer => self.update(Action::StopServer),
            CommandId::CreateBackup => self.update(Action::CreateBackup),
            CommandId::VerifyBackup => self.update(Action::VerifyBackup),
            CommandId::PreviewRestore => self.update(Action::PreviewRestore),
            CommandId::LoadMoreBackups => self.update(Action::LoadMoreBackups),
            CommandId::CancelTask => self.update(Action::CancelTask),
            CommandId::Back => {
                if self.task_detail_visible {
                    self.update(Action::BackFromTaskDetail);
                } else if self.backup_manager_visible {
                    self.update(Action::BackFromBackups);
                } else if self.instance_detail_visible {
                    self.update(Action::BackFromInstanceDetail);
                }
            }
            CommandId::Quit => self.update(Action::Quit),
        }
    }

    fn request_task_cancel(&mut self) {
        let Some(detail) = self.task_detail.as_mut() else {
            return;
        };
        if let Some(task_id) = detail.request_cancel() {
            self.task_cancel_requested = Some(task_id);
        }
        self.status = detail
            .notice
            .clone()
            .or_else(|| detail.error.clone())
            .unwrap_or_else(|| "当前任务不可取消。".to_owned());
    }

    pub fn take_task_cancel_request(&mut self) -> Option<uuid::Uuid> {
        self.task_cancel_requested.take()
    }

    pub fn apply_task_cancel_result(&mut self, result: Result<TaskSnapshot, AppError>) {
        let Some(detail) = self.task_detail.as_mut() else {
            return;
        };
        detail.apply_cancel_result(result);
        if let Some(error) = &detail.error {
            self.status = format!("取消失败：{error}");
        } else {
            self.status = detail
                .notice
                .clone()
                .unwrap_or_else(|| "取消请求已发送。".to_owned());
            self.task_detail_refresh_requested = true;
        }
    }

    pub fn task_detail_should_poll_fast(&self) -> bool {
        self.task_detail_visible
            && self.task_detail.as_ref().is_some_and(|detail| {
                detail.task.kind == mc_core::TaskKind::CreatePaperInstance
                    && (detail.task.status == mc_core::TaskStatus::Queued
                        || detail.task.status == mc_core::TaskStatus::Running)
            })
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn has_animated_work(&self) -> bool {
        self.instance_operation_pending
            || self
                .wizard
                .as_ref()
                .is_some_and(|wizard| wizard.options_loading)
            || (self.backup_manager_visible
                && self
                    .backup_manager
                    .as_ref()
                    .is_some_and(BackupManagerState::is_busy))
            || self.wizard.as_ref().is_some_and(|wizard| wizard.submitting)
            || self.tasks.iter().any(|task| {
                matches!(
                    task.status,
                    mc_core::TaskStatus::Queued | mc_core::TaskStatus::Running
                )
            })
            || (self.task_detail_visible
                && self.task_detail.as_ref().is_some_and(|detail| {
                    matches!(
                        detail.task.status,
                        mc_core::TaskStatus::Queued | mc_core::TaskStatus::Running
                    )
                }))
            || (self.instance_detail_visible
                && self
                    .instance_detail
                    .as_ref()
                    .and_then(|detail| detail.detail.as_ref())
                    .is_some_and(|detail| {
                        matches!(
                            detail.server.state,
                            mc_core::ServerState::Starting | mc_core::ServerState::Stopping
                        )
                    }))
    }

    fn open_selected_task_detail(&mut self) {
        if self.focused_panel != Panel::Tasks {
            self.status = "请先切换到后台任务面板。".to_owned();
            return;
        }
        let Some(task) = self.tasks.get(self.selected_task).cloned() else {
            self.status = "当前没有可查看的后台任务。".to_owned();
            return;
        };
        match self.task_detail.as_mut() {
            Some(detail) if detail.task.id == task.id => detail.task = task,
            _ => self.task_detail = Some(TaskDetailState::new(task)),
        }
        self.task_detail_visible = true;
        self.task_detail_refresh_requested = true;
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
mod tests;
