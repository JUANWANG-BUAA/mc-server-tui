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
