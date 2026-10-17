use std::{
    fs,
    panic::{catch_unwind, AssertUnwindSafe},
    path::Path,
};

use chrono::Utc;
use mc_core::InstanceSummary;
use uuid::Uuid;

use super::{
    plan_restore_blocking, recovery, restore_blocking, restore_blocking_with_hook, transaction,
};
use crate::{
    error::BackupError, store::BackupService, tree::sync_directory, worlds::ensure_real_directory,
};

fn fixture() -> (tempfile::TempDir, InstanceSummary) {
    let root = tempfile::tempdir().expect("create temporary instance");
    let server = root.path().join("server");
    fs::create_dir(&server).expect("create server directory");
    fs::write(server.join("server.properties"), "level-name=primary\n")
        .expect("write server properties");
    write_world_file(
        &server.join("primary"),
        "region/r.0.0.mca",
        b"original primary",
    );
    write_world_file(
        &server.join("primary_nether"),
        "level.dat",
        b"original nether",
    );
    let instance = InstanceSummary {
        id: Uuid::new_v4(),
        name: "restore test".to_owned(),
        provider: "paper".to_owned(),
        game_version: "1.21.1".to_owned(),
        build_id: 1,
        java_path: Path::new("/usr/bin/java").to_path_buf(),
        min_memory_mb: 512,
        max_memory_mb: 1024,
        port: 25565,
        directory: root.path().to_path_buf(),
        created_at: Utc::now(),
    };
    (root, instance)
}

fn write_world_file(world: &Path, relative: &str, contents: &[u8]) {
    let path = world.join(relative);
    fs::create_dir_all(path.parent().expect("world file parent"))
        .expect("create world file parent");
    fs::write(path, contents).expect("write world file");
}

fn backup(instance: &InstanceSummary) -> Uuid {
    let id = Uuid::new_v4();
    BackupService::create_blocking(instance.id, &instance.directory, id)
        .expect("create test backup");
    id
}

fn file_bytes(instance: &InstanceSummary, world: &str, relative: &str) -> Vec<u8> {
    fs::read(instance.directory.join("server").join(world).join(relative)).expect("read world file")
}

#[test]
fn restore_plan_lists_replacements_and_removed_dimension_roots() {
    let (_root, instance) = fixture();
    let backup_id = backup(&instance);
    write_world_file(
        &instance.directory.join("server/primary"),
        "region/r.0.0.mca",
        b"new primary",
    );
    write_world_file(
        &instance.directory.join("server/primary_the_end"),
        "level.dat",
        b"new dimension",
    );

    let plan = plan_restore_blocking(&instance, backup_id).expect("build restore plan");
    assert_eq!(
        plan.current_worlds,
        ["primary", "primary_nether", "primary_the_end"]
    );
    assert_eq!(plan.target_worlds, ["primary", "primary_nether"]);
    assert_eq!(plan.replaced_worlds, ["primary", "primary_nether"]);
    assert!(plan.added_worlds.is_empty());
    assert_eq!(plan.removed_worlds, ["primary_the_end"]);
    assert!(plan.confirmation_token.starts_with("v1:"));
}

#[test]
fn restore_replaces_worlds_and_removes_extra_recognized_dimensions() {
    let (_root, instance) = fixture();
    let backup_id = backup(&instance);
    write_world_file(
        &instance.directory.join("server/primary"),
        "region/r.0.0.mca",
        b"modified primary",
    );
    write_world_file(
        &instance.directory.join("server/primary_the_end"),
        "level.dat",
        b"extra dimension",
    );
    let plan = plan_restore_blocking(&instance, backup_id).expect("plan restore");

    let restored = restore_blocking(&instance, backup_id, &plan.confirmation_token)
        .expect("restore verified backup");
    assert_eq!(restored.id, backup_id);
    assert_eq!(
        file_bytes(&instance, "primary", "region/r.0.0.mca"),
        b"original primary"
    );
    assert_eq!(
        file_bytes(&instance, "primary_nether", "level.dat"),
        b"original nether"
    );
    assert!(!instance.directory.join("server/primary_the_end").exists());
}

#[test]
fn restore_can_recreate_a_missing_primary_world() {
    let (_root, instance) = fixture();
    let backup_id = backup(&instance);
    fs::remove_dir_all(instance.directory.join("server/primary")).expect("remove primary world");
    let plan = plan_restore_blocking(&instance, backup_id).expect("plan restore");

    assert_eq!(plan.current_worlds, ["primary_nether"]);
    assert_eq!(plan.added_worlds, ["primary"]);
    assert_eq!(plan.replaced_worlds, ["primary_nether"]);
    restore_blocking(&instance, backup_id, &plan.confirmation_token)
        .expect("restore missing primary");

    assert_eq!(
        file_bytes(&instance, "primary", "region/r.0.0.mca"),
        b"original primary"
    );
}

