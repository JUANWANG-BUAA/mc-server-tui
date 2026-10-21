//! Integration example: How to use the new error handling system.
//!
//! This example demonstrates best practices for error handling in the TUI.

use mc_core::{AppError, ErrorCode, ErrorMessage, ErrorTemplates, Language};

/// Example: Converting AppError to user-friendly message
pub fn example_app_error_conversion() {
    // Scenario 1: Disk space error with parsed parameters
    let app_error = AppError::new(
        ErrorCode::DiskSpaceLow,
        "Not enough free space for a world backup",
    )
    .with_evidence("need 524288000 bytes, have 104857600")
    .with_action("Free space on the instance volume, then retry the backup.")
    .retryable();

    // Convert to user-friendly message
    let message = app_error.to_error_message(Language::ZhCN);

    // The system automatically detects the pattern and uses the template
    assert!(message.summary.contains("磁盘空间不足"));
    assert!(message.retryable);
}

/// Example: Using error templates directly
pub fn example_error_templates() {
    // Java version mismatch
    let java_error = ErrorTemplates::java_version_mismatch("21", "17", Language::ZhCN);
    println!("{}", java_error.summary);

    // Port in use
    let port_error = ErrorTemplates::port_in_use(25565, Language::ZhCN);
    println!("{}", port_error.summary);

    // Download failed
    let download_error = ErrorTemplates::download_failed(
        "https://api.papermc.io/v2/projects/paper/versions/1.20.4/builds/497/downloads/paper-1.20.4-497.jar",
        "Connection timeout after 30s",
        Language::ZhCN,
    );
    println!("{}", download_error.summary);
}

/// Example: Building custom error messages
pub fn example_custom_error_message() {
    let error = ErrorMessage::new(
        "配置文件格式错误",
        "server.properties 文件包含无效的键值对"
    )
    .with_evidence("line 42: invalid-key without value")
    .with_actions([
        "打开 server.properties 文件",
        "检查第 42 行的配置",
        "确保每行格式为 key=value",
        "保存文件后重启服务器",
    ]);

    // Generate diagnostic report
    let report = error.diagnostic_report();
    println!("{}", report);
}

/// Example: Error dialog state management in TUI
#[cfg(feature = "tui")]
pub mod tui_integration {
    use super::*;

    pub struct AppState {
        pub error_dialog: Option<crate::error_dialog::ErrorDialogState>,
    }

    impl AppState {
        /// Show an error dialog
        pub fn show_error(&mut self, error: ErrorMessage) {
            self.error_dialog = Some(crate::error_dialog::ErrorDialogState::new(error));
        }

        /// Handle error dialog input
        pub fn handle_error_dialog_input(&mut self, key: char) -> bool {
            if let Some(dialog) = &mut self.error_dialog {
                match key {
                    'd' => {
                        dialog.toggle_details();
                        true
                    }
                    'c' => {
                        let report = dialog.diagnostic_report();
                        // TODO: Copy to clipboard
                        println!("Copied to clipboard:\n{}", report);
                        true
                    }
                    'r' if dialog.message.retryable => {
                        // TODO: Retry the operation
                        self.error_dialog = None;
                        true
                    }
                    '\x1b' => { // Escape
                        self.error_dialog = None;
                        true
                    }
                    _ => false,
                }
            } else {
                false
            }
        }
    }
}

/// Example: Error handling in async operations
pub async fn example_async_operation_with_error_handling() -> Result<(), AppError> {
    // Simulate an operation that might fail
    let result = check_disk_space().await;

    match result {
        Ok(_) => Ok(()),
        Err(error) => {
            // Convert to user-friendly message for display
            let _message = error.to_error_message(Language::ZhCN);
            // In real code, you would show this in the UI
            Err(error)
        }
    }
}

async fn check_disk_space() -> Result<(), AppError> {
    let needed = 1024 * 1024 * 500; // 500 MB
    let available = 1024 * 1024 * 100; // 100 MB

    if available < needed {
        return Err(
            AppError::new(
                ErrorCode::DiskSpaceLow,
                format!("Need {} MB, have {} MB", needed / (1024 * 1024), available / (1024 * 1024))
            )
            .with_evidence(format!("need {} bytes, have {}", needed, available))
            .with_action("Free space on the disk")
            .retryable()
        );
    }

    Ok(())
}

/// Example: Chaining errors with context
pub fn example_error_context() {
    // Original error
    let io_error = std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "Permission denied"
    );

    // Wrap with context
    let app_error = AppError::new(
        ErrorCode::InvalidInput,
        "Failed to read server configuration"
    )
    .with_evidence(format!("io_error: {}", io_error))
    .with_action("Check file permissions on server.properties");

    let message = app_error.to_error_message(Language::ZhCN);
    println!("{}", message.summary);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_error_conversion() {
        example_app_error_conversion();
    }

    #[test]
    fn test_error_templates() {
        example_error_templates();
    }

    #[test]
    fn test_custom_error_message() {
        example_custom_error_message();
    }

    #[tokio::test]
    async fn test_async_operation() {
        let result = example_async_operation_with_error_handling().await;
        assert!(result.is_err());
    }

    #[test]
    fn test_error_context() {
        example_error_context();
    }
}
