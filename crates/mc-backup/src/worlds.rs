use std::{fs, io, path::Path};

use crate::{
    error::{io_error, BackupError},
    manifest::is_safe_world_name,
};

const DEFAULT_LEVEL_NAME: &str = "world";

/// Finds the primary world and any Paper/Vanilla sibling dimension folders.
///
/// The caller must hold a [`mc_supervisor::StoppedInstanceGuard`] for the
/// complete backup or restore operation so the server cannot mutate the tree.
pub fn discover_worlds(server_dir: &Path) -> Result<Vec<String>, BackupError> {
    ensure_real_directory(server_dir)?;
    let level_name = read_level_name(server_dir)?;
    if !is_safe_world_name(&level_name) {
        return Err(BackupError::InvalidWorldName(level_name));
    }

    let mut worlds = Vec::new();
    for candidate in [
        level_name.clone(),
        format!("{level_name}_nether"),
        format!("{level_name}_the_end"),
    ] {
        let path = server_dir.join(&candidate);
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(BackupError::UnsafeFilesystemEntry(path));
            }
            Ok(metadata) if metadata.is_dir() => worlds.push(candidate),
            Ok(_) if candidate == level_name => {
                return Err(BackupError::WorldNotFound(candidate));
            }
            Ok(_) => return Err(BackupError::UnsafeFilesystemEntry(path)),
            Err(error) if error.kind() == io::ErrorKind::NotFound && candidate == level_name => {
                return Err(BackupError::WorldNotFound(candidate));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(io_error("Could not inspect a world directory", path, error)),
        }
    }

    if worlds.is_empty() {
        return Err(BackupError::NoWorlds);
    }
    worlds.sort();
    Ok(worlds)
}

pub(crate) fn ensure_real_directory(path: &Path) -> Result<(), BackupError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| io_error("Could not inspect a directory", path, error))?;
    if !metadata.file_type().is_dir() {
        return Err(BackupError::InvalidServerDirectory(path.to_path_buf()));
    }
    Ok(())
}

fn read_level_name(server_dir: &Path) -> Result<String, BackupError> {
    let path = server_dir.join("server.properties");
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(DEFAULT_LEVEL_NAME.to_owned());
        }
        Err(error) => return Err(io_error("Could not inspect server.properties", path, error)),
    };
    if !metadata.file_type().is_file() || metadata.len() > 1024 * 1024 {
        return Err(BackupError::UnsafeFilesystemEntry(path));
    }
    let contents = fs::read_to_string(&path)
        .map_err(|error| io_error("Could not read server.properties", &path, error))?;
    let level_name = contents
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            (key.trim() == "level-name").then(|| value.trim().to_owned())
        })
        .next_back()
        .unwrap_or_else(|| DEFAULT_LEVEL_NAME.to_owned());
    Ok(level_name)
}
