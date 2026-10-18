use std::path::PathBuf;

use mc_core::{JavaRuntime, ServerVersion};

use super::{CreateOptions, CreateWizard, WizardAction, WizardOutcome, WizardStep};

#[test]
fn version_and_java_are_selected_not_typed() {
    let mut wizard = sample_wizard();
    wizard.name = "Survival".to_owned();
    assert_eq!(
        wizard.selected_version().map(|version| version.id.as_str()),
        Some("1.21.11"),
    );
    assert_eq!(
        wizard.selected_java().map(|java| java.major_version),
        Some(Some(21)),
    );

    wizard.handle(WizardAction::FocusNext);
    wizard.handle(WizardAction::NextChoice);
    assert_eq!(
        wizard.selected_version().map(|version| version.id.as_str()),
        Some("1.21.10"),
    );
    wizard.handle(WizardAction::Backspace);
    assert_eq!(
        wizard.selected_version().map(|version| version.id.as_str()),
        Some("1.21.10"),
    );

    wizard.handle(WizardAction::Advance);
    assert_eq!(wizard.step, WizardStep::Runtime);
    wizard.handle(WizardAction::NextChoice);
    assert_eq!(
        wizard.selected_java().map(|java| java.major_version),
        Some(Some(25)),
    );
}

#[test]
fn review_requires_explicit_eula_and_emits_selected_options() {
    let mut wizard = sample_wizard();
    wizard.name = "Survival".to_owned();
    wizard.handle(WizardAction::Advance);
    wizard.handle(WizardAction::Advance);
    assert_eq!(wizard.step, WizardStep::Review);

    assert!(matches!(
        wizard.handle(WizardAction::Advance),
        WizardOutcome::None
    ));
    assert!(wizard.error.as_deref().unwrap().contains("EULA"));

    wizard.handle(WizardAction::ToggleAcceptance);
    let outcome = wizard.handle(WizardAction::Advance);
    assert!(matches!(&outcome, WizardOutcome::Submit(_)));
    if let WizardOutcome::Submit(input) = outcome {
        assert_eq!(input.name, "Survival");
        assert_eq!(input.game_version, "1.21.11");
        assert_eq!(input.java_path, PathBuf::from("/opt/java21/bin/java"));
        assert_eq!(input.min_memory_mb, 1024);
        assert_eq!(input.max_memory_mb, 2048);
        assert_eq!(input.port, 25565);
        assert!(input.accept_eula);
        assert!(wizard.submitting);
    }
}

#[test]
fn loading_and_missing_java_are_actionable_errors() {
    let mut wizard = CreateWizard::default();
    assert!(matches!(
        wizard.handle(WizardAction::Advance),
        WizardOutcome::None
    ));
    assert!(wizard.error.as_deref().unwrap().contains("正在读取"));

    wizard.name = "Survival".to_owned();
    wizard.apply_options(CreateOptions {
        versions: vec![version("1.21.11")],
        java_runtimes: Vec::new(),
        suggested_port: 25565,
    });
    assert!(wizard.options_error.as_deref().unwrap().contains("Java"));

    wizard.options_error = None;
    wizard.handle(WizardAction::Advance);
    wizard.handle(WizardAction::Advance);
    assert!(wizard.error.as_deref().unwrap().contains("Java"));
}

fn sample_wizard() -> CreateWizard {
    let mut wizard = CreateWizard::default();
    wizard.apply_options(CreateOptions {
        versions: vec![version("1.21.11"), version("1.21.10")],
        java_runtimes: vec![
            java(17, "/opt/java17/bin/java"),
            java(21, "/opt/java21/bin/java"),
            java(25, "/opt/java25/bin/java"),
        ],
        suggested_port: 25565,
    });
    wizard
}

fn version(id: &str) -> ServerVersion {
    ServerVersion {
        id: id.to_owned(),
        group: "1.21".to_owned(),
    }
}

fn java(major: u16, executable: &str) -> JavaRuntime {
    JavaRuntime {
        executable: executable.to_owned(),
        version: major.to_string() + ".0.1",
        major_version: Some(major),
        architecture: Some("aarch64".to_owned()),
        vendor: Some("Eclipse Adoptium".to_owned()),
    }
}
