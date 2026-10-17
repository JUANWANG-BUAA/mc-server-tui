use std::{fs, path::Path};

use chrono::{DateTime, TimeZone, Utc};
use mc_core::InstanceSummary;
use uuid::Uuid;

use crate::{store::BackupService, BackupConsistency, BackupError};

fn fixture() -> (tempfile::TempDir, InstanceSummary) {
    let root = tempfile::tempdir().expect("create temp instance");
    let server = root.path().join("server");
    fs::create_dir(&server).expect("create server directory");
    fs::write(server.join("server.properties"), "level-name=primary\n")
        .expect("write server properties");
    write_world_file(&server.join("primary"), "region/r.0.0.mca", b"region bytes");
    write_world_file(&server.join("primary_nether"), "level.dat", b"nether bytes");
    let directory = root.path().to_path_buf();
    (
        root,
        InstanceSummary {
            id: Uuid::new_v4(),
            name: "backup test".to_owned(),
            provider: "paper".to_owned(),
            game_version: "1.21.1".to_owned(),
            build_id: 1,
            java_path: Path::new("/usr/bin/java").to_path_buf(),
            min_memory_mb: 512,
            max_memory_mb: 1024,
            port: 25565,
            directory,
            created_at: Utc::now(),
        },
    )
}

fn write_world_file(world: &Path, relative: &str, contents: &[u8]) {
    let path = world.join(relative);
    fs::create_dir_all(path.parent().expect("world file parent"))
        .expect("create world file parent");
    fs::write(path, contents).expect("write world file");
}

fn set_manifest_created_at(instance: &InstanceSummary, backup_id: Uuid, created_at: DateTime<Utc>) {
    let manifest_path = instance
        .directory
        .join(format!("backups/{backup_id}/manifest.json"));
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).expect("read manifest"))
            .expect("decode manifest");
    manifest["created_at"] = serde_json::to_value(created_at).expect("encode timestamp");
    fs::write(
        manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("encode updated manifest"),
    )
    .expect("write updated manifest");
}

#[tokio::test]
async fn creates_lists_and_verifies_a_stopped_world_snapshot() {
    let (root, instance) = fixture();
    let backup_id = Uuid::new_v4();
    let service = BackupService;
    let manifest = BackupService::create_blocking(instance.id, &instance.directory, backup_id)
        .expect("create backup");

    assert_eq!(manifest.consistency, BackupConsistency::ServerStopped);
    assert_eq!(
        manifest.worlds,
        vec!["primary".to_owned(), "primary_nether".to_owned()]
    );
    assert!(manifest
        .entries
        .iter()
        .any(|entry| entry.path == "primary/region/r.0.0.mca"));
    assert_eq!(
        service
            .verify(&instance, backup_id)
            .await
            .expect("verify backup"),
        manifest
    );

    let staging = root.path().join("backups/.staging-abandoned");
    fs::create_dir(&staging).expect("create abandoned stage");
    fs::write(staging.join("partial"), b"incomplete").expect("write abandoned stage");
    let listed = service.list(&instance).await.expect("list valid backups");
    assert_eq!(listed, [manifest]);
}

#[tokio::test]
async fn verification_detects_changed_world_bytes() {
    let (_root, instance) = fixture();
    let backup_id = Uuid::new_v4();
    BackupService::create_blocking(instance.id, &instance.directory, backup_id)
        .expect("create backup");
    fs::write(
        instance
            .directory
            .join(format!("backups/{backup_id}/primary/level.dat")),
        b"tampered bytes",
    )
    .expect("tamper with copied world");

    assert!(matches!(
        BackupService.verify(&instance, backup_id).await,
        Err(BackupError::IntegrityMismatch(_))
    ));
}

#[tokio::test]
async fn paginated_listing_uses_a_stable_cursor_without_duplicates() {
    let (_root, instance) = fixture();
    let service = BackupService;
    let timestamp = Utc.with_ymd_and_hms(2026, 1, 2, 3, 4, 5).unwrap();
    for value in 1..=3 {
        let id = Uuid::from_u128(value);
        BackupService::create_blocking(instance.id, &instance.directory, id)
            .expect("create backup");
        set_manifest_created_at(&instance, id, timestamp);
    }

    let first = service
        .list_page(&instance, 1, None)
        .await
        .expect("list first page");
    assert_eq!(
        first
            .backups
            .iter()
            .map(|backup| backup.id)
            .collect::<Vec<_>>(),
        [Uuid::from_u128(3)]
    );
    let cursor = first.next_cursor.expect("more backups remain");

    let second = service
        .list_page(&instance, 1, Some(cursor))
        .await
        .expect("list second page");
    assert_eq!(
        second
            .backups
            .iter()
            .map(|backup| backup.id)
            .collect::<Vec<_>>(),
        [Uuid::from_u128(2)]
    );
    let cursor = second.next_cursor.expect("last backup remains");

    let third = service
        .list_page(&instance, 1, Some(cursor))
        .await
        .expect("list third page");
    assert_eq!(
        third
            .backups
            .iter()
            .map(|backup| backup.id)
            .collect::<Vec<_>>(),
        [Uuid::from_u128(1)]
    );
    assert!(third.next_cursor.is_none());
}

