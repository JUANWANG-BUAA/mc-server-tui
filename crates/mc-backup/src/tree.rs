use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

use sha2::{Digest, Sha256};

use crate::{
    error::{io_error, BackupError},
    manifest::{BackupEntry, BackupEntryKind, BACKUP_MANIFEST_FILE},
};

const COPY_BUFFER_BYTES: usize = 64 * 1024;

pub(crate) fn measure_worlds(server_dir: &Path, worlds: &[String]) -> Result<u64, BackupError> {
    let mut total = 0_u64;
    for world in worlds {
        total = total
            .checked_add(measure_node(&server_dir.join(world))?)
            .ok_or_else(|| BackupError::InvalidManifest("world size overflow".to_owned()))?;
    }
    Ok(total)
}

pub(crate) fn copy_worlds(
    server_dir: &Path,
    backup_dir: &Path,
    worlds: &[String],
) -> Result<Vec<BackupEntry>, BackupError> {
    let mut entries = Vec::new();
    for world in worlds {
        copy_node(
            &server_dir.join(world),
            &backup_dir.join(world),
            Path::new(world),
            &mut entries,
        )?;
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(entries)
}

pub(crate) fn scan_backup_tree(backup_dir: &Path) -> Result<Vec<BackupEntry>, BackupError> {
    let mut entries = Vec::new();
    for entry in sorted_entries(backup_dir)? {
        let file_name = entry.file_name();
        if file_name == BACKUP_MANIFEST_FILE {
            continue;
        }
        let name = file_name
            .to_str()
            .ok_or_else(|| BackupError::UnsafeFilesystemEntry(entry.path()))?;
        scan_node(&entry.path(), Path::new(name), &mut entries)?;
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(entries)
}

pub(crate) fn scan_worlds(
    server_dir: &Path,
    worlds: &[String],
) -> Result<Vec<BackupEntry>, BackupError> {
    let mut entries = Vec::new();
    for world in worlds {
        scan_node(&server_dir.join(world), Path::new(world), &mut entries)?;
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(entries)
}

fn measure_node(path: &Path) -> Result<u64, BackupError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| io_error("Could not inspect a world entry", path, error))?;
    if metadata.file_type().is_symlink() {
        return Err(BackupError::UnsafeFilesystemEntry(path.to_path_buf()));
    }
    if metadata.is_file() {
        return Ok(metadata.len());
    }
    if !metadata.is_dir() {
        return Err(BackupError::UnsafeFilesystemEntry(path.to_path_buf()));
    }

    let mut total = 0_u64;
    for entry in sorted_entries(path)? {
        total = total
            .checked_add(measure_node(&entry.path())?)
            .ok_or_else(|| BackupError::InvalidManifest("world size overflow".to_owned()))?;
    }
    Ok(total)
}

fn copy_node(
    source: &Path,
    destination: &Path,
    relative: &Path,
    entries: &mut Vec<BackupEntry>,
) -> Result<(), BackupError> {
    let before = fs::symlink_metadata(source)
        .map_err(|error| io_error("Could not inspect a world entry", source, error))?;
    if before.file_type().is_symlink() {
        return Err(BackupError::UnsafeFilesystemEntry(source.to_path_buf()));
    }
    if before.is_dir() {
        create_private_directory(destination)?;
        entries.push(BackupEntry {
            path: path_string(relative)?,
            kind: BackupEntryKind::Directory,
            size_bytes: 0,
            sha256: None,
        });
        for entry in sorted_entries(source)? {
            let name = entry
                .file_name()
                .to_str()
                .ok_or_else(|| BackupError::UnsafeFilesystemEntry(entry.path()))?
                .to_owned();
            copy_node(
                &entry.path(),
                &destination.join(&name),
                &relative.join(name),
                entries,
            )?;
        }
        ensure_unchanged(source, &before)?;
        sync_directory(destination)?;
        return Ok(());
    }
    if !before.is_file() {
        return Err(BackupError::UnsafeFilesystemEntry(source.to_path_buf()));
    }

    let mut input = File::open(source)
        .map_err(|error| io_error("Could not open a world file", source, error))?;
    let mut output = create_private_file(destination)?;
    let mut hasher = Sha256::new();
    let mut size_bytes = 0_u64;
    let mut buffer = vec![0; COPY_BUFFER_BYTES];
    loop {
        let read = input
            .read(&mut buffer)
            .map_err(|error| io_error("Could not read a world file", source, error))?;
        if read == 0 {
            break;
        }
        output
            .write_all(&buffer[..read])
            .map_err(|error| io_error("Could not write a backup file", destination, error))?;
        hasher.update(&buffer[..read]);
        size_bytes = size_bytes
            .checked_add(read as u64)
            .ok_or_else(|| BackupError::InvalidManifest("file size overflow".to_owned()))?;
    }
    output
        .sync_all()
        .map_err(|error| io_error("Could not sync a backup file", destination, error))?;
    ensure_unchanged(source, &before)?;
    if size_bytes != before.len() {
        return Err(BackupError::SourceChanged(source.to_path_buf()));
    }
    entries.push(BackupEntry {
        path: path_string(relative)?,
        kind: BackupEntryKind::File,
        size_bytes,
        sha256: Some(format!("{:x}", hasher.finalize())),
    });
    Ok(())
}

fn scan_node(
    path: &Path,
    relative: &Path,
    entries: &mut Vec<BackupEntry>,
) -> Result<(), BackupError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| io_error("Could not inspect a backup entry", path, error))?;
    if metadata.file_type().is_symlink() {
        return Err(BackupError::UnsafeFilesystemEntry(path.to_path_buf()));
    }
    if metadata.is_dir() {
        entries.push(BackupEntry {
            path: path_string(relative)?,
            kind: BackupEntryKind::Directory,
            size_bytes: 0,
            sha256: None,
        });
        for entry in sorted_entries(path)? {
            let name = entry
                .file_name()
                .to_str()
                .ok_or_else(|| BackupError::UnsafeFilesystemEntry(entry.path()))?
                .to_owned();
            scan_node(&entry.path(), &relative.join(name), entries)?;
        }
        return Ok(());
    }
    if !metadata.is_file() {
        return Err(BackupError::UnsafeFilesystemEntry(path.to_path_buf()));
    }
    let (size_bytes, digest) = hash_file(path)?;
    entries.push(BackupEntry {
        path: path_string(relative)?,
        kind: BackupEntryKind::File,
        size_bytes,
        sha256: Some(digest),
    });
    Ok(())
}

