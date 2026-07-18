use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use reqwest::{header, Client, Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use tokio::sync::Semaphore;
use url::Url;

use crate::config::AppriseConfig;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Error)]
pub enum UpstreamError {
    #[error("invalid Apprise URL: {0}")]
    InvalidUrl(String),
    #[error("invalid Apprise authentication token: {0}")]
    InvalidToken(#[source] header::InvalidHeaderValue),
    #[error("failed to build Apprise HTTP client: {0}")]
    ClientBuild(#[source] reqwest::Error),
    #[error("Apprise is busy; too many concurrent requests")]
    Overloaded,
    #[error("Apprise request failed: {0}")]
    Request(#[source] reqwest::Error),
    #[error("failed reading Apprise response: {0}")]
    ResponseRead(#[source] reqwest::Error),
    #[error("Apprise response exceeded the configured {limit}-byte limit")]
    ResponseTooLarge { limit: usize },
    #[error("Apprise returned HTTP {status}: {body}")]
    HttpStatus { status: StatusCode, body: String },
}

impl UpstreamError {
    pub fn status(&self) -> Option<StatusCode> {
        match self {
            Self::HttpStatus { status, .. } => Some(*status),
            _ => None,
        }
    }

    pub fn is_connection_failure(&self) -> bool {
        matches!(self, Self::Request(error) if error.is_connect() || error.is_timeout())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NotifyType {
    #[default]
    Info,
    Success,
    Warning,
    Failure,
}

impl NotifyType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Failure => "failure",
        }
    }

    pub fn from_str_opt(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "info" => Some(Self::Info),
            "success" => Some(Self::Success),
            "warning" | "warn" => Some(Self::Warning),
            "failure" | "fail" | "error" => Some(Self::Failure),
            _ => None,
        }
    }
}

/// Bounded HTTP client for the Apprise REST API.
#[derive(Clone)]
pub struct AppriseClient {
    client: Client,
    base_url: Url,
    permits: Arc<Semaphore>,
    max_response_bytes: usize,
}

impl AppriseClient {
    pub fn new(config: &AppriseConfig) -> Result<Self, UpstreamError> {
        let mut headers = header::HeaderMap::new();
        if !config.token.is_empty() {
            let bearer = header::HeaderValue::from_str(&format!("Bearer {}", config.token))
                .map_err(UpstreamError::InvalidToken)?;
            let api_key = header::HeaderValue::from_str(&config.token)
                .map_err(UpstreamError::InvalidToken)?;
            headers.insert(header::AUTHORIZATION, bearer);
            headers.insert("X-Apprise-API-Key", api_key);
        }

        let client = Client::builder()
            .default_headers(headers)
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(UpstreamError::ClientBuild)?;
        let base_url =
            Url::parse(&config.url).map_err(|_| UpstreamError::InvalidUrl(config.url.clone()))?;

        Ok(Self {
            client,
            base_url,
            permits: Arc::new(Semaphore::new(config.max_concurrent_requests)),
            max_response_bytes: config.max_response_bytes,
        })
    }

    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    pub async fn notify(
        &self,
        tag: &str,
        title: Option<&str>,
        body: &str,
        notify_type: &NotifyType,
    ) -> Result<Value, UpstreamError> {
        let url = self.endpoint(&["notify", tag], false)?;
        self.post_notify(url, None, title, body, notify_type).await
    }

    pub async fn notify_all(
        &self,
        title: Option<&str>,
        body: &str,
        notify_type: &NotifyType,
    ) -> Result<Value, UpstreamError> {
        let url = self.endpoint(&["notify"], false)?;
        self.post_notify(url, None, title, body, notify_type).await
    }

    pub async fn notify_url(
        &self,
        urls: &str,
        title: Option<&str>,
        body: &str,
        notify_type: &NotifyType,
    ) -> Result<Value, UpstreamError> {
        let url = self.endpoint(&["notify"], true)?;
        self.post_notify(url, Some(urls), title, body, notify_type)
            .await
    }

    pub async fn health(&self) -> Result<Value, UpstreamError> {
        let _permit = self.acquire()?;
        let url = self.endpoint(&["health"], false)?;
        tracing::debug!(%url, "upstream health check");
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(UpstreamError::Request)?;
        let status = response.status();
        let body = self.read_bounded(response).await?;
        if !status.is_success() {
            return Err(UpstreamError::HttpStatus { status, body });
        }
        Ok(parse_success_body(&body, false))
    }

    fn endpoint(&self, segments: &[&str], trailing_slash: bool) -> Result<Url, UpstreamError> {
        let mut url = self.base_url.clone();
        {
            let mut path = url
                .path_segments_mut()
                .map_err(|_| UpstreamError::InvalidUrl(self.base_url.to_string()))?;
            path.pop_if_empty();
            for segment in segments {
                path.push(segment);
            }
            if trailing_slash {
                path.push("");
            }
        }
        Ok(url)
    }

    fn acquire(&self) -> Result<tokio::sync::OwnedSemaphorePermit, UpstreamError> {
        self.permits
            .clone()
            .try_acquire_owned()
            .map_err(|_| UpstreamError::Overloaded)
    }

    async fn post_notify(
        &self,
        url: Url,
        urls_field: Option<&str>,
        title: Option<&str>,
        body: &str,
        notify_type: &NotifyType,
    ) -> Result<Value, UpstreamError> {
        let _permit = self.acquire()?;
        let mut payload = json!({ "body": body, "type": notify_type.as_str() });
        if let Some(title) = title {
            payload["title"] = json!(title);
        }
        if let Some(urls) = urls_field {
            payload["urls"] = json!(urls);
        }

        tracing::debug!(%url, "upstream notify");
        let response = self
            .client
            .post(url)
            .json(&payload)
            .send()
            .await
            .map_err(UpstreamError::Request)?;
        let status = response.status();
        let body = self.read_bounded(response).await?;
        if !status.is_success() {
            return Err(UpstreamError::HttpStatus { status, body });
        }
        Ok(parse_success_body(&body, true))
    }

    async fn read_bounded(&self, response: Response) -> Result<String, UpstreamError> {
        if response
            .content_length()
            .is_some_and(|length| length > self.max_response_bytes as u64)
        {
            return Err(UpstreamError::ResponseTooLarge {
                limit: self.max_response_bytes,
            });
        }

        let mut bytes = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(UpstreamError::ResponseRead)?;
            if bytes.len().saturating_add(chunk.len()) > self.max_response_bytes {
                return Err(UpstreamError::ResponseTooLarge {
                    limit: self.max_response_bytes,
                });
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }
}

fn parse_success_body(body: &str, notification: bool) -> Value {
    if let Ok(value) = serde_json::from_str(body) {
        value
    } else if notification {
        json!({ "ok": true, "response": body.trim() })
    } else {
        json!({ "status": body.trim() })
    }
}