#[tokio::test]
async fn page_summaries_are_rehashed_and_verification_returns_bounded_metadata() {
    let (_root, instance) = fixture();
    let backup_id = Uuid::new_v4();
    let manifest = BackupService::create_blocking(instance.id, &instance.directory, backup_id)
        .expect("create backup");
    let files = manifest
        .entries
        .iter()
        .filter(|entry| entry.kind == crate::BackupEntryKind::File)
        .collect::<Vec<_>>();

    let listed = BackupService
        .list_page(&instance, 10, None)
        .await
        .expect("list and verify returned page");
    assert_eq!(listed.backups.len(), 1);
    assert_eq!(listed.backups[0].id, backup_id);
    assert_eq!(listed.backups[0].file_count, files.len() as u64);
    assert_eq!(
        listed.backups[0].total_bytes,
        files.iter().map(|entry| entry.size_bytes).sum::<u64>()
    );
    assert!(listed.next_cursor.is_none());

    let verified = BackupService
        .verify_summary(&instance, backup_id)
        .await
        .expect("verify and summarize backup");
    assert_eq!(verified, listed.backups[0]);
}

#[tokio::test]
async fn paginated_listing_rejects_tampered_returned_backup_and_invalid_limit() {
    let (_root, instance) = fixture();
    let backup_id = Uuid::new_v4();
    BackupService::create_blocking(instance.id, &instance.directory, backup_id)
        .expect("create backup");
    fs::write(
        instance
            .directory
            .join(format!("backups/{backup_id}/primary/level.dat")),
        b"tampered bytes",
    )
    .expect("tamper with backup content");

    assert!(matches!(
        BackupService.list_page(&instance, 10, None).await,
        Err(BackupError::IntegrityMismatch(_))
    ));
    assert!(matches!(
        BackupService.list_page(&instance, 0, None).await,
        Err(BackupError::InvalidPageLimit { limit: 0, .. })
    ));
}

#[tokio::test]
async fn paginated_listing_caps_catalog_entries_before_reading_manifests() {
    let (_root, instance) = fixture();
    let backup_root = instance.directory.join("backups");
    fs::create_dir(&backup_root).expect("create backup root");
    for value in 1..=1_001 {
        fs::create_dir(backup_root.join(Uuid::from_u128(value).to_string()))
            .expect("create catalog entry");
    }

    assert!(matches!(
        BackupService.list_page(&instance, 1, None).await,
        Err(BackupError::CatalogTooLarge { maximum: 1_000 })
    ));
}

#[tokio::test]
async fn verification_rejects_manifest_paths_that_escape_world_roots() {
    let (_root, instance) = fixture();
    let backup_id = Uuid::new_v4();
    BackupService::create_blocking(instance.id, &instance.directory, backup_id)
        .expect("create backup");
    let manifest_path = instance
        .directory
        .join(format!("backups/{backup_id}/manifest.json"));
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).expect("read backup manifest"))
            .expect("decode backup manifest");
    manifest["entries"][0]["path"] = serde_json::Value::String("primary/../outside".to_owned());
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).expect("encode changed manifest"),
    )
    .expect("write changed manifest");

    assert!(matches!(
        BackupService.verify(&instance, backup_id).await,
        Err(BackupError::InvalidManifest(_))
    ));
}

#[test]
fn creation_rejects_unsafe_world_names_before_staging() {
    let (root, instance) = fixture();
    fs::write(
        root.path().join("server/server.properties"),
        "level-name=../escape\n",
    )
    .expect("write unsafe world name");

    assert!(matches!(
        BackupService::create_blocking(instance.id, &instance.directory, Uuid::new_v4()),
        Err(BackupError::InvalidWorldName(_))
    ));
    assert!(!root.path().join("backups").exists());
}

#[cfg(unix)]
#[test]
fn creation_rejects_symlinks_inside_worlds() {
    use std::os::unix::fs::symlink;

    let (root, instance) = fixture();
    let outside = root.path().join("outside-secret");
    fs::write(&outside, b"must not be copied").expect("write outside file");
    symlink(&outside, root.path().join("server/primary/escape")).expect("add world symlink");

    assert!(matches!(
        BackupService::create_blocking(instance.id, &instance.directory, Uuid::new_v4()),
        Err(BackupError::UnsafeFilesystemEntry(_))
    ));
    assert!(!root.path().join("backups").exists());
}

#[test]
fn duplicate_backup_id_does_not_replace_an_existing_snapshot() {
    let (_root, instance) = fixture();
    let backup_id = Uuid::new_v4();
    let first = BackupService::create_blocking(instance.id, &instance.directory, backup_id)
        .expect("create first backup");
    let second = BackupService::create_blocking(instance.id, &instance.directory, backup_id);

    assert!(matches!(second, Err(BackupError::AlreadyExists(id)) if id == backup_id));
    assert_eq!(
        BackupService::verify_blocking(&instance, backup_id)
            .expect("existing backup remains valid"),
        first
    );
}
