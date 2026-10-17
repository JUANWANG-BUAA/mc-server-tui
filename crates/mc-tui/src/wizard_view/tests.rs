use ratatui::{backend::TestBackend, Terminal};

use crate::wizard::{CreateWizard, WizardAction, WizardStep};

use super::render;

#[test]
fn renders_all_steps_and_explicit_eula_control_at_reference_width() {
    let mut wizard = CreateWizard {
        name: "Survival".to_owned(),
        game_version: "1.21.11".to_owned(),
        java_path: "/usr/bin/java".to_owned(),
        ..CreateWizard::default()
    };
    let server = render_to_text(&wizard, 80, 24);
    assert!(server.contains("服务信息"));
    assert!(server.contains("Survival"));
    assert!(render_to_text(&wizard, 40, 12).contains("Survival"));

    wizard.handle(WizardAction::Advance);
    let runtime = render_to_text(&wizard, 80, 24);
    assert!(runtime.contains("Java与资源"));
    assert!(runtime.contains("/usr/bin/java"));
    assert!(render_to_text(&wizard, 40, 12).contains("/usr/bin/java"));

    wizard.handle(WizardAction::Advance);
    assert_eq!(wizard.step, WizardStep::Review);
    let review = render_to_text(&wizard, 80, 24);
    assert!(review.contains("预览与确认"));
    assert!(review.contains("我已阅读并接受MinecraftEULA"));
    assert!(render_to_text(&wizard, 40, 12).contains("已阅读并接受EULA"));
}

#[test]
fn compact_wizard_render_does_not_panic() {
    let text = render_to_text(&CreateWizard::default(), 40, 12);
    assert!(text.contains("新建Paper实例"));
}

fn render_to_text(wizard: &CreateWizard, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    terminal
        .draw(|frame| render(frame, wizard))
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
