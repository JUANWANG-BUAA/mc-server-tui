use mc_core::{AppError, ErrorCode};
use std::{fs, path::Path};

pub(crate) async fn ensure_managed_directory(path: &Path) -> Result<(), AppError> {
    tokio::fs::create_dir_all(path)
        .await
        .map_err(|error| io_error("Could not create instance directory.", error))?;
    let metadata = tokio::fs::symlink_metadata(path)
        .await
        .map_err(|error| io_error("Could not inspect instance directory.", error))?;
    if !metadata.file_type().is_dir() {
        return Err(AppError::new(
            ErrorCode::Internal,
            "Instance storage path is not a real directory.",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .await
            .map_err(|error| io_error("Could not protect instance directory.", error))?;
    }
    Ok(())
}

pub(crate) async fn create_staging_directory(path: &Path) -> Result<(), AppError> {
    tokio::fs::create_dir(path)
        .await
        .map_err(|error| io_error("Could not create a unique staging directory.", error))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .await
            .map_err(|error| io_error("Could not protect the staging directory.", error))?;
    }
    Ok(())
}

pub(crate) async fn sync_directory(path: &Path) -> Result<(), AppError> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        fs::File::open(path)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| io_error("Could not sync instance directory.", error))
    })
    .await
    .map_err(|_| AppError::new(ErrorCode::Internal, "Instance sync task failed."))?
}

pub(crate) async fn write_synced(path: &Path, contents: &[u8]) -> Result<(), AppError> {
    use tokio::io::AsyncWriteExt;

    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(path)
        .await
        .map_err(|error| io_error("Could not create instance data file.", error))?;
    file.write_all(contents)
        .await
        .map_err(|error| io_error("Could not write instance data file.", error))?;
    file.sync_all()
        .await
        .map_err(|error| io_error("Could not sync instance data file.", error))
}

pub(crate) fn io_error(reason: &str, error: std::io::Error) -> AppError {
    AppError::new(ErrorCode::Internal, reason).with_evidence(error.kind().to_string())
}
