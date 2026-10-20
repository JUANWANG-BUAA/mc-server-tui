use mc_core::{JavaRuntime, ServerVersion};
use ratatui::{backend::TestBackend, Terminal};

use crate::hit::HitMap;

use crate::wizard::{CreateOptions, CreateWizard, WizardAction, WizardStep};

use super::render;

#[test]
fn renders_guided_version_java_and_review_steps_at_both_sizes() {
    let mut wizard = sample_wizard();
    wizard.name = "Survival".to_owned();
    let server = render_to_text(&wizard, 80, 24);
    assert!(server.contains("实例与游戏版本"));
    assert!(server.contains("Minecraft版本"));
    assert!(server.contains("1.21.11"));
    assert!(render_to_text(&wizard, 40, 12).contains("Survival"));

    wizard.handle(WizardAction::Advance);
    let runtime = render_to_text(&wizard, 80, 24);
    assert!(runtime.contains("Java运行时"));
    assert!(runtime.contains("推荐"));
    assert!(runtime.contains("内存预设"));
    assert!(runtime.contains("Survival·Paper1.21.11"));
    assert!(render_to_text(&wizard, 40, 12).contains("Survival·Paper1.21.11"));
    assert!(render_to_text(&wizard, 40, 12).contains("Java"));

    wizard.handle(WizardAction::Advance);
    assert_eq!(wizard.step, WizardStep::Review);
    let review = render_to_text(&wizard, 80, 24);
    assert!(review.contains("安装计划"));
    assert!(review.contains("Paper1.21.11"));
    assert!(review.contains("MinecraftEULA"));
    let compact = render_to_text(&wizard, 40, 12);
    assert!(compact.contains("EULA"));
}

#[test]
fn loading_wizard_explains_what_is_happening() {
    let text = render_to_text(&CreateWizard::default(), 80, 24);
    assert!(text.contains("正在读取Paper稳定版本"));
}

#[test]
fn name_cursor_uses_terminal_columns_for_wide_characters() {
    let mut wizard = sample_wizard();
    wizard.name = "abc".to_owned();
    let ascii_cursor = render_cursor_x(&wizard);

    wizard.name = "中文服".to_owned();
    let wide_cursor = render_cursor_x(&wizard);

    assert_eq!(wide_cursor - ascii_cursor, 3);
}

fn sample_wizard() -> CreateWizard {
    let mut wizard = CreateWizard::default();
    wizard.apply_options(CreateOptions {
        versions: vec![
            ServerVersion {
                id: "1.21.11".to_owned(),
                group: "1.21".to_owned(),
            },
            ServerVersion {
                id: "1.21.10".to_owned(),
                group: "1.21".to_owned(),
            },
        ],
        java_runtimes: vec![
            JavaRuntime {
                executable: "/opt/java21/bin/java".to_owned(),
                version: "21.0.2".to_owned(),
                major_version: Some(21),
                architecture: Some("aarch64".to_owned()),
                vendor: Some("Eclipse Adoptium".to_owned()),
            },
            JavaRuntime {
                executable: "/opt/java25/bin/java".to_owned(),
                version: "25.0.1".to_owned(),
                major_version: Some(25),
                architecture: Some("aarch64".to_owned()),
                vendor: Some("Eclipse Adoptium".to_owned()),
            },
        ],
        suggested_port: 25565,
    });
    wizard
}

fn render_to_text(wizard: &CreateWizard, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    terminal
        .draw(|frame| render(frame, frame.area(), wizard, 0, &mut HitMap::default()))
        .expect("render");
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .filter(|symbol| !symbol.chars().all(char::is_whitespace))
        .collect()
}

fn render_cursor_x(wizard: &CreateWizard) -> u16 {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    terminal
        .draw(|frame| render(frame, frame.area(), wizard, 0, &mut HitMap::default()))
        .expect("render");
    terminal.get_cursor_position().expect("cursor position").x
}

#[test]
fn runtime_summary_requires_explicit_expansion_before_editing() {
    let mut wizard = sample_wizard();
    wizard.name = "朋友服".into();
    wizard.handle(WizardAction::Advance);
    let before = wizard.selected_java;
    wizard.handle(WizardAction::Down);
    assert_eq!(wizard.selected_java, before);
    assert!(!render_to_text(&wizard, 80, 24).contains("/opt/java"));
    wizard.handle(WizardAction::ToggleRuntimeSettings);
    assert!(render_to_text(&wizard, 80, 24).contains("/opt/java"));
    wizard.handle(WizardAction::Down);
    assert_ne!(wizard.selected_java, before);
    wizard.handle(WizardAction::ToggleRuntimeSettings);
    assert!(!wizard.runtime_settings_expanded);
}
