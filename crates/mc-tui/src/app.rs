mod backup;
mod instance;
mod modpack;
mod shell;

use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    time::{Duration, Instant},
};

use mc_core::{AppError, InstanceSummary, TaskSnapshot};

use crate::{
    backup_manager::{BackupCommand, BackupManagerState},
    command_palette::{CommandId, CommandPaletteState},
    hit::{HitMap, HitTarget},
    instance_detail::{InstanceCommand, InstanceDetailState},
    modpack_import::ModpackImportState,
    task_detail::{TaskDetailQuery, TaskDetailState, TaskDetailUpdate},
    transcript::TranscriptEntry,
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
    PagePrevious,
    PageNext,
    First,
    Last,
    ScrollUp,
    ScrollDown,
    TogglePanel,
    FocusInstances,
    FocusTasks,
    GoDashboard,
    ToggleHelp,
    CloseHelp,
    ShowShell,
    ShowClassic,
    CycleTheme,
    SetTheme(crate::theme::ThemeId),
    ShellInsert(char),
    ShellBackspace,
    ShellDelete,
    ShellCursorLeft,
    ShellCursorRight,
    ShellCursorHome,
    ShellCursorEnd,
    ShellSubmit,
    ShellComplete,
    ShellUp,
    ShellDown,
    ShellEscape,
    ShellClear,
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
    CancelWizard,
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
    pub help_visible: bool,
    pub help_scroll: usize,
    pub help_max_scroll: Cell<usize>,
    pub shell_visible: bool,
    pub shell_input: String,
    pub shell_cursor: usize,
    pub shell_viewport: Cell<usize>,
    pub shell_history: Vec<String>,
    pub shell_history_pos: Option<usize>,
    pub shell_suggestion_index: usize,
    pub shell_scroll: usize,
    pub shell_max_scroll: Cell<usize>,
    pub transcript: Vec<TranscriptEntry>,
    pub hits: RefCell<HitMap>,
    last_click: Option<(Instant, HitTarget)>,
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
            help_visible: false,
            help_scroll: 0,
            help_max_scroll: Cell::new(0),
            shell_visible: true,
            shell_input: String::new(),
            shell_cursor: 0,
            shell_viewport: Cell::new(0),
            shell_history: Vec::new(),
            shell_history_pos: None,
            shell_suggestion_index: 0,
            shell_scroll: 0,
            shell_max_scroll: Cell::new(0),
            transcript: vec![TranscriptEntry::assistant(crate::shell::welcome())],
            hits: RefCell::new(HitMap::default()),
            last_click: None,
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
            Action::Previous => self.step_selection(false),
            Action::Next => self.step_selection(true),
            Action::PagePrevious => self.step_selection_by(-5),
            Action::PageNext => self.step_selection_by(5),
            Action::First => self.jump_selection(false),
            Action::Last => self.jump_selection(true),
            Action::ScrollUp => self.step_selection(false),
            Action::ScrollDown => self.step_selection(true),
            Action::TogglePanel => {
                self.focused_panel = match self.focused_panel {
                    Panel::Instances => Panel::Tasks,
                    Panel::Tasks => Panel::Instances,
                };
            }
            Action::FocusInstances => self.focused_panel = Panel::Instances,
            Action::FocusTasks => self.focused_panel = Panel::Tasks,
            Action::GoDashboard => self.go_dashboard(),
            Action::ToggleHelp => self.toggle_help(),
            Action::CloseHelp => self.help_visible = false,
            Action::ShowShell => self.show_shell(),
            Action::ShowClassic => self.show_classic(),
            Action::ShellInsert(character) => self.shell_insert(character),
            Action::ShellBackspace => self.shell_backspace(),
            Action::ShellDelete => self.shell_delete(),
            Action::ShellCursorLeft => self.shell_cursor_left(),
            Action::ShellCursorRight => self.shell_cursor_right(),
            Action::ShellCursorHome => self.shell_cursor_home(),
            Action::ShellCursorEnd => self.shell_cursor_end(),
            Action::ShellSubmit => self.shell_submit(),
            Action::ShellComplete => self.shell_complete(),
            Action::ShellUp => self.shell_up(),
            Action::ShellDown => self.shell_down(),
            Action::ShellEscape => self.shell_escape(),
            Action::ShellClear => self.shell_clear(),
            Action::CycleTheme => {
                let theme = crate::theme::cycle_theme();
                self.apply_theme_notice(theme);
            }
            Action::SetTheme(theme) => {
                crate::theme::set_theme(theme);
                self.apply_theme_notice(theme);
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
            Action::CancelWizard => {
                self.wizard = None;
                self.status = "已取消创建向导。".to_owned();
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
        match result {
            Ok(task) => {
                if self.wizard.is_none() {
                    return;
                }
                self.tasks.insert(0, task);
                self.selected_task = 0;
                self.wizard = None;
                self.status = "创建任务已加入后台队列。".to_owned();
                self.refresh_requested = true;
                self.push_transcript(TranscriptEntry::success(
                    "创建任务已加入后台队列；下载进度会显示在输入框下方的状态坞。",
                ));
            }
            Err(error) => {
                let Some(wizard) = self.wizard.as_mut() else {
                    return;
                };
                let message = match error.next_action {
                    Some(action) => format!("{} · 下一步：{}", error.reason, action),
                    None => error.reason.clone(),
                };
                wizard.set_submit_error(message);
                self.status = format!("提交失败 [{}]：{}", error.code, error.reason);
                self.push_transcript(TranscriptEntry::error(format!("创建失败：{}", self.status)));
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
                if self.shell_visible {
                    let selected_id = self
                        .instances
                        .get(self.selected_instance)
                        .map(|instance| instance.id);
                    match selected_id {
                        Some(instance_id) => {
                            if self.instance_detail_id != Some(instance_id) {
                                self.select_shell_instance(&instance_id);
                            }
                        }
                        None => {
                            self.instance_detail_id = None;
                            self.instance_detail = None;
                        }
                    }
                }
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
        self.help_visible = false;
        self.command_palette = Some(CommandPaletteState::for_state(self));
    }

    fn apply_theme_notice(&mut self, theme: crate::theme::ThemeId) {
        self.status = format!("主题：{}（{}）", theme.name(), theme.slug());
        self.push_transcript(TranscriptEntry::muted(format!(
            "主题已切换为 {}（{}）；F2 循环，MC_SERVER_TUI_THEME 可设置默认值。",
            theme.name(),
            theme.slug()
        )));
    }

    fn toggle_help(&mut self) {
        self.help_visible = !self.help_visible;
        self.help_scroll = 0;
        if self.help_visible {
            self.command_palette = None;
        }
    }

    fn scroll_help(&mut self, delta: isize) {
        let max = self.help_max_scroll.get() as isize;
        let next = (self.help_scroll as isize + delta).clamp(0, max.max(0));
        self.help_scroll = next as usize;
    }

    fn go_dashboard(&mut self) {
        self.task_detail_visible = false;
        self.instance_detail_visible = false;
        self.backup_manager_visible = false;
        self.modpack_import_visible = false;
        self.wizard = None;
        self.command_palette = None;
        self.help_visible = false;
        self.status = if self.shell_visible {
            "已返回会话终端。".to_owned()
        } else {
            "已返回仪表盘。".to_owned()
        };
    }

    /// Resolve a mouse position through the hit map recorded by the last draw.
    pub fn hit_target_at(&self, column: u16, row: u16) -> Option<HitTarget> {
        self.hits.borrow().target_at(column, row)
    }

    /// Single click selects; a second click on the same target within 450 ms
    /// opens the detail/executes the palette entry.
    pub fn apply_click(&mut self, target: HitTarget) {
        let now = Instant::now();
        let double_click = self.last_click.is_some_and(|(when, previous)| {
            previous == target && now.duration_since(when) <= Duration::from_millis(450)
        });
        self.last_click = Some((now, target));

        match target {
            HitTarget::Instance(index) => {
                self.focused_panel = Panel::Instances;
                if index < self.instances.len() {
                    self.selected_instance = index;
                }
                if double_click {
                    self.open_selected_instance_detail();
                }
            }
            HitTarget::Task(index) => {
                self.focused_panel = Panel::Tasks;
                if index < self.tasks.len() {
                    self.selected_task = index;
                }
                if double_click {
                    self.open_selected_task_detail();
                }
            }
            HitTarget::Version(index) => {
                if let Some(wizard) = self.wizard.as_mut() {
                    wizard.handle(WizardAction::FocusField(1));
                    wizard.handle(WizardAction::SelectChoice(index));
                }
            }
            HitTarget::Java(index) => {
                if let Some(wizard) = self.wizard.as_mut() {
                    wizard.handle(WizardAction::FocusField(0));
                    wizard.handle(WizardAction::SelectChoice(index));
                }
            }
            HitTarget::Memory(index) => {
                if let Some(wizard) = self.wizard.as_mut() {
                    wizard.handle(WizardAction::FocusField(1));
                    wizard.handle(WizardAction::SelectChoice(index));
                }
            }
            HitTarget::WizardField(index) => {
                if let Some(wizard) = self.wizard.as_mut() {
                    wizard.handle(WizardAction::FocusField(index));
                }
            }
            HitTarget::PaletteEntry(index) => {
                if let Some(palette) = self.command_palette.as_mut() {
                    if index < palette.filtered_len() {
                        palette.selected = index;
                    }
                }
                if double_click {
                    self.submit_command_palette();
                }
            }
            HitTarget::Backup(index) => {
                if let Some(manager) = self.backup_manager.as_mut() {
                    if index < manager.backups.len() {
                        manager.selected = index;
                    }
                }
            }
            HitTarget::Modpack(index) => {
                if let Some(import) = self.modpack_import.as_mut() {
                    if index < import.candidates.len() {
                        import.selected = index;
                    }
                }
            }
            HitTarget::ToggleEula => {
                if !double_click {
                    self.update(Action::Wizard(WizardAction::ToggleAcceptance));
                }
            }
            HitTarget::HelpClose => self.help_visible = false,
            HitTarget::ShellSuggestion(index) => {
                self.shell_suggestion_index = index;
                self.shell_complete();
            }
        }
    }

    fn step_selection(&mut self, forward: bool) {
        if self.help_visible {
            self.scroll_help(if forward { 1 } else { -1 });
            return;
        }
        if let Some(palette) = self.command_palette.as_mut() {
            if forward {
                palette.select_next();
            } else {
                palette.select_previous();
            }
            return;
        }
        if self.wizard.is_some() {
            let action = if forward {
                WizardAction::NextChoice
            } else {
                WizardAction::PreviousChoice
            };
            self.update(Action::Wizard(action));
            return;
        }
        if self.modpack_import_visible {
            if let Some(import) = self.modpack_import.as_mut() {
                if forward {
                    import.select_next();
                } else {
                    import.select_previous();
                }
            }
            return;
        }
        if self.backup_manager_visible {
            if let Some(manager) = self.backup_manager.as_mut() {
                if forward {
                    manager.select_next();
                } else {
                    manager.select_previous();
                }
            }
            return;
        }
        if self.task_detail_visible {
            if let Some(detail) = self.task_detail.as_mut() {
                if forward {
                    detail.scroll_newer();
                } else {
                    detail.scroll_older();
                }
            }
            return;
        }
        if self.instance_detail_visible {
            if let Some(detail) = self.instance_detail.as_mut() {
                detail.scroll_logs(if forward { -1 } else { 1 });
            }
            return;
        }
        if self.shell_visible {
            self.scroll_shell(if forward { -3 } else { 3 });
            return;
        }
        self.move_selection(forward);
    }

    fn step_selection_by(&mut self, delta: isize) {
        if self.help_visible {
            self.scroll_help(delta);
            return;
        }
        if self.command_palette.is_some() {
            let count = delta.unsigned_abs();
            for _ in 0..count {
                self.step_selection(delta > 0);
            }
            return;
        }
        if self.wizard.is_some() {
            let action = if delta > 0 {
                WizardAction::PageNext
            } else {
                WizardAction::PagePrevious
            };
            self.update(Action::Wizard(action));
            return;
        }
        if self.modpack_import_visible || self.backup_manager_visible {
            for _ in 0..delta.unsigned_abs() {
                self.step_selection(delta > 0);
            }
            return;
        }
        if self.task_detail_visible {
            for _ in 0..delta.unsigned_abs() {
                self.step_selection(delta > 0);
            }
            return;
        }
        if self.instance_detail_visible {
            if let Some(detail) = self.instance_detail.as_mut() {
                detail.scroll_logs(-delta);
            }
            return;
        }
        if self.shell_visible {
            self.scroll_shell(-delta);
            return;
        }
        self.move_selection_by(delta);
    }

    fn jump_selection(&mut self, last: bool) {
        if self.help_visible {
            self.help_scroll = if last { self.help_max_scroll.get() } else { 0 };
            return;
        }
        if let Some(palette) = self.command_palette.as_mut() {
            palette.select_edge(last);
            return;
        }
        if self.wizard.is_some() {
            let action = if last {
                WizardAction::LastChoice
            } else {
                WizardAction::FirstChoice
            };
            self.update(Action::Wizard(action));
            return;
        }
        if self.modpack_import_visible {
            if let Some(import) = self.modpack_import.as_mut() {
                import.selected = if last {
                    import.candidates.len().saturating_sub(1)
                } else {
                    0
                };
            }
            return;
        }
        if self.backup_manager_visible {
            if let Some(manager) = self.backup_manager.as_mut() {
                manager.selected = if last {
                    manager.backups.len().saturating_sub(1)
                } else {
                    0
                };
            }
            return;
        }
        if self.task_detail_visible {
            if let Some(detail) = self.task_detail.as_mut() {
                detail.scroll_offset = if last {
                    0
                } else {
                    detail.events.len().saturating_sub(1)
                };
            }
            return;
        }
        if self.instance_detail_visible {
            if let Some(detail) = self.instance_detail.as_mut() {
                if last {
                    detail.scroll_to_latest();
                } else {
                    detail.scroll_to_oldest();
                }
            }
            return;
        }
        if self.shell_visible {
            self.jump_shell(last);
            return;
        }
        let length = match self.focused_panel {
            Panel::Instances => self.instances.len(),
            Panel::Tasks => self.tasks.len(),
        };
        let selection = if last { length.saturating_sub(1) } else { 0 };
        match self.focused_panel {
            Panel::Instances => self.selected_instance = selection,
            Panel::Tasks => self.selected_task = selection,
        }
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
            CommandId::OpenSelectedInstance => {
                self.update(Action::FocusInstances);
                self.update(Action::OpenInstanceDetail);
            }
            CommandId::OpenSelectedTask => {
                self.update(Action::FocusTasks);
                self.update(Action::OpenTaskDetail);
            }
            CommandId::FocusInstances => self.update(Action::FocusInstances),
            CommandId::FocusTasks => self.update(Action::FocusTasks),
            CommandId::Refresh => {
                if self.task_detail_visible {
                    self.update(Action::RefreshTaskDetail);
                } else if self.backup_manager_visible {
                    self.update(Action::RefreshBackups);
                } else if self.instance_detail_visible {
                    self.update(Action::RefreshInstanceDetail);
                } else if self.modpack_import_visible {
                    self.update(Action::RefreshModpack);
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
            CommandId::BeginConsoleInput => self.update(Action::BeginConsoleInput),
            CommandId::AnalyzeModpack => self.update(Action::AnalyzeModpack),
            CommandId::RefreshModpack => self.update(Action::RefreshModpack),
            CommandId::WizardAdvance => self.update(Action::Wizard(WizardAction::Advance)),
            CommandId::WizardBack => self.update(Action::Wizard(WizardAction::Back)),
            CommandId::CancelWizard => self.update(Action::CancelWizard),
            CommandId::GoDashboard => self.update(Action::GoDashboard),
            CommandId::ShowShell => self.update(Action::ShowShell),
            CommandId::ShowClassic => self.update(Action::ShowClassic),
            CommandId::Theme => self.update(Action::CycleTheme),
            CommandId::Help => self.update(Action::ToggleHelp),
            CommandId::Back => {
                if self.task_detail_visible {
                    self.update(Action::BackFromTaskDetail);
                } else if self.backup_manager_visible {
                    self.update(Action::BackFromBackups);
                } else if self.instance_detail_visible {
                    self.update(Action::BackFromInstanceDetail);
                } else if self.modpack_import_visible {
                    self.update(Action::BackFromModpack);
                } else if self.wizard.is_some() {
                    self.update(Action::Wizard(WizardAction::Back));
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
        if self.task_detail.is_none() {
            match result {
                Ok(task) => {
                    if let Some(existing) = self.tasks.iter_mut().find(|item| item.id == task.id) {
                        *existing = task;
                    }
                    self.status = "取消请求已发送；任务会在安全点停止。".to_owned();
                    self.push_transcript(TranscriptEntry::warning(self.status.clone()));
                    self.refresh_requested = true;
                }
                Err(error) => {
                    let message = match error.next_action {
                        Some(action) => format!("{} · 下一步：{}", error.reason, action),
                        None => error.reason,
                    };
                    self.status = format!("取消失败：{message}");
                    self.push_transcript(TranscriptEntry::error(format!("取消失败：{message}")));
                }
            }
            return;
        }
        let Some(detail) = self.task_detail.as_mut() else {
            return;
        };
        detail.apply_cancel_result(result);
        let error = detail.error.clone();
        let notice = detail
            .notice
            .clone()
            .unwrap_or_else(|| "取消请求已发送。".to_owned());
        self.status = error
            .as_ref()
            .map(|error| format!("取消失败：{error}"))
            .unwrap_or_else(|| notice.clone());
        if error.is_none() {
            self.task_detail_refresh_requested = true;
        }
        match error {
            Some(error) => {
                self.push_transcript(TranscriptEntry::error(format!("任务取消失败：{error}")))
            }
            None => self.push_transcript(TranscriptEntry::warning(notice)),
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

    /// The mascot only needs a frame clock while the session shell is the
    /// visible primary surface; overlays and detail pages have their own
    /// animation predicates.
    pub fn shell_animation_active(&self) -> bool {
        self.shell_visible
            && !self.help_visible
            && self.command_palette.is_none()
            && self.wizard.is_none()
            && !self.task_detail_visible
            && !self.backup_manager_visible
            && !self.instance_detail_visible
            && !self.modpack_import_visible
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

    fn move_selection_by(&mut self, delta: isize) {
        let (selection, length) = match self.focused_panel {
            Panel::Instances => (&mut self.selected_instance, self.instances.len()),
            Panel::Tasks => (&mut self.selected_task, self.tasks.len()),
        };
        if length == 0 {
            *selection = 0;
            return;
        }
        let next = *selection as isize + delta;
        *selection = next.clamp(0, length.saturating_sub(1) as isize) as usize;
    }
}

#[cfg(test)]
mod tests;
