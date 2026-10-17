use crate::digest::hex_digest;
use crate::errors::io_error;
use crate::VerifiedBlob;
use fs2::FileExt;
use mc_core::{AppError, ArtifactPlan, ErrorCode};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use tokio::io::AsyncReadExt;

const COPY_BUFFER_SIZE: usize = 64 * 1024;

pub(crate) async fn verify_cached_blob(
    path: &Path,
    plan: &ArtifactPlan,
) -> Result<Option<VerifiedBlob>, AppError> {
    let metadata = match tokio::fs::symlink_metadata(path).await {
        Ok(metadata) if metadata.file_type().is_file() => metadata,
        Ok(_) => {
            return Err(AppError::new(
                ErrorCode::Internal,
                "Cache entry is not a regular file.",
            ))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(io_error("Could not inspect cache entry.", error)),
    };
    if metadata.len() != plan.expected_size {
        tokio::fs::remove_file(path)
            .await
            .map_err(|error| io_error("Could not remove invalid cache entry.", error))?;
        return Ok(None);
    }
    let actual_hash = hash_file(path).await?;
    if !actual_hash.eq_ignore_ascii_case(&plan.checksum.value) {
        tokio::fs::remove_file(path)
            .await
            .map_err(|error| io_error("Could not remove invalid cache entry.", error))?;
        return Ok(None);
    }
    Ok(Some(VerifiedBlob {
        path: path.to_path_buf(),
        size: metadata.len(),
        sha256: actual_hash,
    }))
}

async fn hash_file(path: &Path) -> Result<String, AppError> {
    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|error| io_error("Could not read cached artifact.", error))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; COPY_BUFFER_SIZE];
    loop {
        let count = file
            .read(&mut buffer)
            .await
            .map_err(|error| io_error("Could not hash cached artifact.", error))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hex_digest(hasher.finalize().as_slice()))
}

pub(crate) async fn prepare_cache_root(root: &Path) -> Result<PathBuf, AppError> {
    tokio::fs::create_dir_all(root)
        .await
        .map_err(|error| io_error("Could not create artifact cache directory.", error))?;
    let base = tokio::fs::canonicalize(root)
        .await
        .map_err(|error| io_error("Could not resolve artifact cache directory.", error))?;
    let metadata = tokio::fs::symlink_metadata(&base)
        .await
        .map_err(|error| io_error("Could not inspect artifact cache base directory.", error))?;
    if !metadata.file_type().is_dir() {
        return Err(AppError::new(
            ErrorCode::Internal,
            "Artifact cache base path is not a real directory.",
        ));
    }
    // Keep the caller-owned base untouched. All private permissions apply to
    // this namespaced directory and its descendants only.
    let managed_root = base.join("mc-server-tui-artifacts");
    ensure_private_dir(&managed_root).await?;
    Ok(managed_root)
}

pub(crate) async fn ensure_private_dir(path: &Path) -> Result<(), AppError> {
    tokio::fs::create_dir_all(path)
        .await
        .map_err(|error| io_error("Could not create cache directory.", error))?;
    let metadata = tokio::fs::symlink_metadata(path)
        .await
        .map_err(|error| io_error("Could not inspect cache directory.", error))?;
    if !metadata.file_type().is_dir() {
        return Err(AppError::new(
            ErrorCode::Internal,
            "A cache directory path is not a real directory.",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .await
            .map_err(|error| io_error("Could not protect cache directory permissions.", error))?;
    }
    Ok(())
}

pub(crate) async fn acquire_lock(path: PathBuf) -> Result<File, AppError> {
    tokio::task::spawn_blocking(move || {
        let mut options = OpenOptions::new();
        options.create(true).read(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW);
            options.mode(0o600);
        }
        let file = options
            .open(path)
            .map_err(|error| io_error("Could not open artifact cache lock.", error))?;
        file.lock_exclusive()
            .map_err(|error| io_error("Could not lock artifact cache entry.", error))?;
        Ok(file)
    })
    .await
    .map_err(|_| AppError::new(ErrorCode::Internal, "Artifact cache lock task failed."))?
}

pub(crate) async fn check_available_space(root: &Path, expected_size: u64) -> Result<(), AppError> {
    let path = root.to_path_buf();
    let available = tokio::task::spawn_blocking(move || fs2::available_space(path))
        .await
        .map_err(|_| AppError::new(ErrorCode::Internal, "Disk space check task failed."))?
        .map_err(|error| io_error("Could not check available disk space.", error))?;
    if available < expected_size {
        return Err(AppError::new(
            ErrorCode::DiskSpaceLow,
            "There is not enough free disk space for this artifact.",
        )
        .with_evidence(format!("required={expected_size}, available={available}")));
    }
    Ok(())
}

pub(crate) async fn discard_partial(part: &Path, metadata: &Path) -> Result<(), AppError> {
    remove_if_regular_file(part).await?;
    remove_if_regular_file(metadata).await?;
    Ok(())
}

pub(crate) async fn remove_if_regular_file(path: &Path) -> Result<(), AppError> {
    match tokio::fs::symlink_metadata(path).await {
        Ok(metadata) if metadata.file_type().is_file() => tokio::fs::remove_file(path)
            .await
            .map_err(|error| io_error("Could not remove temporary download data.", error)),
        Ok(_) => Err(AppError::new(
            ErrorCode::Internal,
            "Temporary download path is not a regular file.",
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error(
            "Could not inspect temporary download data.",
            error,
        )),
    }
}

pub(crate) async fn sync_directory(path: PathBuf) -> Result<(), AppError> {
    tokio::task::spawn_blocking(move || {
        File::open(path)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| io_error("Could not sync artifact cache directory.", error))
    })
    .await
    .map_err(|_| AppError::new(ErrorCode::Internal, "Cache sync task failed."))?
}
