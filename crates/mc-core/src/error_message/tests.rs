//! Tests for error message module.

use super::*;

#[test]
fn error_message_basic_creation() {
    let msg = ErrorMessage::new("Test error", "Test reason");
    assert_eq!(msg.summary, "Test error");
    assert_eq!(msg.reason, "Test reason");
    assert!(msg.evidence.is_none());
    assert!(msg.actions.is_empty());
    assert!(!msg.retryable);
}

#[test]
fn error_message_with_all_fields() {
    let msg = ErrorMessage::new("Test error", "Test reason")
        .with_evidence("error_code=123")
        .with_action("First action")
        .with_action("Second action")
        .retryable();

    assert_eq!(msg.summary, "Test error");
    assert_eq!(msg.reason, "Test reason");
    assert_eq!(msg.evidence, Some("error_code=123".to_string()));
    assert_eq!(msg.actions.len(), 2);
    assert_eq!(msg.actions[0], "First action");
    assert_eq!(msg.actions[1], "Second action");
    assert!(msg.retryable);
}

#[test]
fn error_message_with_multiple_actions() {
    let actions = vec!["Action 1", "Action 2", "Action 3"];
    let msg = ErrorMessage::new("Test", "Reason").with_actions(actions);

    assert_eq!(msg.actions.len(), 3);
    assert_eq!(msg.actions[0], "Action 1");
    assert_eq!(msg.actions[1], "Action 2");
    assert_eq!(msg.actions[2], "Action 3");
}

#[test]
fn diagnostic_report_contains_all_sections() {
    let msg = ErrorMessage::new("Test error", "Test reason")
        .with_evidence("test_evidence=123")
        .with_actions(["Action 1", "Action 2"])
        .retryable();

    let report = msg.diagnostic_report();

    assert!(report.contains("错误诊断报告"));
    assert!(report.contains("Error Diagnostic Report"));
    assert!(report.contains("Test error"));
    assert!(report.contains("Test reason"));
    assert!(report.contains("test_evidence=123"));
    assert!(report.contains("Action 1"));
    assert!(report.contains("Action 2"));
    assert!(report.contains("可重试"));
    assert!(report.contains("是"));
    assert!(report.contains("Yes"));
}

#[test]
fn diagnostic_report_without_evidence() {
    let msg = ErrorMessage::new("Test error", "Test reason")
        .with_action("Fix it")
        .retryable();

    let report = msg.diagnostic_report();

    assert!(!report.contains("技术细节"));
    assert!(report.contains("Test error"));
    assert!(report.contains("Fix it"));
}

#[test]
fn diagnostic_report_without_actions() {
    let msg = ErrorMessage::new("Test error", "Test reason").with_evidence("evidence");

    let report = msg.diagnostic_report();

    assert!(!report.contains("建议操作"));
    assert!(!report.contains("Recommended Actions"));
}

mod templates {
    use super::*;

    #[test]
    fn java_version_mismatch_zh_cn() {
        let msg = ErrorTemplates::java_version_mismatch("21", "17", Language::ZhCN);

        assert!(msg.summary.contains("Java"));
        assert!(msg.summary.contains("不匹配"));
        assert!(msg.reason.contains("21"));
        assert!(msg.reason.contains("17"));
        assert!(msg.evidence.is_some());
        assert!(!msg.actions.is_empty());
        assert!(msg.actions[0].contains("adoptium.net"));
    }

    #[test]
    fn java_version_mismatch_en() {
        let msg = ErrorTemplates::java_version_mismatch("21", "17", Language::En);

        assert!(msg.summary.contains("Java"));
        assert!(msg.summary.contains("mismatch"));
        assert!(msg.reason.contains("21"));
        assert!(msg.reason.contains("17"));
        assert!(!msg.actions.is_empty());
    }

    #[test]
    fn port_in_use_zh_cn() {
        let msg = ErrorTemplates::port_in_use(25565, Language::ZhCN);

        assert!(msg.summary.contains("端口"));
        assert!(msg.summary.contains("占用"));
        assert!(msg.reason.contains("25565"));
        assert!(msg.retryable);
        assert!(!msg.actions.is_empty());
        assert!(msg.actions.iter().any(|a| a.contains("lsof")));
    }

    #[test]
    fn port_in_use_en() {
        let msg = ErrorTemplates::port_in_use(25565, Language::En);

        assert!(msg.summary.contains("Port"));
        assert!(msg.reason.contains("25565"));
        assert!(msg.retryable);
    }

