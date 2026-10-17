use super::*;
use crate::parse::{validate_version, ParsedBuild};

const BUILDS: &str = include_str!("../tests/fixtures/builds.json");
const PROJECT: &str = include_str!("../tests/fixtures/project.json");

#[test]
fn parses_grouped_versions_from_fill_v3() {
    let value: Value = serde_json::from_str(PROJECT).expect("fixture JSON");
    let groups = value
        .get("versions")
        .and_then(Value::as_object)
        .expect("version map");
    let versions: Vec<_> = groups
        .iter()
        .flat_map(|(group, values)| {
            values
                .as_array()
                .expect("group array")
                .iter()
                .map(move |value| (group, value))
        })
        .collect();

    assert_eq!(versions[0].1.as_str(), Some("26.1"));
    assert_eq!(versions[1].1.as_str(), Some("1.21.11"));
}

#[test]
fn parses_stable_build_artifact_and_checksum() {
    let value: Value = serde_json::from_str(BUILDS).expect("fixture JSON");
    let builds = value.as_array().expect("build array");
    let parsed: Vec<_> = builds
        .iter()
        .map(ParsedBuild::from_value)
        .collect::<Result<_, _>>()
        .expect("valid builds");
    let stable = parsed
        .iter()
        .find(|build| build.info.channel.is_stable())
        .expect("stable build");
    let artifact = stable.artifact.as_ref().expect("server artifact");

    assert_eq!(stable.info.id, 17);
    assert_eq!(artifact.name, "paper-26.1-17.jar");
    assert_eq!(artifact.size, 54_185_955);
    assert_eq!(
        artifact.sha256.value,
        "bfca155b4a6b45644bfc1766f4e02a83c736e45fcc060e8788c71d6e7b3d56f6"
    );
}

#[test]
fn selects_the_first_stable_build_and_rejects_experimental_or_missing_builds() {
    let value: Value = serde_json::from_str(BUILDS).expect("fixture JSON");
    let builds = value
        .as_array()
        .expect("build array")
        .iter()
        .map(ParsedBuild::from_value)
        .collect::<Result<Vec<_>, _>>()
        .expect("valid builds");

    assert_eq!(
        select_stable_build(builds.clone(), "26.1", None)
            .expect("stable build")
            .info
            .id,
        17
    );
    assert_eq!(
        select_stable_build(builds.clone(), "26.1", Some(18))
            .expect_err("experimental build rejected")
            .code,
        ErrorCode::NoStableBuild
    );
    assert_eq!(
        select_stable_build(builds, "26.1", Some(999))
            .expect_err("unknown build rejected")
            .code,
        ErrorCode::NoStableBuild
    );
}

#[test]
fn rejects_builds_without_a_stable_entry() {
    let value: Value = serde_json::from_str(BUILDS).expect("fixture JSON");
    let mut builds = value.as_array().expect("build array").to_vec();
    builds[0]["channel"] = Value::String("EXPERIMENTAL".to_owned());
    let parsed = builds
        .iter()
        .map(ParsedBuild::from_value)
        .collect::<Result<Vec<_>, _>>()
        .expect("valid builds");

    assert_eq!(
        select_stable_build(parsed, "26.1", None)
            .expect_err("no stable build")
            .code,
        ErrorCode::NoStableBuild
    );
}

#[test]
fn rejects_non_official_download_hosts_and_bad_hashes() {
    let mut value: Value = serde_json::from_str(BUILDS).expect("fixture JSON");
    value[0]["downloads"]["server:default"]["url"] =
        Value::String("https://example.invalid/paper.jar".to_owned());
    assert_eq!(
        ParsedBuild::from_value(&value[0])
            .expect_err("untrusted host")
            .code,
        ErrorCode::RedirectBlocked
    );

    value[0]["downloads"]["server:default"]["url"] =
        Value::String("https://fill-data.papermc.io/paper.jar".to_owned());
    value[0]["downloads"]["server:default"]["checksums"]["sha256"] =
        Value::String("bad".to_owned());
    assert_eq!(
        ParsedBuild::from_value(&value[0])
            .expect_err("bad hash")
            .code,
        ErrorCode::SourceSchemaChanged
    );
}

#[test]
fn rejects_malformed_game_version_before_request() {
    assert!(validate_version("../other").is_err());
    assert!(validate_version("26.1").is_ok());
}

#[test]
fn requires_project_identity_and_contact_in_user_agent() {
    let provider =
        PaperProvider::new(Some("generic client".to_owned())).expect("valid client setup");
    assert_eq!(
        provider
            .validated_user_agent()
            .expect_err("identity required")
            .code,
        ErrorCode::UserAgentRequired
    );

    let provider = PaperProvider::new(Some(
        "mc-server-tui/0.1.0 (https://example.org/project)".to_owned(),
    ))
    .expect("valid client setup");
    assert!(provider.validated_user_agent().is_ok());
}
