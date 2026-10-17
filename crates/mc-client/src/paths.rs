use std::{io, path::PathBuf};

use directories::ProjectDirs;

pub fn default_socket_path() -> Result<PathBuf, io::Error> {
    ProjectDirs::from("org", "mc-server-tui", "mc-server-tui")
        .map(|directories| {
            directories
                .state_dir()
                .unwrap_or(directories.data_local_dir())
                .join("run")
                .join("daemon.sock")
        })
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "could not determine application data directory",
            )
        })
}
