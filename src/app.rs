use std::sync::Arc;

use reqwest::StatusCode;
use serde_json::{json, Value};
use thiserror::Error;
use tracing::Instrument;

use crate::apprise::{AppriseClient, NotifyType, UpstreamError};
use crate::config::McpConfig;
use crate::observability::{Counters, ServerClock};
use crate::token_limit::truncate_body;

#[derive(Debug, Error)]
#[error("{message}")]
pub struct ServiceError {
    message: String,
    #[source]
    source: UpstreamError,
}

impl ServiceError {
    pub fn upstream(&self) -> &UpstreamError {
        &self.source
    }
}

#[derive(Clone)]
pub struct AppriseService {
    client: AppriseClient,
    apprise_url: String,
    config: Option<McpConfig>,
    pub counters: Arc<Counters>,
    pub clock: Arc<ServerClock>,
}

impl AppriseService {
    pub fn new(client: AppriseClient, apprise_url: String) -> Self {
        Self {
            client,
            apprise_url,
            config: None,
            counters: Arc::new(Counters::default()),
            clock: Arc::new(ServerClock::new()),
        }
    }

    pub fn with_config(mut self, config: McpConfig) -> Self {
        self.config = Some(config);
        self
    }

    pub fn with_counters(mut self, counters: Arc<Counters>) -> Self {
        self.counters = counters;
        self
    }

    pub fn with_clock(mut self, clock: Arc<ServerClock>) -> Self {
        self.clock = clock;
        self
    }

    pub async fn notify(
        &self,
        tag: &str,
        title: Option<&str>,
        body: &str,
        notify_type: &NotifyType,
    ) -> Result<Value, ServiceError> {
        let (body, warning) = truncate_body(body);
        self.counters.inc_upstream_calls();
        let result = self
            .client
            .notify(tag, title, &body, notify_type)
            .instrument(tracing::info_span!("upstream.notify", %tag))
            .await
            .map_err(|source| self.enrich_error(source, Some(tag)));
        self.finish(result, warning, "notify").await
    }

    pub async fn notify_all(
        &self,
        title: Option<&str>,
        body: &str,
        notify_type: &NotifyType,
    ) -> Result<Value, ServiceError> {
        let (body, warning) = truncate_body(body);
        self.counters.inc_upstream_calls();
        let result = self
            .client
            .notify_all(title, &body, notify_type)
            .instrument(tracing::info_span!("upstream.notify_all"))
            .await
            .map_err(|source| self.enrich_error(source, None));
        self.finish(result, warning, "notify_all").await
    }

    pub async fn notify_url(
        &self,
        urls: &str,
        title: Option<&str>,
        body: &str,
        notify_type: &NotifyType,
    ) -> Result<Value, ServiceError> {
        let (body, warning) = truncate_body(body);
        self.counters.inc_upstream_calls();
        let result = self
            .client
            .notify_url(urls, title, &body, notify_type)
            .instrument(tracing::info_span!("upstream.notify_url"))
            .await
            .map_err(|source| self.enrich_error(source, None));
        self.finish(result, warning, "notify_url").await
    }

    pub async fn health(&self) -> Result<Value, ServiceError> {
        self.counters.inc_upstream_calls();
        let result = self
            .client
            .health()
            .instrument(tracing::info_span!("upstream.health"))
            .await
            .map_err(|source| self.enrich_error(source, None));
        match result {
            Ok(value) => {
                tracing::debug!("health ok");
                Ok(value)
            }
            Err(error) => {
                self.counters.inc_upstream_errors();
                tracing::warn!(%error, "health check failed");
                Err(error)
            }
        }
    }

    pub fn status(&self) -> Value {
        let snap = self.counters.snapshot();
        let mut out = json!({
            "status": "ok",
            "server": {
                "version": env!("CARGO_PKG_VERSION"),
                "uptime_secs": self.clock.uptime_secs(),
                "pid": std::process::id(),
                "data_dir": crate::config::default_data_dir(),
            },
            "counters": {
                "requests_total": snap.requests_total,
                "errors_total": snap.errors_total,
                "upstream_calls": snap.upstream_calls,
                "upstream_errors": snap.upstream_errors,
            },
            "upstream": { "url": redacted_url(&self.apprise_url) },
        });
        if let Some(config) = &self.config {
            out["config"] = json!({
                "host": config.host,
                "port": config.port,
                "server_name": config.server_name,
            });
        }
        out
    }

    async fn finish(
        &self,
        result: Result<Value, ServiceError>,
        warning: Option<String>,
        operation: &'static str,
    ) -> Result<Value, ServiceError> {
        match result {
            Ok(value) => {
                tracing::debug!(operation, "upstream operation succeeded");
                Ok(attach_warning(value, warning))
            }
            Err(error) => {
                self.counters.inc_upstream_errors();
                tracing::warn!(operation, %error, "upstream operation failed");
                Err(error)
            }
        }
    }

    fn enrich_error(&self, source: UpstreamError, tag: Option<&str>) -> ServiceError {
        let message = if source.is_connection_failure() {
            format!(
                "Apprise server at {} is unreachable; use action=health to check connectivity: {source}",
                self.apprise_url
            )
        } else if source.status() == Some(StatusCode::NOT_FOUND) {
            match tag {
                Some(tag) => format!(
                    "Apprise tag {tag:?} has no configured services at {}; use action=health to verify the server: {source}",
                    self.apprise_url
                ),
                None => source.to_string(),
            }
        } else if matches!(
            source.status(),
            Some(StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN)
        ) {
            format!(
                "APPRISE_TOKEN was rejected by {}: {source}",
                self.apprise_url
            )
        } else {
            source.to_string()
        };
        ServiceError { message, source }
    }
}

fn attach_warning(mut value: Value, warning: Option<String>) -> Value {
    let Some(warning) = warning else {
        return value;
    };
    if let Some(object) = value.as_object_mut() {
        object.insert("body_truncation_warning".into(), json!(warning));
        value
    } else {
        json!({ "result": value, "body_truncation_warning": warning })
    }
}

fn redacted_url(value: &str) -> String {
    let Ok(mut url) = url::Url::parse(value) else {
        return "<invalid-url>".into();
    };
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.set_query(None);
    url.set_fragment(None);
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::redacted_url;

    #[test]
    fn status_url_removes_credentials_query_and_fragment() {
        assert_eq!(
            redacted_url("https://user:secret@example.test:8443/api?token=hidden#fragment"),
            "https://example.test:8443/api"
        );
    }
}
