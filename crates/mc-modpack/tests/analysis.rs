use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

use mc_modpack::{analyze, ModpackFileDisposition, ModpackSeverity, ModpackStatus};
use tempfile::TempDir;
use zip::{write::SimpleFileOptions, ZipWriter};

#[test]
fn classifies_server_files_and_blocks_path_traversal() {
    let directory = TempDir::new().expect("tempdir");
    let pack = write_pack(
        directory.path(),
        r#"{
          "formatVersion": 1,
          "game": "minecraft",
          "versionId": "1.0.0",
          "name": "Compatibility Fixture",
          "dependencies": {"minecraft": "1.21.1", "fabric-loader": "0.16.9"},
          "files": [
            {
              "path": "mods/server.jar",
              "hashes": {"sha512": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},
              "env": {"client": "unsupported", "server": "required"},
              "downloads": ["https://cdn.modrinth.com/data/a/versions/b/server.jar"],
              "fileSize": 1048576
            },
            {
              "path": "mods/client.jar",
              "hashes": {"sha1": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"},
              "env": {"client": "required", "server": "unsupported"},
              "downloads": ["https://cdn.modrinth.com/data/c/versions/d/client.jar"],
              "fileSize": 2048
            },
            {
              "path": "mods/unknown.jar",
              "hashes": {"sha512": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"},
              "downloads": ["https://cdn.modrinth.com/data/e/versions/f/unknown.jar"],
              "fileSize": 4096
            },
            {
              "path": "../escape.jar",
              "hashes": {"sha1": "dddddddddddddddddddddddddddddddddddddddd"},
              "env": {"server": "required"}
            }
          ]
        }"#,
    );

    let report = analyze(&pack).expect("analysis succeeds");
    assert_eq!(report.minecraft_version.as_deref(), Some("1.21.1"));
    assert_eq!(
        report
            .loader
            .as_ref()
            .map(|loader| loader.dependency.as_str()),
        Some("fabric-loader"),
    );
    assert_eq!(report.status, ModpackStatus::Blocked);
    assert_eq!(report.install_files, 1);
    assert_eq!(report.review_files, 1);
    assert_eq!(report.excluded_files, 1);
    assert_eq!(report.server_override_files, 1);
    assert_eq!(report.mod_files, 3);
    assert!(report.issues.iter().any(|issue| {
        issue.severity == ModpackSeverity::Blocker && issue.code == "unsafe_path"
    }));
    let server = report
        .files
        .iter()
        .find(|file| file.path == "mods/server.jar")
        .expect("server file");
    assert_eq!(server.disposition, ModpackFileDisposition::Install);
    assert!(server.has_verified_download);
    let client = report
        .files
        .iter()
        .find(|file| file.path == "mods/client.jar")
        .expect("client file");
    assert_eq!(client.disposition, ModpackFileDisposition::Exclude);
}

#[test]
fn rejects_non_mrpack_extensions_before_reading() {
    let error = analyze("/tmp/not-a-pack.zip").expect_err("extension is rejected");
    assert_eq!(error.code, mc_core::ErrorCode::InvalidInput);
}

fn write_pack(directory: &Path, index: &str) -> PathBuf {
    let path = directory.join("fixture.mrpack");
    let file = File::create(&path).expect("pack file");
    let mut writer = ZipWriter::new(file);
    writer
        .start_file("modrinth.index.json", SimpleFileOptions::default())
        .expect("index header");
    writer.write_all(index.as_bytes()).expect("index bytes");
    writer
        .start_file(
            "server-overrides/config/server.toml",
            SimpleFileOptions::default(),
        )
        .expect("override header");
    writer.write_all(b"motd=fixture").expect("override bytes");
    writer.finish().expect("finish zip");
    path
}