    #[test]
    fn disk_space_low_zh_cn() {
        let needed = 1024 * 1024 * 500; // 500 MB
        let available = 1024 * 1024 * 100; // 100 MB
        let msg = ErrorTemplates::disk_space_low(needed, available, Language::ZhCN);

        assert!(msg.summary.contains("磁盘空间"));
        assert!(msg.summary.contains("不足"));
        assert!(msg.reason.contains("500"));
        assert!(msg.reason.contains("100"));
        assert!(msg.retryable);
        assert!(msg.actions.len() >= 2);
    }

    #[test]
    fn disk_space_low_en() {
        let needed = 1024 * 1024 * 1000; // 1000 MB
        let available = 1024 * 1024 * 50; // 50 MB
        let msg = ErrorTemplates::disk_space_low(needed, available, Language::En);

        assert!(msg.summary.contains("Insufficient"));
        assert!(msg.reason.contains("1000"));
        assert!(msg.reason.contains("50"));
        assert!(msg.retryable);
    }

    #[test]
    fn download_failed_zh_cn() {
        let msg = ErrorTemplates::download_failed(
            "https://example.com/file.jar",
            "连接超时",
            Language::ZhCN,
        );

        assert!(msg.summary.contains("下载"));
        assert!(msg.summary.contains("失败"));
        assert!(msg.reason.contains("连接超时"));
        assert!(msg.evidence.is_some());
        assert!(msg.retryable);
        assert!(msg.actions.iter().any(|a| a.contains("网络")));
    }

    #[test]
    fn download_failed_en() {
        let msg = ErrorTemplates::download_failed(
            "https://example.com/file.jar",
            "connection timeout",
            Language::En,
        );

        assert!(msg.summary.contains("Download"));
        assert!(msg.summary.contains("failed"));
        assert!(msg.retryable);
    }

    #[test]
    fn server_start_failed_zh_cn() {
        let msg = ErrorTemplates::server_start_failed("端口冲突", Language::ZhCN);

        assert!(msg.summary.contains("服务器"));
        assert!(msg.summary.contains("启动"));
        assert!(msg.summary.contains("失败"));
        assert!(msg.reason.contains("端口冲突"));
        assert!(!msg.retryable); // Server start failures are not automatically retryable
        assert!(msg.actions.len() >= 3);
        assert!(msg.actions.iter().any(|a| a.contains("日志")));
    }

    #[test]
    fn server_start_failed_en() {
        let msg = ErrorTemplates::server_start_failed("port conflict", Language::En);

        assert!(msg.summary.contains("Server"));
        assert!(msg.summary.contains("failed"));
        assert!(msg.reason.contains("port conflict"));
    }

    #[test]
    fn backup_failed_zh_cn() {
        let msg = ErrorTemplates::backup_failed("磁盘空间不足", Language::ZhCN);

        assert!(msg.summary.contains("备份"));
        assert!(msg.summary.contains("失败"));
        assert!(msg.reason.contains("磁盘空间不足"));
        assert!(msg.retryable);
        assert!(!msg.actions.is_empty());
    }

    #[test]
    fn backup_failed_en() {
        let msg = ErrorTemplates::backup_failed("disk space low", Language::En);

        assert!(msg.summary.contains("Backup"));
        assert!(msg.summary.contains("failed"));
        assert!(msg.retryable);
    }

    #[test]
    fn network_timeout_zh_cn() {
        let msg = ErrorTemplates::network_timeout("下载服务器核心", Language::ZhCN);

        assert!(msg.summary.contains("网络"));
        assert!(msg.summary.contains("超时"));
        assert!(msg.reason.contains("下载服务器核心"));
        assert!(msg.retryable);
    }

    #[test]
    fn network_timeout_en() {
        let msg = ErrorTemplates::network_timeout("downloading server", Language::En);

        assert!(msg.summary.contains("Network"));
        assert!(msg.summary.contains("timeout"));
        assert!(msg.retryable);
    }

    #[test]
    fn hash_mismatch_zh_cn() {
        let msg = ErrorTemplates::hash_mismatch("abc123", "def456", Language::ZhCN);

        assert!(msg.summary.contains("完整性"));
        assert!(msg.summary.contains("失败"));
        assert!(msg.evidence.is_some());
        let evidence = msg.evidence.unwrap();
        assert!(evidence.contains("abc123"));
        assert!(evidence.contains("def456"));
        assert!(msg.retryable);
    }

