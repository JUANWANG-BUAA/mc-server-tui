use std::path::PathBuf;

use mc_core::{JavaRuntime, ServerVersion};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WizardStep {
    Server,
    Runtime,
    Review,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WizardAction {
    Advance,
    Back,
    Up,
    Down,
    FocusNext,
    FocusPrevious,
    FocusField(usize),
    PreviousChoice,
    NextChoice,
    PagePrevious,
    PageNext,
    FirstChoice,
    LastChoice,
    SelectChoice(usize),
    Insert(char),
    Backspace,
    ToggleAcceptance,
    ToggleRuntimeSettings,
}

#[derive(Debug)]
pub(crate) enum WizardOutcome {
    None,
    Cancelled,
    Submit(CreateInstanceInput),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreateInstanceInput {
    pub name: String,
    pub game_version: String,
    pub java_path: PathBuf,
    pub min_memory_mb: u32,
    pub max_memory_mb: u32,
    pub port: u16,
    pub accept_eula: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct CreateOptions {
    pub versions: Vec<ServerVersion>,
    pub java_runtimes: Vec<JavaRuntime>,
    pub suggested_port: u16,
}

const MEMORY_PROFILES: [(u32, u32); 4] = [(512, 1024), (1024, 2048), (2048, 4096), (4096, 8192)];
const CHOICE_PAGE_STEP: isize = 5;

#[derive(Debug)]
pub(crate) struct CreateWizard {
    pub step: WizardStep,
    pub name: String,
    pub versions: Vec<ServerVersion>,
    pub selected_version: usize,
    pub java_runtimes: Vec<JavaRuntime>,
    pub selected_java: usize,
    pub memory_profile: usize,
    pub runtime_settings_expanded: bool,
    pub port: String,
    pub accept_eula: bool,
    pub focused_field: usize,
    pub error: Option<String>,
    pub options_loading: bool,
    pub options_error: Option<String>,
    pub submitting: bool,
}

impl Default for CreateWizard {
    fn default() -> Self {
        Self {
            step: WizardStep::Server,
            name: String::new(),
            versions: Vec::new(),
            selected_version: 0,
            java_runtimes: Vec::new(),
            selected_java: 0,
            memory_profile: 1,
            runtime_settings_expanded: false,
            port: "25565".to_owned(),
            accept_eula: false,
            focused_field: 0,
            error: None,
            options_loading: true,
            options_error: None,
            submitting: false,
        }
    }
}

impl CreateWizard {
    pub fn apply_options(&mut self, mut options: CreateOptions) {
        self.options_loading = false;
        options.versions.retain(|version| {
            let id = version.id.to_ascii_lowercase();
            !id.contains("-pre") && !id.contains("-rc")
        });
        if options.versions.is_empty() {
            self.options_error = Some("Paper 没有返回可用稳定版本，请稍后重试。".to_owned());
            return;
        }
        options
            .versions
            .sort_by_key(|v| std::cmp::Reverse(mc_core::minecraft_version_key(&v.id)));
        self.versions = options.versions;
        self.selected_version = 0;
        self.java_runtimes = options.java_runtimes;
        self.selected_java =
            preferred_java_index(&self.java_runtimes, self.required_java()).unwrap_or(0);
        self.port = options.suggested_port.to_string();
        self.options_error = if self.java_runtimes.is_empty() {
            Some("未检测到可用 Java。请先安装 Java 21 或更高版本，再重新打开创建向导。".to_owned())
        } else {
            None
        };
        self.error = None;
    }

    pub fn set_options_error(&mut self, error: String) {
        self.options_loading = false;
        self.options_error = Some(error);
    }

    pub fn required_java(&self) -> u16 {
        self.selected_version()
            .map(|v| mc_core::minecraft_java_requirement(&v.id))
            .unwrap_or(21)
    }

    pub fn selected_version(&self) -> Option<&ServerVersion> {
        self.versions.get(self.selected_version)
    }

    pub fn selected_java(&self) -> Option<&JavaRuntime> {
        self.java_runtimes.get(self.selected_java)
    }

    pub fn memory_range(&self) -> (u32, u32) {
        MEMORY_PROFILES
            .get(self.memory_profile)
            .copied()
            .unwrap_or(MEMORY_PROFILES[1])
    }

    pub fn java_choice_label(&self, index: usize) -> String {
        let Some(runtime) = self.java_runtimes.get(index) else {
            return "无可用 Java".to_owned();
        };
        let major = runtime
            .major_version
            .map(|major| format!("Java {major}"))
            .unwrap_or_else(|| runtime.version.clone());
        let vendor = runtime
            .vendor
            .clone()
            .unwrap_or_else(|| "未知发行版".to_owned());
        let arch = runtime
            .architecture
            .clone()
            .unwrap_or_else(|| "未知架构".to_owned());
        format!("{major} · {vendor} · {arch}")
    }

    pub fn java_is_recommended(&self, index: usize) -> bool {
        preferred_java_index(&self.java_runtimes, self.required_java()) == Some(index)
    }

    pub fn handle(&mut self, action: WizardAction) -> WizardOutcome {
        if self.submitting {
            return WizardOutcome::None;
        }
        if self.step == WizardStep::Runtime
            && !self.runtime_settings_expanded
            && matches!(
                action,
                WizardAction::Up
                    | WizardAction::Down
                    | WizardAction::PreviousChoice
                    | WizardAction::NextChoice
                    | WizardAction::PagePrevious
                    | WizardAction::PageNext
                    | WizardAction::FirstChoice
                    | WizardAction::LastChoice
                    | WizardAction::SelectChoice(_)
                    | WizardAction::Insert(_)
                    | WizardAction::Backspace
            )
        {
            return WizardOutcome::None;
        }
        match action {
            WizardAction::ToggleRuntimeSettings => {
                if self.step == WizardStep::Runtime {
                    self.runtime_settings_expanded = !self.runtime_settings_expanded;
                    self.focused_field = 0;
                }
                WizardOutcome::None
            }
            WizardAction::Advance => self.advance(),
            WizardAction::Back => self.back(),
            WizardAction::Up => {
                if self.focus_is_choice() {
                    self.adjust_choice(-1);
                    WizardOutcome::None
                } else {
                    self.handle(WizardAction::FocusPrevious)
                }
            }
            WizardAction::Down => {
                if self.focus_is_choice() {
                    self.adjust_choice(1);
                    WizardOutcome::None
                } else {
                    self.handle(WizardAction::FocusNext)
                }
            }
            WizardAction::FocusNext => {
                if self.step == WizardStep::Runtime {
                    self.runtime_settings_expanded = true;
                }
                self.focused_field = (self.focused_field + 1) % self.field_count();
                self.error = None;
                WizardOutcome::None
            }
            WizardAction::FocusPrevious => {
                if self.step == WizardStep::Runtime {
                    self.runtime_settings_expanded = true;
                }
                self.focused_field = self
                    .focused_field
                    .checked_sub(1)
                    .unwrap_or_else(|| self.field_count() - 1);
                self.error = None;
                WizardOutcome::None
            }
            WizardAction::FocusField(index) => {
                if index < self.field_count() {
                    self.focused_field = index;
                    self.error = None;
                }
                WizardOutcome::None
            }
            WizardAction::PreviousChoice => {
                self.adjust_choice(-1);
                WizardOutcome::None
            }
            WizardAction::NextChoice => {
                self.adjust_choice(1);
                WizardOutcome::None
            }
            WizardAction::PagePrevious => {
                self.adjust_choice(-CHOICE_PAGE_STEP);
                WizardOutcome::None
            }
            WizardAction::PageNext => {
                self.adjust_choice(CHOICE_PAGE_STEP);
                WizardOutcome::None
            }
            WizardAction::FirstChoice => {
                self.select_choice(0);
                WizardOutcome::None
            }
            WizardAction::LastChoice => {
                let last = self.choice_count().saturating_sub(1);
                self.select_choice(last);
                WizardOutcome::None
            }
            WizardAction::SelectChoice(index) => {
                self.select_choice(index);
                WizardOutcome::None
            }
            WizardAction::Insert(character) => {
                self.insert(character);
                WizardOutcome::None
            }
            WizardAction::Backspace => {
                self.backspace();
                WizardOutcome::None
            }
            WizardAction::ToggleAcceptance if self.step == WizardStep::Review => {
                self.accept_eula = !self.accept_eula;
                self.error = None;
                WizardOutcome::None
            }
            WizardAction::ToggleAcceptance => WizardOutcome::None,
        }
    }

    pub fn set_submit_error(&mut self, error: String) {
        self.submitting = false;
        self.error = Some(error);
    }

    fn focus_is_choice(&self) -> bool {
        matches!(
            (self.step, self.focused_field),
            (WizardStep::Server, 1) | (WizardStep::Runtime, 0) | (WizardStep::Runtime, 1)
        )
    }

    fn choice_count(&self) -> usize {
        match (self.step, self.focused_field) {
            (WizardStep::Server, 1) => self.versions.len(),
            (WizardStep::Runtime, 0) => self.java_runtimes.len(),
            (WizardStep::Runtime, 1) => MEMORY_PROFILES.len(),
            _ => 0,
        }
    }

    fn select_choice(&mut self, index: usize) {
        self.error = None;
        match (self.step, self.focused_field) {
            (WizardStep::Server, 1) if index < self.versions.len() => {
                self.selected_version = index;
            }
            (WizardStep::Runtime, 0) if index < self.java_runtimes.len() => {
                self.selected_java = index;
            }
            (WizardStep::Runtime, 1) if index < MEMORY_PROFILES.len() => {
                self.memory_profile = index;
            }
            _ => {}
        }
    }

    fn adjust_choice(&mut self, delta: isize) {
        self.error = None;
        match (self.step, self.focused_field) {
            (WizardStep::Server, 1) if !self.versions.is_empty() => {
                self.selected_version =
                    adjusted_index(self.selected_version, self.versions.len(), delta);
            }
            (WizardStep::Runtime, 0) if !self.java_runtimes.is_empty() => {
                self.selected_java =
                    adjusted_index(self.selected_java, self.java_runtimes.len(), delta);
            }
            (WizardStep::Runtime, 1) => {
                self.memory_profile =
                    adjusted_index(self.memory_profile, MEMORY_PROFILES.len(), delta);
            }
            _ => {}
        }
    }

    fn advance(&mut self) -> WizardOutcome {
        self.error = None;
        match self.step {
            WizardStep::Server => {
                if let Err((field, message)) = self.validate_server() {
                    self.focused_field = field;
                    self.error = Some(message);
                    return WizardOutcome::None;
                }
                self.step = WizardStep::Runtime;
                self.selected_java =
                    preferred_java_index(&self.java_runtimes, self.required_java())
                        .unwrap_or(self.selected_java);
                self.focused_field = 0;
                WizardOutcome::None
            }
            WizardStep::Runtime => {
                if let Err((field, message)) = self.validate_runtime() {
                    self.runtime_settings_expanded = true;
                    self.focused_field = field;
                    self.error = Some(message);
                    return WizardOutcome::None;
                }
                self.step = WizardStep::Review;
                self.focused_field = 0;
                WizardOutcome::None
            }
            WizardStep::Review => match self.to_input() {
                Ok(input) => {
                    self.submitting = true;
                    WizardOutcome::Submit(input)
                }
                Err(message) => {
                    self.error = Some(message);
                    WizardOutcome::None
                }
            },
        }
    }

    fn back(&mut self) -> WizardOutcome {
        self.error = None;
        match self.step {
            WizardStep::Server => WizardOutcome::Cancelled,
            WizardStep::Runtime => {
                self.step = WizardStep::Server;
                self.focused_field = 0;
                WizardOutcome::None
            }
            WizardStep::Review => {
                self.step = WizardStep::Runtime;
                self.focused_field = 0;
                WizardOutcome::None
            }
        }
    }

    fn field_count(&self) -> usize {
        match self.step {
            WizardStep::Server => 2,
            WizardStep::Runtime => 3,
            WizardStep::Review => 1,
        }
    }

    fn insert(&mut self, character: char) {
        if character.is_control() || self.step == WizardStep::Review {
            return;
        }
        let value = match (self.step, self.focused_field) {
            (WizardStep::Server, 0) => &mut self.name,
            (WizardStep::Runtime, 2) => &mut self.port,
            _ => return,
        };
        value.push(character);
        self.error = None;
    }

    fn backspace(&mut self) {
        let value = match (self.step, self.focused_field) {
            (WizardStep::Server, 0) => &mut self.name,
            (WizardStep::Runtime, 2) => &mut self.port,
            _ => return,
        };
        value.pop();
        self.error = None;
    }

    fn validate_server(&self) -> Result<(), (usize, String)> {
        if self.options_loading {
            return Err((0, "正在读取 Paper 稳定版本，请稍候。".to_owned()));
        }
        if let Some(error) = &self.options_error {
            return Err((0, error.clone()));
        }
        let name = self.name.trim();
        if name.is_empty() || name.len() > 128 || name.chars().any(char::is_control) {
            return Err((
                0,
                "请输入实例名称（最多 128 字节，中文约 42 字）。".to_owned(),
            ));
        }
        if self.selected_version().is_none() {
            return Err((1, "请选择一个 Paper 稳定版本。".to_owned()));
        }
        Ok(())
    }

    fn validate_runtime(&self) -> Result<(), (usize, String)> {
        if self.selected_java().is_none() {
            return Err((
                0,
                "没有可用的 Java 运行时，请先安装 Java 21 或更高版本。".to_owned(),
            ));
        }
        if !self.selected_java().is_some_and(|java| {
            java.major_version
                .is_some_and(|v| v >= self.required_java())
        }) {
            return Err((
                0,
                format!(
                    "Minecraft {} 至少需要 Java {}，请选择兼容的运行时。",
                    self.selected_version().map(|v| v.id.as_str()).unwrap_or(""),
                    self.required_java()
                ),
            ));
        }
        let port = self
            .port
            .parse::<u16>()
            .ok()
            .filter(|port| *port >= 1024)
            .ok_or_else(|| (2, "端口需为 1024–65535 之间的整数。".to_owned()))?;
        let _ = port;
        Ok(())
    }

    fn to_input(&self) -> Result<CreateInstanceInput, String> {
        self.validate_server().map_err(|(_, message)| message)?;
        self.validate_runtime().map_err(|(_, message)| message)?;
        if !self.accept_eula {
            return Err("请先用 Space 明确接受 Minecraft EULA。".to_owned());
        }
        let version = self
            .selected_version()
            .ok_or_else(|| "Paper 版本选择已失效，请重新打开向导。".to_owned())?;
        let java = self
            .selected_java()
            .ok_or_else(|| "Java 运行时选择已失效，请重新打开向导。".to_owned())?;
        let (min_memory_mb, max_memory_mb) = self.memory_range();
        let port = self
            .port
            .parse::<u16>()
            .map_err(|_| "端口格式不正确。".to_owned())?;
        Ok(CreateInstanceInput {
            name: self.name.trim().to_owned(),
            game_version: version.id.clone(),
            java_path: PathBuf::from(&java.executable),
            min_memory_mb,
            max_memory_mb,
            port,
            accept_eula: true,
        })
    }
}

fn adjusted_index(current: usize, length: usize, delta: isize) -> usize {
    if length == 0 {
        return 0;
    }
    let next = current as isize + delta;
    next.rem_euclid(length as isize) as usize
}

fn preferred_java_index(runtimes: &[JavaRuntime], required: u16) -> Option<usize> {
    if runtimes.is_empty() {
        return None;
    }
    if let Some(index) = runtimes
        .iter()
        .position(|runtime| runtime.major_version == Some(required))
    {
        return Some(index);
    }
    if let Some((index, _)) = runtimes
        .iter()
        .enumerate()
        .filter(|(_, runtime)| runtime.major_version.is_some_and(|major| major >= required))
        .min_by_key(|(_, runtime)| runtime.major_version.unwrap_or(u16::MAX))
    {
        return Some(index);
    }
    runtimes
        .iter()
        .enumerate()
        .max_by_key(|(_, runtime)| runtime.major_version.unwrap_or(0))
        .map(|(index, _)| index)
}

#[cfg(test)]
mod tests;
