use crate::storage::RuntimeRecord;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProcessIdentity {
    Matches,
    NotRunning,
    Unverified,
}

pub(crate) async fn inspect(record: &RuntimeRecord) -> Result<ProcessIdentity, mc_core::AppError> {
    #[cfg(target_os = "linux")]
    {
        let path = format!("/proc/{}/cmdline", record.pid);
        match tokio::fs::read(path).await {
            Ok(command_line) if command_line.is_empty() => Ok(ProcessIdentity::NotRunning),
            Ok(command_line) => {
                if command_line
                    .split(|byte| *byte == 0)
                    .any(|argument| argument == record.marker.as_bytes())
                {
                    Ok(ProcessIdentity::Matches)
                } else {
                    Ok(ProcessIdentity::Unverified)
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(ProcessIdentity::NotRunning)
            }
            Err(_) => Ok(ProcessIdentity::Unverified),
        }
    }

    #[cfg(target_os = "macos")]
    {
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            tokio::process::Command::new("/bin/ps")
                .args(["-ww", "-p", &record.pid.to_string(), "-o", "command="])
                .output(),
        )
        .await;
        match output {
            Ok(Ok(output)) if output.stdout.is_empty() && output.status.code() == Some(1) => {
                Ok(ProcessIdentity::NotRunning)
            }
            Ok(Ok(output)) if !output.status.success() => Ok(ProcessIdentity::Unverified),
            Ok(Ok(output)) if output.stdout.is_empty() => Ok(ProcessIdentity::NotRunning),
            Ok(Ok(output)) => {
                let command = String::from_utf8_lossy(&output.stdout);
                if command.contains(&record.marker) {
                    Ok(ProcessIdentity::Matches)
                } else {
                    Ok(ProcessIdentity::Unverified)
                }
            }
            _ => Ok(ProcessIdentity::Unverified),
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = record;
        Ok(ProcessIdentity::Unverified)
    }
}
