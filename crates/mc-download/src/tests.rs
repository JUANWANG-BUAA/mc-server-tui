use super::*;
use crate::digest::hex_digest;
use crate::resume::{
    is_strong_etag, read_resume_state, url_fingerprint, valid_content_range, ResumeMetadata,
};
use crate::security::{is_public_ip, validate_url};
use chrono::Utc;
use mc_core::{Checksum, ReleaseChannel};
use sha2::{Digest, Sha256};
use std::sync::{atomic::AtomicBool, Arc};
use url::Url;

fn plan(url: Url, bytes: &[u8]) -> ArtifactPlan {
    ArtifactPlan::new(
        "paper",
        "paper",
        "1.21.1",
        42,
        ReleaseChannel("STABLE".to_owned()),
        "paper.jar",
        Url::parse("https://fill.papermc.io/metadata").expect("metadata URL"),
        url,
        bytes.len() as u64,
        Checksum::sha256(hex_digest(&Sha256::digest(bytes))).expect("checksum"),
        Utc::now(),
    )
    .expect("plan")
}

#[test]
fn range_parser_requires_exact_start_and_total() {
    assert!(valid_content_range("bytes 10-19/20", 10, 20));
    assert!(!valid_content_range("bytes 9-19/20", 10, 20));
    assert!(!valid_content_range("bytes 10-20/20", 10, 20));
    assert!(!valid_content_range("bytes 10-14/20", 10, 20));
    assert!(!valid_content_range("bytes */20", 10, 20));
}

#[test]
fn weak_or_malformed_etags_are_not_resumable() {
    assert!(is_strong_etag("\"abc\""));
    assert!(!is_strong_etag("W/\"abc\""));
    assert!(!is_strong_etag("abc"));
    assert!(!is_strong_etag("\"a\"b\""));
}

#[test]
fn private_and_special_ip_ranges_are_rejected() {
    for ip in [
        "127.0.0.1",
        "10.2.3.4",
        "100.64.1.2",
        "192.0.2.1",
        "198.19.0.1",
        "169.254.0.1",
        "::1",
        "fc00::1",
        "fe80::1",
        "2001:db8::1",
        "::ffff:127.0.0.1",
    ] {
        assert!(!is_public_ip(ip.parse().expect("IP address")), "{ip}");
    }
    for ip in ["8.8.8.8", "1.1.1.1", "2606:4700:4700::1111"] {
        assert!(is_public_ip(ip.parse().expect("IP address")), "{ip}");
    }
}

#[test]
fn url_validation_requires_exact_trusted_https_host() {
    let hosts = ["fill-data.papermc.io".to_owned()].into_iter().collect();
    assert!(validate_url(
        &Url::parse("https://fill-data.papermc.io/file.jar").unwrap(),
        &hosts
    )
    .is_ok());
    assert!(validate_url(
        &Url::parse("https://evil.fill-data.papermc.io/file.jar").unwrap(),
        &hosts
    )
    .is_err());
    assert!(validate_url(
        &Url::parse("http://fill-data.papermc.io/file.jar").unwrap(),
        &hosts
    )
    .is_err());
}

#[tokio::test]
async fn cache_hits_are_rehashed_before_becoming_verified_blobs() {
    let temp = tempfile::tempdir().expect("tempdir");
    let body = b"verified fixture bytes";
    let plan = plan(
        Url::parse("https://fill-data.papermc.io/paper.jar").unwrap(),
        body,
    );
    let manager = DownloadManager::new(DownloadConfig::paper(
        temp.path(),
        "mc-server-tui/0.1 contact: https://example.invalid",
    ))
    .expect("manager");
    let cache_path = tokio::fs::canonicalize(temp.path())
        .await
        .unwrap()
        .join("mc-server-tui-artifacts")
        .join("sha256")
        .join(&plan.checksum.value[..2])
        .join(&plan.checksum.value);
    tokio::fs::create_dir_all(cache_path.parent().unwrap())
        .await
        .unwrap();
    tokio::fs::write(&cache_path, body).await.unwrap();

    let blob = manager.fetch(&plan).await.expect("cache hit");
    assert_eq!(blob.path(), cache_path);
    assert_eq!(blob.size(), body.len() as u64);
    assert_eq!(blob.sha256(), plan.checksum.value);

    tokio::fs::write(&cache_path, b"tampered fixture bytes")
        .await
        .unwrap();
    // Invalid cache bytes are removed after size and hash revalidation.
    let result = verify_cached_blob(&cache_path, &plan)
        .await
        .expect("cache validation succeeds");
    assert!(result.is_none());
    assert!(!cache_path.exists());
}

#[tokio::test]
async fn resume_requires_matching_identity_and_a_strong_etag() {
    let temp = tempfile::tempdir().expect("tempdir");
    let body = b"expected complete response";
    let plan = plan(
        Url::parse("https://fill-data.papermc.io/paper.jar?token=secret").unwrap(),
        body,
    );
    let part_path = temp.path().join("artifact.part");
    let metadata_path = temp.path().join("artifact.part.json");
    tokio::fs::write(&part_path, b"already received")
        .await
        .unwrap();
    let metadata = ResumeMetadata {
        url_fingerprint: url_fingerprint(&plan.download_url),
        expected_size: plan.expected_size,
        expected_sha256: plan.checksum.value.clone(),
        strong_etag: Some("\"revision-1\"".to_owned()),
    };
    tokio::fs::write(&metadata_path, serde_json::to_vec(&metadata).unwrap())
        .await
        .unwrap();

    let resume = read_resume_state(&part_path, &metadata_path, &plan)
        .await
        .expect("resume metadata is readable")
        .expect("identity and strong ETag match");
    assert_eq!(resume.offset, b"already received".len() as u64);
    assert_eq!(resume.strong_etag.as_deref(), Some("\"revision-1\""));

    let weak_metadata = ResumeMetadata {
        strong_etag: Some("W/\"revision-1\"".to_owned()),
        ..metadata
    };
    tokio::fs::write(&metadata_path, serde_json::to_vec(&weak_metadata).unwrap())
        .await
        .unwrap();
    assert!(read_resume_state(&part_path, &metadata_path, &plan)
        .await
        .expect("weak ETag safely invalidates resume")
        .is_none());
    assert!(!part_path.exists());
    assert!(!metadata_path.exists());
}

#[cfg(unix)]
#[tokio::test]
async fn cache_setup_does_not_change_caller_owned_parent_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::tempdir().expect("tempdir");
    tokio::fs::set_permissions(temp.path(), std::fs::Permissions::from_mode(0o755))
        .await
        .unwrap();
    let managed = crate::cache::prepare_cache_root(temp.path())
        .await
        .expect("managed cache root");

    assert_eq!(
        tokio::fs::metadata(temp.path())
            .await
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o755
    );
    assert_eq!(
        tokio::fs::metadata(managed)
            .await
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
}

#[tokio::test]
async fn cancelled_download_fails_before_network_transfer() {
    let temp = tempfile::tempdir().expect("tempdir");
    let body = b"cancelled fixture";
    let plan = plan(
        Url::parse("https://fill-data.papermc.io/paper.jar").unwrap(),
        body,
    );
    let manager = DownloadManager::new(DownloadConfig::paper(
        temp.path(),
        "mc-server-tui/0.1 contact: https://example.invalid",
    ))
    .expect("manager");
    let control = DownloadControl::new(Arc::new(AtomicBool::new(true)));
    let error = manager
        .fetch_with_control(&plan, &control)
        .await
        .expect_err("cancelled download is rejected");
    assert_eq!(error.code, mc_core::ErrorCode::TaskCancelled);
    assert!(!error.retryable);
}
