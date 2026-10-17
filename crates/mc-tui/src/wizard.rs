use std::path::PathBuf;

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
    FocusNext,
    FocusPrevious,
    Insert(char),
    Backspace,
    ToggleAcceptance,
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

#[derive(Debug)]
pub(crate) struct CreateWizard {
    pub step: WizardStep,
    pub name: String,
    pub game_version: String,
    pub java_path: String,
    pub min_memory_mb: String,
    pub max_memory_mb: String,
    pub port: String,
    pub accept_eula: bool,
    pub focused_field: usize,
    pub error: Option<String>,
    pub submitting: bool,
}

impl Default for CreateWizard {
    fn default() -> Self {
        Self {
            step: WizardStep::Server,
            name: String::new(),
            game_version: String::new(),
            java_path: String::new(),
            min_memory_mb: "1024".to_owned(),
            max_memory_mb: "2048".to_owned(),
            port: "25565".to_owned(),
            accept_eula: false,
            focused_field: 0,
            error: None,
            submitting: false,
        }
    }
}

impl CreateWizard {
    pub fn handle(&mut self, action: WizardAction) -> WizardOutcome {
        if self.submitting {
            return WizardOutcome::None;
        }
        match action {
            WizardAction::Advance => self.advance(),
            WizardAction::Back => self.back(),
            WizardAction::FocusNext => {
                self.focused_field = (self.focused_field + 1) % self.field_count();
                self.error = None;
                WizardOutcome::None
            }
            WizardAction::FocusPrevious => {
                self.focused_field = self
                    .focused_field
                    .checked_sub(1)
                    .unwrap_or_else(|| self.field_count() - 1);
                self.error = None;
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
                self.focused_field = 0;
                WizardOutcome::None
            }
            WizardStep::Runtime => {
                if let Err((field, message)) = self.validate_runtime() {
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
            WizardStep::Runtime => 4,
            WizardStep::Review => 1,
        }
    }

    fn insert(&mut self, character: char) {
        if character.is_control() || self.step == WizardStep::Review {
            return;
        }
        let value = match (self.step, self.focused_field) {
            (WizardStep::Server, 0) => &mut self.name,
            (WizardStep::Server, 1) => &mut self.game_version,
            (WizardStep::Runtime, 0) => &mut self.java_path,
            (WizardStep::Runtime, 1) => &mut self.min_memory_mb,
            (WizardStep::Runtime, 2) => &mut self.max_memory_mb,
            (WizardStep::Runtime, 3) => &mut self.port,
            _ => return,
        };
        value.push(character);
        self.error = None;
    }

    fn backspace(&mut self) {
        let value = match (self.step, self.focused_field) {
            (WizardStep::Server, 0) => &mut self.name,
            (WizardStep::Server, 1) => &mut self.game_version,
            (WizardStep::Runtime, 0) => &mut self.java_path,
            (WizardStep::Runtime, 1) => &mut self.min_memory_mb,
            (WizardStep::Runtime, 2) => &mut self.max_memory_mb,
            (WizardStep::Runtime, 3) => &mut self.port,
            _ => return,
        };
        value.pop();
        self.error = None;
    }

    fn validate_server(&self) -> Result<(), (usize, String)> {
        let name = self.name.trim();
        if name.is_empty() || name.len() > 128 || name.chars().any(char::is_control) {
            return Err((0, "实例名称需为 1–128 个 UTF-8 字节。".to_owned()));
        }
        if self.game_version.trim().is_empty() {
            return Err((1, "请输入 Minecraft 版本，例如 1.21.11。".to_owned()));
        }
        Ok(())
    }

    fn validate_runtime(&self) -> Result<(), (usize, String)> {
        self.validated_runtime().map(|_| ())
    }

    fn validated_runtime(&self) -> Result<(PathBuf, u32, u32, u16), (usize, String)> {
        let java_path = PathBuf::from(self.java_path.trim());
        if !java_path.is_absolute() {
            return Err((0, "Java 路径必须是绝对路径。".to_owned()));
        }
        let min_memory_mb = parse_memory(&self.min_memory_mb).map_err(|message| (1, message))?;
        let max_memory_mb = parse_memory(&self.max_memory_mb).map_err(|message| (2, message))?;
        if min_memory_mb > max_memory_mb {
            return Err((1, "最小内存不能大于最大内存。".to_owned()));
        }
        let port = self
            .port
            .parse::<u16>()
            .ok()
            .filter(|port| *port >= 1024)
            .ok_or_else(|| (3, "端口需为 1024–65535 之间的整数。".to_owned()))?;
        Ok((java_path, min_memory_mb, max_memory_mb, port))
    }

    fn to_input(&self) -> Result<CreateInstanceInput, String> {
        self.validate_server().map_err(|(_, message)| message)?;
        let (java_path, min_memory_mb, max_memory_mb, port) =
            self.validated_runtime().map_err(|(_, message)| message)?;
        if !self.accept_eula {
            return Err("请先阅读并明确接受 Minecraft EULA。".to_owned());
        }
        Ok(CreateInstanceInput {
            name: self.name.trim().to_owned(),
            game_version: self.game_version.trim().to_owned(),
            java_path,
            min_memory_mb,
            max_memory_mb,
            port,
            accept_eula: true,
        })
    }
}

fn parse_memory(value: &str) -> Result<u32, String> {
    value
        .parse::<u32>()
        .ok()
        .filter(|memory| (512..=65_536).contains(memory))
        .ok_or_else(|| "内存需为 512–65536 MiB 之间的整数。".to_owned())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{CreateWizard, WizardAction, WizardOutcome, WizardStep};

    #[test]
    fn validates_each_step_and_keeps_values_when_moving_back() {
        let mut wizard = CreateWizard::default();
        assert!(matches!(
            wizard.handle(WizardAction::Advance),
            WizardOutcome::None
        ));
        assert!(wizard.error.as_deref().unwrap().contains("实例名称"));

        wizard.handle(WizardAction::Insert('S'));
        wizard.handle(WizardAction::FocusNext);
        wizard.handle(WizardAction::Insert('1'));
        wizard.handle(WizardAction::Insert('.'));
        wizard.handle(WizardAction::Insert('2'));
        wizard.handle(WizardAction::Advance);
        assert_eq!(wizard.step, WizardStep::Runtime);
        assert_eq!(wizard.name, "S");

        wizard.handle(WizardAction::Back);
        assert_eq!(wizard.step, WizardStep::Server);
        assert_eq!(wizard.game_version, "1.2");
    }

    #[test]
    fn review_requires_explicit_eula_and_emits_typed_input_after_acceptance() {
        let mut wizard = CreateWizard {
            name: "Survival".to_owned(),
            game_version: "1.21.11".to_owned(),
            java_path: "/usr/bin/java".to_owned(),
            ..CreateWizard::default()
        };
        wizard.handle(WizardAction::Advance);
        wizard.handle(WizardAction::Advance);

        assert_eq!(wizard.step, WizardStep::Review);
        assert!(matches!(
            wizard.handle(WizardAction::Advance),
            WizardOutcome::None
        ));
        assert!(wizard.error.as_deref().unwrap().contains("EULA"));

        wizard.handle(WizardAction::ToggleAcceptance);
        let WizardOutcome::Submit(input) = wizard.handle(WizardAction::Advance) else {
            panic!("accepted review should submit");
        };
        assert_eq!(input.name, "Survival");
        assert_eq!(input.java_path, PathBuf::from("/usr/bin/java"));
        assert_eq!(input.port, 25565);
        assert!(input.accept_eula);
        assert!(wizard.submitting);
    }

    #[test]
    fn runtime_rejects_relative_java_and_invalid_resource_ranges() {
        let mut wizard = CreateWizard {
            name: "Survival".to_owned(),
            game_version: "1.21.11".to_owned(),
            java_path: "java".to_owned(),
            ..CreateWizard::default()
        };
        wizard.handle(WizardAction::Advance);
        wizard.handle(WizardAction::Advance);
        assert!(wizard.error.as_deref().unwrap().contains("绝对路径"));

        wizard.java_path = "/usr/bin/java".to_owned();
        wizard.min_memory_mb = "4096".to_owned();
        wizard.max_memory_mb = "2048".to_owned();
        wizard.handle(WizardAction::Advance);
        assert!(wizard.error.as_deref().unwrap().contains("不能大于"));

        wizard.min_memory_mb = "1024".to_owned();
        wizard.port = "80".to_owned();
        wizard.handle(WizardAction::Advance);
        assert!(wizard.error.as_deref().unwrap().contains("1024–65535"));
    }

    #[test]
    fn submission_error_restores_editing_without_losing_values() {
        let mut wizard = CreateWizard {
            name: "Survival".to_owned(),
            ..CreateWizard::default()
        };
        wizard.submitting = true;
        wizard.set_submit_error("daemon unavailable".to_owned());

        assert!(!wizard.submitting);
        assert_eq!(wizard.name, "Survival");
        assert_eq!(wizard.error.as_deref(), Some("daemon unavailable"));
    }
}
