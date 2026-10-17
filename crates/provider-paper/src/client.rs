//! HTTP client and metadata transport for Paper Fill v3.

use std::time::Duration;

use mc_core::{AppError, ErrorCode};
use reqwest::{header, redirect::Policy, Client};
use serde_json::Value;
use url::Url;

use super::{
    errors::{network_error, status_error},
    parse::{is_contact_address, schema_error, validate_version, ParsedBuild},
    API_HOST, API_ROOT, MAX_METADATA_BYTES,
};

#[derive(Debug, Clone)]
pub struct PaperProvider {
    client: Client,
    user_agent: Option<String>,
    api_root: Url,
}

impl PaperProvider {
    pub fn new(user_agent: Option<String>) -> Result<Self, AppError> {
        let api_root = Url::parse(API_ROOT).map_err(|error| {
            AppError::new(
                ErrorCode::Internal,
                "The built-in Paper API URL is invalid.",
            )
            .with_evidence(error.to_string())
        })?;
        Self::with_api_root(user_agent, api_root)
    }

    fn with_api_root(user_agent: Option<String>, api_root: Url) -> Result<Self, AppError> {
        if api_root.scheme() != "https" || api_root.host_str() != Some(API_HOST) {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Paper API base URL must use the official HTTPS host.",
            ));
        }
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .redirect(Policy::custom(|attempt| {
                if attempt.previous().len() >= 5 {
                    return attempt.stop();
                }
                let url = attempt.url();
                if url.scheme() == "https" && url.host_str() == Some(API_HOST) {
                    attempt.follow()
                } else {
                    attempt.stop()
                }
            }))
            .build()
            .map_err(|error| {
                AppError::new(
                    ErrorCode::Internal,
                    "Could not initialize the Paper HTTP client.",
                )
                .with_evidence(error.to_string())
            })?;

        Ok(Self {
            client,
            user_agent,
            api_root,
        })
    }

    pub(super) fn validated_user_agent(&self) -> Result<&str, AppError> {
        let value = self
            .user_agent
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let Some(value) = value else {
            return Err(AppError::new(
                ErrorCode::UserAgentRequired,
                "Paper requires a specific application User-Agent with a contact address.",
            )
            .with_action("Set MC_SERVER_TUI_USER_AGENT to `mc-server-tui/<version> (https://your-project-url)` or a real contact email."));
        };

        let contact = value
            .split_once('(')
            .map(|(_, contact)| contact.trim_end_matches(')').trim());
        let has_contact = contact.is_some_and(is_contact_address);
        let valid_header = header::HeaderValue::from_str(value).is_ok();
        if !value.starts_with("mc-server-tui/") || !has_contact || !valid_header {
            return Err(AppError::new(
                ErrorCode::UserAgentRequired,
                "Paper User-Agent must identify mc-server-tui and include a real contact URL or email.",
            )
            .with_action("Set MC_SERVER_TUI_USER_AGENT to `mc-server-tui/<version> (https://your-project-url)` or a real contact email."));
        }
        Ok(value)
    }

    pub(super) async fn get_json(&self, url: Url) -> Result<Value, AppError> {
        let user_agent = self.validated_user_agent()?;
        let response = self
            .client
            .get(url.clone())
            .header(header::USER_AGENT, user_agent)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(|error| network_error(error, &url))?;

        if response.url().scheme() != "https" || response.url().host_str() != Some(API_HOST) {
            return Err(AppError::new(
                ErrorCode::RedirectBlocked,
                "Paper metadata request redirected to an untrusted host.",
            )
            .with_evidence("Only HTTPS requests to fill.papermc.io are allowed."));
        }
        let status = response.status();
        if status.is_redirection() {
            return Err(AppError::new(
                ErrorCode::RedirectBlocked,
                "Paper metadata redirected to a destination that the adapter will not follow.",
            )
            .with_evidence(format!(
                "HTTP {status}; metadata redirects must remain on {API_HOST}."
            )));
        }
        if !status.is_success() {
            return Err(status_error(
                status,
                response
                    .headers()
                    .get(header::RETRY_AFTER)
                    .and_then(|v| v.to_str().ok()),
            ));
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_METADATA_BYTES as u64)
        {
            return Err(AppError::new(
                ErrorCode::RequestTooLarge,
                "Paper metadata response exceeded the 2 MiB safety limit.",
            ));
        }

        let mut response = response;
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|error| network_error(error, &url))?
        {
            if bytes.len().saturating_add(chunk.len()) > MAX_METADATA_BYTES {
                return Err(AppError::new(
                    ErrorCode::RequestTooLarge,
                    "Paper metadata response exceeded the 2 MiB safety limit.",
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes).map_err(|error| {
            AppError::new(
                ErrorCode::SourceSchemaChanged,
                "Paper returned invalid JSON metadata.",
            )
            .with_evidence(error.to_string())
        })
    }

    pub(super) fn endpoint(&self, path: &str) -> Url {
        self.api_root
            .join(path)
            .expect("relative Paper API path is valid")
    }

    pub(super) async fn parse_builds(
        &self,
        game_version: &str,
    ) -> Result<Vec<ParsedBuild>, AppError> {
        validate_version(game_version)?;
        let endpoint = self.endpoint(&format!("projects/paper/versions/{game_version}/builds"));
        let value = self.get_json(endpoint).await?;
        let Some(builds) = value.as_array() else {
            return Err(schema_error(
                "Expected the Paper builds response to be an array.",
            ));
        };

        builds.iter().map(ParsedBuild::from_value).collect()
    }
}