#[test]
fn corrupt_backup_is_rejected_before_any_live_world_is_changed() {
    let (_root, instance) = fixture();
    let backup_id = backup(&instance);
    write_world_file(
        &instance
            .directory
            .join(format!("backups/{backup_id}/primary")),
        "region/r.0.0.mca",
        b"tampered backup",
    );
    write_world_file(
        &instance.directory.join("server/primary"),
        "region/r.0.0.mca",
        b"live content remains",
    );

    assert!(matches!(
        plan_restore_blocking(&instance, backup_id),
        Err(BackupError::IntegrityMismatch(_))
    ));
    assert_eq!(
        file_bytes(&instance, "primary", "region/r.0.0.mca"),
        b"live content remains"
    );
}

#[test]
fn stale_confirmation_does_not_modify_live_worlds() {
    let (_root, instance) = fixture();
    let backup_id = backup(&instance);
    let plan = plan_restore_blocking(&instance, backup_id).expect("plan restore");
    write_world_file(
        &instance.directory.join("server/primary"),
        "region/r.0.0.mca",
        b"changed after preview",
    );

    assert!(matches!(
        restore_blocking(&instance, backup_id, &plan.confirmation_token),
        Err(BackupError::RestorePlanChanged)
    ));
    assert_eq!(
        file_bytes(&instance, "primary", "region/r.0.0.mca"),
        b"changed after preview"
    );
}

#[test]
fn restore_refuses_a_backup_for_another_configured_primary_world() {
    let (_root, instance) = fixture();
    let backup_id = backup(&instance);
    fs::write(
        instance.directory.join("server/server.properties"),
        "level-name=renamed\n",
    )
    .expect("change configured world name");

    assert!(matches!(
        plan_restore_blocking(&instance, backup_id),
        Err(BackupError::RestorePrimaryWorldMismatch { .. })
    ));
}

#[test]
fn an_error_after_install_rolls_back_all_original_roots() {
    let (_root, instance) = fixture();
    let backup_id = backup(&instance);
    write_world_file(
        &instance.directory.join("server/primary"),
        "region/r.0.0.mca",
        b"keep this live world",
    );
    let plan = plan_restore_blocking(&instance, backup_id).expect("plan restore");
    let mut injected = false;
    let result = restore_blocking_with_hook(
        &instance,
        backup_id,
        &plan.confirmation_token,
        &mut |step| {
            if matches!(step, transaction::RestoreStep::NewRootInstalled(_)) && !injected {
                injected = true;
                return Err(BackupError::Io {
                    operation: "Injected restore failure",
                    path: instance.directory.join("server"),
                    source: std::io::Error::other("test failpoint"),
                });
            }
            Ok(())
        },
    );

    assert!(injected);
    assert!(matches!(result, Err(BackupError::Io { .. })));
    assert_eq!(
        file_bytes(&instance, "primary", "region/r.0.0.mca"),
        b"keep this live world"
    );
    assert_eq!(
        file_bytes(&instance, "primary_nether", "level.dat"),
        b"original nether"
    );
}

#[test]
fn pending_swap_is_rolled_back_after_a_simulated_process_interruption() {
    let (_root, instance) = fixture();
    let backup_id = backup(&instance);
    write_world_file(
        &instance.directory.join("server/primary"),
        "region/r.0.0.mca",
        b"keep after restart",
    );
    let plan = plan_restore_blocking(&instance, backup_id).expect("plan restore");

    let interrupted = catch_unwind(AssertUnwindSafe(|| {
        let _ = restore_blocking_with_hook(
            &instance,
            backup_id,
            &plan.confirmation_token,
            &mut |step| {
                if matches!(step, transaction::RestoreStep::NewRootInstalled(_)) {
                    panic!("simulate process interruption after a root rename");
                }
                Ok(())
            },
        );
    }));
    assert!(interrupted.is_err());
    assert!(
        recovery::has_pending_restore(&instance).expect("detect pending transaction"),
        "the durable marker must survive the simulated process exit"
    );

    recovery::recover_pending(&instance).expect("recover interrupted restore");
    assert!(!recovery::has_pending_restore(&instance).expect("confirm recovery completion"));
    assert_eq!(
        file_bytes(&instance, "primary", "region/r.0.0.mca"),
        b"keep after restart"
    );
    assert_eq!(
        file_bytes(&instance, "primary_nether", "level.dat"),
        b"original nether"
    );
}

#[test]
fn unmarked_user_directory_with_restore_like_name_is_not_removed() {
    let (_root, instance) = fixture();
    let server_dir = instance.directory.join("server");
    let unrelated = server_dir.join(format!(".mc-restore-{}", Uuid::new_v4()));
    fs::create_dir(&unrelated).expect("create unrelated directory");
    fs::write(unrelated.join("keep.txt"), b"user data").expect("write user data");
    ensure_real_directory(&server_dir).expect("server directory remains valid");

    recovery::recover_pending(&instance).expect("ignore unmarked directory");
    assert_eq!(
        fs::read(unrelated.join("keep.txt")).expect("read user data"),
        b"user data"
    );
    sync_directory(&server_dir).expect("sync fixture server directory");
}
