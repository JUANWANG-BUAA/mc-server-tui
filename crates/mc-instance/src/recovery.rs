use crate::filesystem::{ensure_managed_directory, io_error};
use crate::model::{InstanceConfig, InstanceLock};
use mc_core::{AppError, ErrorCode, InstanceSummary};
use std::path::Path;
use tokio::io::AsyncReadExt;
use uuid::Uuid;

pub(crate) async fn recover_instances(root: &Path) -> Result<Vec<InstanceSummary>, AppError> {
    let quarantine = root.join("quarantine");
    ensure_managed_directory(&quarantine).await?;
    let mut directory = tokio::fs::read_dir(root)
        .await
        .map_err(|error| io_error("Could not scan instance directory.", error))?;
    let mut summaries = Vec::new();
    while let Some(entry) = directory
        .next_entry()
        .await
        .map_err(|error| io_error("Could not scan instance directory.", error))?
    {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "quarantine" {
            continue;
        }
        if name.starts_with(".staging-") {
            quarantine_path(&path, &quarantine, &name).await?;
            continue;
        }
        let Ok(id) = Uuid::parse_str(&name) else {
            continue;
        };
        let file_type = entry
            .file_type()
            .await
            .map_err(|error| io_error("Could not inspect instance directory entry.", error))?;
        if !file_type.is_dir() {
            quarantine_path(&path, &quarantine, &name).await?;
            continue;
        }
        match read_instance(&path, id).await {
            Ok(summary) => summaries.push(summary),
            Err(_) => quarantine_path(&path, &quarantine, &name).await?,
        }
    }
    summaries.sort_by(|left, right| {
        left.created_at
            .cmp(&right.created_at)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok(summaries)
}

async fn read_instance(directory: &Path, id: Uuid) -> Result<InstanceSummary, AppError> {
    let config_bytes = read_regular_file(&directory.join("instance.toml")).await?;
    let config: InstanceConfig =
        toml::from_str(std::str::from_utf8(&config_bytes).map_err(|_| {
            AppError::new(
                ErrorCode::SourceSchemaChanged,
                "Instance configuration is not valid UTF-8.",
            )
        })?)
        .map_err(|error| {
            AppError::new(
                ErrorCode::SourceSchemaChanged,
                "Instance configuration could not be parsed.",
            )
            .with_evidence(error.to_string())
        })?;
    config.validate()?;
    if config.id != id {
        return Err(AppError::new(
            ErrorCode::SourceSchemaChanged,
            "Instance directory ID does not match its configuration.",
        ));
    }
    let lock_bytes = read_regular_file(&directory.join("lock.json")).await?;
    let lock: InstanceLock = serde_json::from_slice(&lock_bytes).map_err(|error| {
        AppError::new(
            ErrorCode::SourceSchemaChanged,
            "Instance lock file could not be parsed.",
        )
        .with_evidence(error.to_string())
    })?;
    lock.validate(&config)?;
    let server_jar = directory.join("server").join("server.jar");
    let metadata = tokio::fs::symlink_metadata(&server_jar)
        .await
        .map_err(|error| io_error("Installed server artifact is unavailable.", error))?;
    if !metadata.file_type().is_file() || metadata.len() != lock.artifact_size {
        return Err(AppError::new(
            ErrorCode::SourceSchemaChanged,
            "Installed server artifact does not match the instance lock size.",
        ));
    }
    Ok(config.summary(directory.to_path_buf()))
}

async fn read_regular_file(path: &Path) -> Result<Vec<u8>, AppError> {
    let metadata = tokio::fs::symlink_metadata(path)
        .await
        .map_err(|error| io_error("Could not inspect instance metadata file.", error))?;
    if !metadata.file_type().is_file() {
        return Err(AppError::new(
            ErrorCode::SourceSchemaChanged,
            "Instance metadata path is not a regular file.",
        ));
    }
    let mut options = tokio::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    options.custom_flags(libc::O_NOFOLLOW);
    let file = options
        .open(path)
        .await
        .map_err(|error| io_error("Could not open instance metadata file.", error))?;
    let mut contents = Vec::new();
    file.take(1024 * 1024 + 1)
        .read_to_end(&mut contents)
        .await
        .map_err(|error| io_error("Could not read instance metadata file.", error))?;
    if contents.len() > 1024 * 1024 {
        return Err(AppError::new(
            ErrorCode::RequestTooLarge,
            "Instance metadata file exceeds the 1 MiB limit.",
        ));
    }
    Ok(contents)
}

async fn quarantine_path(
    path: &Path,
    quarantine: &Path,
    original_name: &str,
) -> Result<(), AppError> {
    let target = quarantine.join(format!("{original_name}-{}", Uuid::new_v4()));
    tokio::fs::rename(path, target)
        .await
        .map_err(|error| io_error("Could not quarantine incomplete instance data.", error))
}
