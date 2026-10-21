//! Human-friendly error messages with structured three-part format.
//!
//! Each error message follows a three-part structure:
//! 1. **What happened** (user-friendly summary)
//! 2. **Why it happened** (technical details with evidence)
//! 3. **What to do next** (actionable steps)

use serde::{Deserialize, Serialize};

/// Language preference for error messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    /// English
    En,
    /// Simplified Chinese (简体中文)
    ZhCN,
}

impl Default for Language {
    fn default() -> Self {
        Self::ZhCN
    }
}

/// A human-friendly error message with three-part structure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorMessage {
    /// What happened (user-friendly summary)
    pub summary: String,
    /// Why it happened (technical reason)
    pub reason: String,
    /// Technical evidence (can be collapsed in UI)
    pub evidence: Option<String>,
    /// What to do next (actionable steps)
    pub actions: Vec<String>,
    /// Whether the operation can be retried
    pub retryable: bool,
}

impl ErrorMessage {
    pub fn new(summary: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
            reason: reason.into(),
            evidence: None,
            actions: Vec::new(),
            retryable: false,
        }
    }

    pub fn with_evidence(mut self, evidence: impl Into<String>) -> Self {
        self.evidence = Some(evidence.into());
        self
    }

    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.actions.push(action.into());
        self
    }

    pub fn with_actions<I, S>(mut self, actions: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.actions.extend(actions.into_iter().map(Into::into));
        self
    }

    pub fn retryable(mut self) -> Self {
        self.retryable = true;
        self
    }

    /// Generate a diagnostic report for copying to clipboard.
    pub fn diagnostic_report(&self) -> String {
        let mut report = String::new();
        report.push_str("=== 错误诊断报告 | Error Diagnostic Report ===\n\n");

        report.push_str("问题 | Issue:\n");
        report.push_str(&self.summary);
        report.push_str("\n\n");

        report.push_str("原因 | Reason:\n");
        report.push_str(&self.reason);
        report.push_str("\n\n");

        if let Some(evidence) = &self.evidence {
            report.push_str("技术细节 | Technical Details:\n");
            report.push_str(evidence);
            report.push_str("\n\n");
        }

        if !self.actions.is_empty() {
            report.push_str("建议操作 | Recommended Actions:\n");
            for (i, action) in self.actions.iter().enumerate() {
                report.push_str(&format!("{}. {}\n", i + 1, action));
            }
            report.push_str("\n");
        }

        report.push_str("可重试 | Retryable: ");
        report.push_str(if self.retryable { "是 | Yes" } else { "否 | No" });
        report.push_str("\n\n");

        report.push_str(&format!(
            "生成时间 | Generated: {}\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));

        report
    }
}

/// Error message templates for common scenarios.
pub struct ErrorTemplates;

impl ErrorTemplates {
    /// Java version mismatch error.
    pub fn java_version_mismatch(
        required: &str,
        found: &str,
        lang: Language,
    ) -> ErrorMessage {
        match lang {
            Language::En => ErrorMessage::new(
                "Java version mismatch",
                format!("Server requires Java {required}, but found Java {found}"),
            )
            .with_evidence(format!(
                "java_version_required={required}, java_version_found={found}"
            ))
            .with_actions([
                format!("Install Java {required} from https://adoptium.net"),
                "Set JAVA_HOME environment variable to the correct Java installation".to_string(),
                "Or use the TUI to configure a custom Java path".to_string(),
            ]),
            Language::ZhCN => ErrorMessage::new(
                "Java 版本不匹配",
                format!("服务器需要 Java {required}，但发现 Java {found}"),
            )
            .with_evidence(format!(
                "java_version_required={required}, java_version_found={found}"
            ))
            .with_actions([
                format!("从 https://adoptium.net 安装 Java {required}"),
                "设置 JAVA_HOME 环境变量指向正确的 Java 安装位置".to_string(),
                "或使用 TUI 配置自定义 Java 路径".to_string(),
            ]),
        }
    }

    /// Port already in use error.
    pub fn port_in_use(port: u16, lang: Language) -> ErrorMessage {
        match lang {
            Language::En => ErrorMessage::new(
                "Port already in use",
                format!("Cannot start server because port {port} is already occupied"),
            )
            .with_evidence(format!("port={port}, error=EADDRINUSE"))
            .with_actions([
                format!("Stop the process using port {port}"),
                format!("Change the server port in server.properties (server-port={port})"),
                "Use 'lsof -i :{port}' (macOS/Linux) or 'netstat -ano | findstr :{port}' (Windows) to find the process"
                    .replace("{port}", &port.to_string()),
            ])
            .retryable(),
            Language::ZhCN => ErrorMessage::new(
                "端口已被占用",
                format!("无法启动服务器，因为端口 {port} 已被占用"),
            )
            .with_evidence(format!("port={port}, error=EADDRINUSE"))
            .with_actions([
                format!("停止占用端口 {port} 的进程"),
                format!("在 server.properties 中修改服务器端口 (server-port={port})"),
                "使用 'lsof -i :{port}' (macOS/Linux) 或 'netstat -ano | findstr :{port}' (Windows) 查找占用进程"
                    .replace("{port}", &port.to_string()),
            ])
            .retryable(),
        }
    }

    /// Disk space low error.
    pub fn disk_space_low(needed: u64, available: u64, lang: Language) -> ErrorMessage {
        let needed_mb = needed / (1024 * 1024);
        let available_mb = available / (1024 * 1024);

        match lang {
            Language::En => ErrorMessage::new(
                "Insufficient disk space",
                format!("Need {needed_mb} MB, but only {available_mb} MB available"),
            )
            .with_evidence(format!(
                "required_bytes={needed}, available_bytes={available}"
            ))
            .with_actions([
                "Free up disk space by deleting unnecessary files".to_string(),
                "Remove old backups or world saves".to_string(),
                "Move the server instance to a volume with more space".to_string(),
            ])
            .retryable(),
            Language::ZhCN => ErrorMessage::new(
                "磁盘空间不足",
                format!("需要 {needed_mb} MB，但只有 {available_mb} MB 可用"),
            )
            .with_evidence(format!(
                "required_bytes={needed}, available_bytes={available}"
            ))
            .with_actions([
                "删除不必要的文件以释放磁盘空间".to_string(),
                "删除旧备份或世界存档".to_string(),
                "将服务器实例移动到空间更大的磁盘".to_string(),
            ])
            .retryable(),
        }
    }

    /// Download failed error.
    pub fn download_failed(url: &str, reason: &str, lang: Language) -> ErrorMessage {
        match lang {
            Language::En => ErrorMessage::new(
                "Download failed",
                format!("Failed to download server file: {reason}"),
            )
            .with_evidence(format!("url={url}, error={reason}"))
            .with_actions([
                "Check your network connection".to_string(),
                "Verify the download URL is accessible".to_string(),
                "Try again after a short delay".to_string(),
                "Check if a proxy or firewall is blocking the connection".to_string(),
            ])
            .retryable(),
            Language::ZhCN => ErrorMessage::new(
                "下载失败",
                format!("无法下载服务器文件：{reason}"),
            )
            .with_evidence(format!("url={url}, error={reason}"))
            .with_actions([
                "检查网络连接".to_string(),
                "验证下载链接是否可访问".to_string(),
                "稍后重试".to_string(),
                "检查代理或防火墙是否阻止了连接".to_string(),
            ])
            .retryable(),
        }
    }

    /// Server start failed error.
    pub fn server_start_failed(reason: &str, lang: Language) -> ErrorMessage {
        match lang {
            Language::En => ErrorMessage::new(
                "Server failed to start",
                format!("The Minecraft server process exited unexpectedly: {reason}"),
            )
            .with_evidence(reason.to_string())
            .with_actions([
                "Check the server logs for detailed error messages".to_string(),
                "Verify Java version compatibility".to_string(),
                "Ensure eula.txt is accepted (eula=true)".to_string(),
                "Check for port conflicts".to_string(),
                "Verify world files are not corrupted".to_string(),
            ]),
            Language::ZhCN => ErrorMessage::new(
                "服务器启动失败",
                format!("Minecraft 服务器进程意外退出：{reason}"),
            )
            .with_evidence(reason.to_string())
            .with_actions([
                "查看服务器日志了解详细错误信息".to_string(),
                "验证 Java 版本兼容性".to_string(),
                "确保已接受 EULA 协议 (eula=true)".to_string(),
                "检查端口冲突".to_string(),
                "验证世界文件是否损坏".to_string(),
            ]),
        }
    }

    /// Backup failed error.
    pub fn backup_failed(reason: &str, lang: Language) -> ErrorMessage {
        match lang {
            Language::En => ErrorMessage::new(
                "Backup failed",
                format!("Failed to create world backup: {reason}"),
            )
            .with_evidence(reason.to_string())
            .with_actions([
                "Ensure the server is stopped before backing up".to_string(),
                "Check available disk space".to_string(),
                "Verify world directory permissions".to_string(),
                "Try backing up individual worlds instead of all at once".to_string(),
            ])
            .retryable(),
            Language::ZhCN => ErrorMessage::new(
                "备份失败",
                format!("无法创建世界备份：{reason}"),
            )
            .with_evidence(reason.to_string())
            .with_actions([
                "确保服务器已停止再进行备份".to_string(),
                "检查可用磁盘空间".to_string(),
                "验证世界目录权限".to_string(),
                "尝试逐个备份世界，而不是一次性备份所有".to_string(),
            ])
            .retryable(),
        }
    }

    /// Network timeout error.
    pub fn network_timeout(operation: &str, lang: Language) -> ErrorMessage {
        match lang {
            Language::En => ErrorMessage::new(
                "Network timeout",
                format!("Operation timed out: {operation}"),
            )
            .with_evidence(format!("operation={operation}, error=timeout"))
            .with_actions([
                "Check your internet connection".to_string(),
                "Try again later when network conditions improve".to_string(),
                "Consider using a different mirror or download source".to_string(),
            ])
            .retryable(),
            Language::ZhCN => ErrorMessage::new(
                "网络超时",
                format!("操作超时：{operation}"),
            )
            .with_evidence(format!("operation={operation}, error=timeout"))
            .with_actions([
                "检查互联网连接".to_string(),
                "网络状况改善后重试".to_string(),
                "考虑使用其他镜像或下载源".to_string(),
            ])
            .retryable(),
        }
    }

    /// Hash verification failed error.
    pub fn hash_mismatch(expected: &str, actual: &str, lang: Language) -> ErrorMessage {
        match lang {
            Language::En => ErrorMessage::new(
                "File integrity check failed",
                "Downloaded file hash does not match expected value",
            )
            .with_evidence(format!(
                "expected_hash={expected}, actual_hash={actual}"
            ))
            .with_actions([
                "Delete the corrupted file and retry the download".to_string(),
                "Check if the download source is trustworthy".to_string(),
                "Verify your network connection is stable".to_string(),
            ])
            .retryable(),
            Language::ZhCN => ErrorMessage::new(
                "文件完整性检查失败",
                "下载文件的哈希值与预期不符",
            )
            .with_evidence(format!(
                "expected_hash={expected}, actual_hash={actual}"
            ))
            .with_actions([
                "删除损坏的文件并重新下载".to_string(),
                "检查下载源是否可信".to_string(),
                "验证网络连接是否稳定".to_string(),
            ])
            .retryable(),
        }
    }

    /// Permission denied error.
    pub fn permission_denied(path: &str, operation: &str, lang: Language) -> ErrorMessage {
        match lang {
            Language::En => ErrorMessage::new(
                "Permission denied",
                format!("Cannot {operation}: {path}"),
            )
            .with_evidence(format!("path={path}, operation={operation}, error=EACCES"))
            .with_actions([
                "Check file and directory permissions".to_string(),
                format!("Ensure the user has write access to {path}"),
                "Run with appropriate permissions or change the instance location".to_string(),
            ]),
            Language::ZhCN => ErrorMessage::new(
                "权限被拒绝",
                format!("无法{operation}：{path}"),
            )
            .with_evidence(format!("path={path}, operation={operation}, error=EACCES"))
            .with_actions([
                "检查文件和目录权限".to_string(),
                format!("确保用户对 {path} 有写入权限"),
                "使用适当的权限运行或更改实例位置".to_string(),
            ]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_message_builder() {
        let msg = ErrorMessage::new("Something went wrong", "The reason is X")
            .with_evidence("error_code=123")
            .with_action("Try this")
            .with_action("Or try that")
            .retryable();

        assert_eq!(msg.summary, "Something went wrong");
        assert_eq!(msg.reason, "The reason is X");
        assert_eq!(msg.evidence, Some("error_code=123".to_string()));
        assert_eq!(msg.actions.len(), 2);
        assert!(msg.retryable);
    }

    #[test]
    fn diagnostic_report_generation() {
        let msg = ErrorMessage::new("Test error", "Test reason")
            .with_evidence("test_evidence=123")
            .with_actions(["Action 1", "Action 2"])
            .retryable();

        let report = msg.diagnostic_report();
        assert!(report.contains("Test error"));
        assert!(report.contains("Test reason"));
        assert!(report.contains("test_evidence=123"));
        assert!(report.contains("Action 1"));
        assert!(report.contains("可重试"));
    }

    #[test]
    fn java_version_mismatch_template() {
        let msg = ErrorTemplates::java_version_mismatch("21", "17", Language::ZhCN);
        assert!(msg.summary.contains("Java 版本不匹配"));
        assert!(msg.reason.contains("21"));
        assert!(msg.reason.contains("17"));
        assert!(!msg.actions.is_empty());
    }

    #[test]
    fn port_in_use_template() {
        let msg = ErrorTemplates::port_in_use(25565, Language::En);
        assert!(msg.summary.contains("Port already in use"));
        assert!(msg.reason.contains("25565"));
        assert!(msg.retryable);
    }

    #[test]
    fn disk_space_low_template() {
        let msg = ErrorTemplates::disk_space_low(
            1024 * 1024 * 500, // 500 MB needed
            1024 * 1024 * 100, // 100 MB available
            Language::ZhCN,
        );
        assert!(msg.summary.contains("磁盘空间不足"));
        assert!(msg.reason.contains("500"));
        assert!(msg.reason.contains("100"));
    }
}
