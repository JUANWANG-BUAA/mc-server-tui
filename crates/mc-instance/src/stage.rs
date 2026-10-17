use crate::{
    filesystem::{ensure_managed_directory, sync_directory, write_synced},
    model::{InstanceConfig, InstanceLock},
};
use mc_core::{AppError, ArtifactPlan, ErrorCode};
use mc_download::VerifiedBlob;
use sha2::{Digest, Sha256};
use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const COPY_BUFFER_SIZE: usize = 64 * 1024;

pub(crate) async fn write_staged_instance(
    staging_path: &Path,
    plan: &ArtifactPlan,
    blob: &VerifiedBlob,
    config: &InstanceConfig,
    lock: &InstanceLock,
) -> Result<(), AppError> {
    for name in ["server", "logs", "backups", "tmp"] {
        ensure_managed_directory(&staging_path.join(name)).await?;
    }
    let server_path = staging_path.join("server");
    copy_and_verify_blob(blob, plan, &server_path.join("server.jar")).await?;

    let config_text = toml::to_string_pretty(config).map_err(|error| {
        AppError::new(
            ErrorCode::Internal,
            "Could not encode instance configuration.",
        )
        .with_evidence(error.to_string())
    })?;
    let lock_json = serde_json::to_vec_pretty(lock).map_err(|error| {
        AppError::new(ErrorCode::Internal, "Could not encode instance lock file.")
            .with_evidence(error.to_string())
    })?;
    write_synced(&staging_path.join("instance.toml"), config_text.as_bytes()).await?;
    write_synced(&staging_path.join("lock.json"), &lock_json).await?;
    write_synced(&server_path.join("eula.txt"), b"eula=true\n").await?;

    for directory in [
        server_path,
        staging_path.join("logs"),
        staging_path.join("backups"),
        staging_path.join("tmp"),
        staging_path.to_path_buf(),
    ] {
        sync_directory(&directory).await?;
    }
    Ok(())
}

async fn copy_and_verify_blob(
    blob: &VerifiedBlob,
    plan: &ArtifactPlan,
    destination: &Path,
) -> Result<(), AppError> {
    let metadata = tokio::fs::symlink_metadata(blob.path())
        .await
        .map_err(|error| io_error("Could not inspect verified artifact.", error))?;
    if !metadata.file_type().is_file() || metadata.len() != plan.expected_size {
        return Err(AppError::new(
            ErrorCode::HashMismatch,
            "Verified artifact changed before instance staging.",
        ));
    }

    let mut source_options = tokio::fs::OpenOptions::new();
    source_options.read(true);
    let mut destination_options = tokio::fs::OpenOptions::new();
    destination_options.write(true).create_new(true);
    #[cfg(unix)]
    {
        destination_options.mode(0o600);
        source_options.custom_flags(libc::O_NOFOLLOW);
        destination_options.custom_flags(libc::O_NOFOLLOW);
    }
    let mut source = source_options
        .open(blob.path())
        .await
        .map_err(|error| io_error("Could not open verified artifact.", error))?;
    let mut destination = destination_options
        .open(destination)
        .await
        .map_err(|error| io_error("Could not create staged server artifact.", error))?;

    let mut hasher = Sha256::new();
    let mut copied = 0u64;
    let mut buffer = vec![0; COPY_BUFFER_SIZE];
    loop {
        let count = source
            .read(&mut buffer)
            .await
            .map_err(|error| io_error("Could not read verified artifact.", error))?;
        if count == 0 {
            break;
        }
        copied = copied.checked_add(count as u64).ok_or_else(|| {
            AppError::new(
                ErrorCode::DownloadTooLarge,
                "Staged artifact size overflowed.",
            )
        })?;
        if copied > plan.expected_size {
            return Err(AppError::new(
                ErrorCode::DownloadTooLarge,
                "Staged artifact exceeds the exact plan size.",
            ));
        }
        hasher.update(&buffer[..count]);
        destination
            .write_all(&buffer[..count])
            .await
            .map_err(|error| io_error("Could not write staged server artifact.", error))?;
    }
    if copied != plan.expected_size {
        return Err(AppError::new(
            ErrorCode::HashMismatch,
            "Staged artifact length does not match its exact plan.",
        ));
    }
    let actual_sha256 = hex_digest(hasher.finalize().as_slice());
    if !actual_sha256.eq_ignore_ascii_case(&plan.checksum.value)
        || !actual_sha256.eq_ignore_ascii_case(blob.sha256())
    {
        return Err(AppError::new(
            ErrorCode::HashMismatch,
            "Staged artifact SHA-256 does not match its exact plan.",
        ));
    }
    destination
        .flush()
        .await
        .map_err(|error| io_error("Could not flush staged server artifact.", error))?;
    destination
        .sync_all()
        .await
        .map_err(|error| io_error("Could not sync staged server artifact.", error))
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(result, "{byte:02x}");
    }
    result
}

fn io_error(reason: &str, error: std::io::Error) -> AppError {
    AppError::new(ErrorCode::Internal, reason).with_evidence(error.kind().to_string())
}