fn hash_file(path: &Path) -> Result<(u64, String), BackupError> {
    let before = fs::symlink_metadata(path)
        .map_err(|error| io_error("Could not inspect a backup file", path, error))?;
    if !before.file_type().is_file() {
        return Err(BackupError::UnsafeFilesystemEntry(path.to_path_buf()));
    }
    let mut file =
        File::open(path).map_err(|error| io_error("Could not read a backup file", path, error))?;
    let mut hasher = Sha256::new();
    let mut size_bytes = 0_u64;
    let mut buffer = vec![0; COPY_BUFFER_BYTES];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| io_error("Could not read a backup file", path, error))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        size_bytes = size_bytes
            .checked_add(read as u64)
            .ok_or_else(|| BackupError::InvalidManifest("file size overflow".to_owned()))?;
    }
    ensure_unchanged(path, &before)?;
    if size_bytes != before.len() {
        return Err(BackupError::SourceChanged(path.to_path_buf()));
    }
    Ok((size_bytes, format!("{:x}", hasher.finalize())))
}

fn sorted_entries(path: &Path) -> Result<Vec<fs::DirEntry>, BackupError> {
    let entries = fs::read_dir(path)
        .map_err(|error| io_error("Could not list a world directory", path, error))?;
    let mut entries = entries
        .map(|entry| {
            entry.map_err(|error| io_error("Could not list a world directory", path, error))
        })
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(fs::DirEntry::file_name);
    Ok(entries)
}

pub(crate) fn create_private_directory(path: &Path) -> Result<(), BackupError> {
    fs::create_dir(path)
        .map_err(|error| io_error("Could not create a backup directory", path, error))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|error| io_error("Could not protect a backup directory", path, error))?;
    }
    Ok(())
}

fn create_private_file(path: &Path) -> Result<File, BackupError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
        .open(path)
        .map_err(|error| io_error("Could not create a backup file", path, error))
}

fn ensure_unchanged(path: &Path, before: &fs::Metadata) -> Result<(), BackupError> {
    let after = fs::symlink_metadata(path)
        .map_err(|error| io_error("Could not recheck a world entry", path, error))?;
    let same_modified = before.modified().ok() == after.modified().ok();
    if before.len() != after.len()
        || !same_modified
        || before.is_file() != after.is_file()
        || before.is_dir() != after.is_dir()
    {
        return Err(BackupError::SourceChanged(path.to_path_buf()));
    }
    Ok(())
}

pub(crate) fn sync_directory(path: &Path) -> Result<(), BackupError> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| io_error("Could not sync a backup directory", path, error))
}

fn path_string(path: &Path) -> Result<String, BackupError> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| BackupError::UnsafeFilesystemEntry(path.to_path_buf()))
}
