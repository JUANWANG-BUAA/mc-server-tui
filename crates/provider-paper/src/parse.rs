//! Validation and decoding of Paper metadata into domain values.

use chrono::{DateTime, Utc};
use mc_core::{AppError, BuildInfo, Checksum, ErrorCode, ReleaseChannel};
use serde_json::Value;
use url::Url;

use super::DATA_HOST;

#[derive(Debug, Clone)]
pub(super) struct ParsedBuild {
    pub(super) info: BuildInfo,
    pub(super) artifact: Option<ParsedArtifact>,
}

impl ParsedBuild {
    pub(super) fn from_value(value: &Value) -> Result<Self, AppError> {
        let object = value
            .as_object()
            .ok_or_else(|| schema_error("A Paper build entry was not an object."))?;
        let id = object
            .get("id")
            .and_then(Value::as_u64)
            .ok_or_else(|| schema_error("Paper build is missing a numeric id."))?;
        let channel = object
            .get("channel")
            .and_then(Value::as_str)
            .ok_or_else(|| schema_error("Paper build is missing a channel."))?;
        let released_at = object
            .get("time")
            .and_then(Value::as_str)
            .map(DateTime::parse_from_rfc3339)
            .transpose()
            .map_err(|error| schema_error(&format!("Paper build has an invalid time: {error}")))?
            .map(|time| time.with_timezone(&Utc));
        let artifact = parse_artifact(object.get("downloads"))?;
        let info = BuildInfo {
            id,
            channel: ReleaseChannel(channel.to_owned()),
            released_at,
            artifact_name: artifact.as_ref().map(|item| item.name.clone()),
            artifact_size: artifact.as_ref().map(|item| item.size),
        };
        Ok(Self { info, artifact })
    }
}

#[derive(Debug, Clone)]
pub(super) struct ParsedArtifact {
    pub(super) name: String,
    pub(super) url: Url,
    pub(super) size: u64,
    pub(super) sha256: Checksum,
}

pub(super) fn select_stable_build(
    builds: Vec<ParsedBuild>,
    game_version: &str,
    build_id: Option<u64>,
) -> Result<ParsedBuild, AppError> {
    let selected = match build_id {
        Some(id) => builds.into_iter().find(|build| build.info.id == id),
        None => builds
            .into_iter()
            .find(|build| build.info.channel.is_stable()),
    }
    .ok_or_else(|| {
        AppError::new(
            ErrorCode::NoStableBuild,
            format!("No matching Paper build was found for Minecraft {game_version}."),
        )
        .with_action("Choose a version/build listed by Paper and try again.")
    })?;

    if !selected.info.channel.is_stable() {
        return Err(AppError::new(
            ErrorCode::NoStableBuild,
            "The selected Paper build is not marked stable.",
        )
        .with_action("Select a stable build or use a future explicit experimental-build flow."));
    }
    Ok(selected)
}

fn parse_artifact(downloads: Option<&Value>) -> Result<Option<ParsedArtifact>, AppError> {
    let Some(downloads) = downloads else {
        return Ok(None);
    };
    let Some(artifact) = downloads.get("server:default") else {
        return Ok(None);
    };
    let object = artifact
        .as_object()
        .ok_or_else(|| schema_error("Paper server download was not an object."))?;
    let name = object
        .get("name")
        .and_then(Value::as_str)
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| schema_error("Paper server download is missing its filename."))?
        .to_owned();
    let raw_url = object
        .get("url")
        .and_then(Value::as_str)
        .ok_or_else(|| schema_error("Paper server download is missing its URL."))?;
    let url = Url::parse(raw_url)
        .map_err(|error| schema_error(&format!("Paper server URL is invalid: {error}")))?;
    if url.scheme() != "https"
        || url.host_str() != Some(DATA_HOST)
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(AppError::new(
            ErrorCode::RedirectBlocked,
            "Paper returned a download URL outside its official HTTPS artifact host.",
        )
        .with_evidence("Expected an HTTPS URL on fill-data.papermc.io."));
    }
    let size = object
        .get("size")
        .and_then(Value::as_u64)
        .filter(|size| *size > 0)
        .ok_or_else(|| schema_error("Paper server download is missing a positive file size."))?;
    let checksum = object
        .get("checksums")
        .and_then(|checksums| checksums.get("sha256"))
        .and_then(Value::as_str)
        .ok_or_else(|| schema_error("Paper server download is missing its SHA-256 checksum."))?;

    Ok(Some(ParsedArtifact {
        name,
        url,
        size,
        sha256: Checksum::sha256(checksum)?,
    }))
}

pub(super) fn validate_version(game_version: &str) -> Result<(), AppError> {
    let valid = !game_version.trim().is_empty()
        && game_version.len() <= 64
        && game_version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'));
    if !valid {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Minecraft version may contain only letters, digits, dots, dashes, and underscores.",
        ));
    }
    Ok(())
}

pub(super) fn is_contact_address(contact: &str) -> bool {
    if let Ok(url) = Url::parse(contact) {
        return url.scheme() == "https" && url.host_str().is_some();
    }
    let Some((_, domain)) = contact.split_once('@') else {
        return false;
    };
    domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')
}

pub(super) fn schema_error(message: &str) -> AppError {
    AppError::new(ErrorCode::SourceSchemaChanged, message)
        .with_action("Keep existing instances unchanged and update the Paper adapter after reviewing the upstream response.")
}
