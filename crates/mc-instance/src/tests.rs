use super::*;
use crate::model::INSTANCE_SCHEMA_VERSION;
use chrono::Utc;
use mc_core::{ArtifactPlan, Checksum, ErrorCode, ReleaseChannel};
use sha2::{Digest, Sha256};
use std::path::Path;
use uuid::Uuid;

fn make_plan(body: &[u8]) -> ArtifactPlan {
    let hash = hex_digest(&Sha256::digest(body));
    ArtifactPlan::new(
        "paper",
        "paper",
        "1.21.1",
        42,
        ReleaseChannel("STABLE".to_owned()),
        "paper-1.21.1-42.jar",
        url::Url::parse("https://fill.papermc.io/metadata").unwrap(),
        url::Url::parse("https://fill-data.papermc.io/paper.jar").unwrap(),
        body.len() as u64,
        Checksum::sha256(hash).unwrap(),
        Utc::now(),
    )
    .unwrap()
}

fn verified_fixture(path: &Path, body: &[u8]) -> mc_download::VerifiedBlob {
    mc_download::VerifiedBlob::test_fixture(
        path.to_path_buf(),
        body.len() as u64,
        hex_digest(&Sha256::digest(body)),
    )
}

fn spec(java_path: &Path) -> InstanceSpec {
    InstanceSpec {
        name: "Test Paper Server".to_owned(),
        java_path: java_path.to_path_buf(),
        min_memory_mb: 1024,
        max_memory_mb: 2048,
        port: 25565,
        accept_eula: true,
    }
}

fn make_executable_java(path: &Path) {
    std::fs::write(path, b"fixture Java executable").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(result, "{byte:02x}");
    }
    result
}

#[tokio::test]
async fn create_commits_verified_instance_files_and_rebuilds_index() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let java_path = temp.path().join("java");
    make_executable_java(&java_path);
    let body = b"known good server artifact";
    let artifact_path = temp.path().join("verified.jar");
    tokio::fs::write(&artifact_path, body).await.unwrap();
    let plan = make_plan(body);
    let blob = verified_fixture(&artifact_path, body);
    let service = InstanceService::open(&state).await.unwrap();

    let created = service
        .create(&plan, &blob, &spec(&java_path))
        .await
        .unwrap();
    assert_eq!(created.name, "Test Paper Server");
    assert_eq!(created.game_version, "1.21.1");
    assert_eq!(service.list().unwrap(), vec![created.clone()]);

    let server_jar = created.directory.join("server/server.jar");
    assert_eq!(tokio::fs::read(&server_jar).await.unwrap(), body);
    assert_eq!(
        tokio::fs::read_to_string(created.directory.join("server/eula.txt"))
            .await
            .unwrap(),
        "eula=true\n"
    );
    let lock: InstanceLock = serde_json::from_slice(
        &tokio::fs::read(created.directory.join("lock.json"))
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(lock.local_sha256, blob.sha256());
    assert_eq!(lock.build_id, 42);
    assert!(
        tokio::fs::try_exists(created.directory.join("instance.toml"))
            .await
            .unwrap()
    );

    drop(service);
    let reopened = InstanceService::open(&state).await.unwrap();
    assert_eq!(reopened.list().unwrap(), vec![created]);
}

#[tokio::test]
async fn failed_copy_never_registers_or_commits_an_instance() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let java_path = temp.path().join("java");
    make_executable_java(&java_path);
    let expected = b"expected server artifact";
    let artifact_path = temp.path().join("tampered.jar");
    tokio::fs::write(&artifact_path, b"tampered server artifact")
        .await
        .unwrap();
    let plan = make_plan(expected);
    let blob = mc_download::VerifiedBlob::test_fixture(
        artifact_path,
        expected.len() as u64,
        hex_digest(&Sha256::digest(expected)),
    );
    let service = InstanceService::open(&state).await.unwrap();

    let error = service
        .create(&plan, &blob, &spec(&java_path))
        .await
        .expect_err("changed bytes must fail verification");
    assert_eq!(error.code, ErrorCode::HashMismatch);
    assert!(service.list().unwrap().is_empty());
    let mut entries = tokio::fs::read_dir(state.join("instances")).await.unwrap();
    while let Some(entry) = entries.next_entry().await.unwrap() {
        assert!(!entry.file_name().to_string_lossy().starts_with(".staging-"));
    }
}

#[tokio::test]
async fn startup_quarantines_incomplete_staging_directories() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let instances = state.join("instances");
    tokio::fs::create_dir_all(instances.join(".staging-interrupted"))
        .await
        .unwrap();

    let service = InstanceService::open(&state).await.unwrap();
    assert!(service.list().unwrap().is_empty());
    assert!(
        !tokio::fs::try_exists(instances.join(".staging-interrupted"))
            .await
            .unwrap()
    );
    let mut quarantined = tokio::fs::read_dir(instances.join("quarantine"))
        .await
        .unwrap();
    assert!(quarantined.next_entry().await.unwrap().is_some());
}

#[test]
fn sqlite_index_migrates_transactionally_and_rejects_future_schema() {
    let temp = tempfile::tempdir().unwrap();
    let database = temp.path().join("index.sqlite3");
    let catalog = SqliteCatalog::open(&database).unwrap();
    assert!(catalog.list().unwrap().is_empty());
    drop(catalog);
    let connection = rusqlite::Connection::open(&database).unwrap();
    let version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, 1);
    drop(connection);

    let newer_database = temp.path().join("newer.sqlite3");
    let connection = rusqlite::Connection::open(&newer_database).unwrap();
    connection.pragma_update(None, "user_version", 99).unwrap();
    drop(connection);
    let error = match SqliteCatalog::open(&newer_database) {
        Ok(_) => panic!("future schema must be rejected"),
        Err(error) => error,
    };
    assert_eq!(error.code, ErrorCode::SourceSchemaChanged);
}

#[test]
fn instance_spec_requires_explicit_eula_acceptance_and_valid_memory() {
    let temp = tempfile::tempdir().unwrap();
    let java_path = temp.path().join("java");
    make_executable_java(&java_path);
    let mut spec = spec(&java_path);
    spec.accept_eula = false;
    assert_eq!(
        spec.validate_for_creation().unwrap_err().code,
        ErrorCode::InvalidInput
    );

    spec.accept_eula = true;
    spec.max_memory_mb = 256;
    assert_eq!(
        spec.validate_for_creation().unwrap_err().code,
        ErrorCode::InvalidInput
    );
}

#[test]
fn installed_lock_must_match_the_instance_version() {
    let temp = tempfile::tempdir().unwrap();
    let java_path = temp.path().join("java");
    make_executable_java(&java_path);
    let body = b"server artifact";
    let plan = make_plan(body);
    let spec = spec(&java_path);
    let (name, java_path) = spec.validate_details().unwrap();
    let config = InstanceConfig {
        schema_version: INSTANCE_SCHEMA_VERSION,
        id: Uuid::new_v4(),
        name,
        provider: plan.provider.clone(),
        project: plan.project.clone(),
        game_version: plan.game_version.clone(),
        build_id: plan.build_id,
        java_path,
        min_memory_mb: spec.min_memory_mb,
        max_memory_mb: spec.max_memory_mb,
        port: spec.port,
        eula_accepted: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let mut lock = InstanceLock::from_plan(&plan, hex_digest(&Sha256::digest(body)));
    lock.game_version = "different-version".to_owned();
    assert_eq!(
        lock.validate(&config).unwrap_err().code,
        ErrorCode::SourceSchemaChanged
    );
}