    #[test]
    fn hash_mismatch_en() {
        let msg = ErrorTemplates::hash_mismatch("abc123", "def456", Language::En);

        assert!(msg.summary.contains("integrity"));
        assert!(msg.summary.contains("failed"));
        assert!(msg.retryable);
    }

    #[test]
    fn permission_denied_zh_cn() {
        let msg = ErrorTemplates::permission_denied("/var/minecraft/world", "写入", Language::ZhCN);

        assert!(msg.summary.contains("权限"));
        assert!(msg.summary.contains("拒绝"));
        assert!(msg.reason.contains("/var/minecraft/world"));
        assert!(msg.reason.contains("写入"));
        assert!(!msg.retryable);
    }

    #[test]
    fn permission_denied_en() {
        let msg = ErrorTemplates::permission_denied("/var/minecraft/world", "write", Language::En);

        assert!(msg.summary.contains("Permission"));
        assert!(msg.summary.contains("denied"));
        assert!(msg.reason.contains("/var/minecraft/world"));
    }
}

mod language {
    use super::*;

    #[test]
    fn language_default_is_zh_cn() {
        assert_eq!(Language::default(), Language::ZhCN);
    }

    #[test]
    fn all_templates_support_both_languages() {
        // Smoke test to ensure all templates work with both languages
        let _ = ErrorTemplates::java_version_mismatch("21", "17", Language::En);
        let _ = ErrorTemplates::java_version_mismatch("21", "17", Language::ZhCN);

        let _ = ErrorTemplates::port_in_use(25565, Language::En);
        let _ = ErrorTemplates::port_in_use(25565, Language::ZhCN);

        let _ = ErrorTemplates::disk_space_low(1000, 100, Language::En);
        let _ = ErrorTemplates::disk_space_low(1000, 100, Language::ZhCN);

        let _ = ErrorTemplates::download_failed("url", "reason", Language::En);
        let _ = ErrorTemplates::download_failed("url", "reason", Language::ZhCN);

        let _ = ErrorTemplates::server_start_failed("reason", Language::En);
        let _ = ErrorTemplates::server_start_failed("reason", Language::ZhCN);

        let _ = ErrorTemplates::backup_failed("reason", Language::En);
        let _ = ErrorTemplates::backup_failed("reason", Language::ZhCN);

        let _ = ErrorTemplates::network_timeout("op", Language::En);
        let _ = ErrorTemplates::network_timeout("op", Language::ZhCN);

        let _ = ErrorTemplates::hash_mismatch("exp", "act", Language::En);
        let _ = ErrorTemplates::hash_mismatch("exp", "act", Language::ZhCN);

        let _ = ErrorTemplates::permission_denied("path", "op", Language::En);
        let _ = ErrorTemplates::permission_denied("path", "op", Language::ZhCN);
    }
}

#[test]
fn conversion_preserves_operation_evidence_and_recovery_policy() {
    use crate::{AppError, ErrorCode};
    for code in [
        ErrorCode::BackupFailed,
        ErrorCode::ServerStartFailed,
        ErrorCode::HashMismatch,
    ] {
        let error = AppError::new(code, "Actual operation failure")
            .with_evidence("expected_hash=abc, actual_hash=def")
            .with_action("Inspect the recovery journal before proceeding");
        let message = error.to_error_message(Language::ZhCN);
        assert_eq!(message.reason, error.reason);
        assert_eq!(
            message.actions[0],
            error.next_action.as_ref().expect("next action").as_str()
        );
        assert!(!message.retryable);
        let report = message.diagnostic_report();
        assert!(report.contains("expected_hash=abc, actual_hash=def"));
        assert!(report.contains(&code.to_string()));
    }
}

#[test]
fn disk_space_conversion_accepts_existing_evidence_formats() {
    use crate::{AppError, ErrorCode};
    for evidence in [
        "need 524288000 bytes, have 104857600",
        "need 524288000 bytes, have 104857600 bytes",
    ] {
        let message = AppError::new(ErrorCode::DiskSpaceLow, "Backup volume is full")
            .with_evidence(evidence)
            .retryable()
            .to_error_message(Language::En);
        assert_eq!(message.summary, "Insufficient disk space");
        assert!(message.actions.len() >= 3);
        assert!(message.retryable);
        assert!(message.diagnostic_report().contains(evidence));
    }
}
