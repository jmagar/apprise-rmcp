use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use thiserror::Error;

use crate::app::AppriseService;
use crate::apprise::{AppriseClient, UpstreamError};
use crate::config::{default_data_dir, AuthMode, Config};
use crate::mcp::{AppState, AuthPolicy};

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error(transparent)]
    Upstream(#[from] UpstreamError),
    #[error("failed to resolve MCP bind host {host:?}: {source}")]
    Resolve {
        host: String,
        #[source]
        source: std::io::Error,
    },
    #[error("MCP bind host {host:?} resolved to no addresses")]
    NoResolvedAddress { host: String },
    #[error("APPRISE_MCP_NO_AUTH is only allowed when the selected bind address for {host:?} is loopback")]
    UnsafeNoAuth { host: String },
    #[error("APPRISE_MCP_TOKEN is required for bearer authentication")]
    MissingBearerToken,
    #[error("failed to build OAuth configuration: {0}")]
    OAuthConfig(#[source] lab_auth::error::AuthError),
    #[error("failed to initialize OAuth state: {0}")]
    OAuthState(#[source] lab_auth::error::AuthError),
    #[error("failed to seed OAuth allowlist for {email}: {source}")]
    OAuthAllowlist {
        email: String,
        #[source]
        source: lab_auth::error::AuthError,
    },
}

pub struct PreparedRuntime {
    pub state: AppState,
    pub bind_addrs: Vec<SocketAddr>,
}

pub async fn build_state(config: Config) -> Result<PreparedRuntime, RuntimeError> {
    let bind_addrs = resolve_bind_addrs(&config.mcp.host, config.mcp.port).await?;
    if config.mcp.no_auth && !bind_addrs.iter().all(|address| address.ip().is_loopback()) {
        return Err(RuntimeError::UnsafeNoAuth {
            host: config.mcp.host.clone(),
        });
    }

    let client = AppriseClient::new(&config.apprise)?;
    let service =
        AppriseService::new(client, config.apprise.url.clone()).with_config(config.mcp.clone());
    let auth_policy = if config.mcp.no_auth {
        AuthPolicy::LoopbackDev
    } else if config.mcp.auth.mode == AuthMode::OAuth {
        AuthPolicy::Mounted {
            auth_state: Some(Arc::new(build_oauth_state(&config).await?)),
        }
    } else {
        if config.mcp.api_token.as_deref().is_none_or(str::is_empty) {
            return Err(RuntimeError::MissingBearerToken);
        }
        AuthPolicy::Mounted { auth_state: None }
    };

    let counters = service.counters.clone();
    let clock = service.clock.clone();
    Ok(PreparedRuntime {
        state: AppState {
            config: config.mcp,
            auth_policy,
            service,
            counters,
            clock,
        },
        bind_addrs,
    })
}

pub async fn resolve_bind_addrs(host: &str, port: u16) -> Result<Vec<SocketAddr>, RuntimeError> {
    let addresses = tokio::net::lookup_host((host, port))
        .await
        .map_err(|source| RuntimeError::Resolve {
            host: host.into(),
            source,
        })?
        .collect::<Vec<_>>();
    if addresses.is_empty() {
        return Err(RuntimeError::NoResolvedAddress { host: host.into() });
    }
    Ok(addresses)
}

pub async fn build_oauth_state(
    config: &Config,
) -> Result<lab_auth::state::AuthState, RuntimeError> {
    let auth = &config.mcp.auth;
    let mut vars = vec![
        ("APPRISE_MCP_AUTH_MODE".into(), "oauth".into()),
        (
            "APPRISE_MCP_AUTH_SQLITE_PATH".into(),
            auth.sqlite_path.clone(),
        ),
        ("APPRISE_MCP_AUTH_KEY_PATH".into(), auth.key_path.clone()),
        (
            "APPRISE_MCP_AUTH_ACCESS_TOKEN_TTL_SECS".into(),
            auth.access_token_ttl_secs.to_string(),
        ),
        (
            "APPRISE_MCP_AUTH_REFRESH_TOKEN_TTL_SECS".into(),
            auth.refresh_token_ttl_secs.to_string(),
        ),
        (
            "APPRISE_MCP_AUTH_CODE_TTL_SECS".into(),
            auth.auth_code_ttl_secs.to_string(),
        ),
        (
            "APPRISE_MCP_AUTH_REGISTER_REQUESTS_PER_MINUTE".into(),
            auth.register_rpm.to_string(),
        ),
        (
            "APPRISE_MCP_AUTH_AUTHORIZE_REQUESTS_PER_MINUTE".into(),
            auth.authorize_rpm.to_string(),
        ),
        (
            "APPRISE_MCP_AUTH_MAX_PENDING_OAUTH_STATES".into(),
            auth.max_pending_oauth_states.to_string(),
        ),
    ];
    push_option(&mut vars, "APPRISE_MCP_PUBLIC_URL", &auth.public_url);
    push_option(
        &mut vars,
        "APPRISE_MCP_GOOGLE_CLIENT_ID",
        &auth.google_client_id,
    );
    push_option(
        &mut vars,
        "APPRISE_MCP_GOOGLE_CLIENT_SECRET",
        &auth.google_client_secret,
    );
    push_nonempty(&mut vars, "APPRISE_MCP_AUTH_ADMIN_EMAIL", &auth.admin_email);
    if !auth.allowed_client_redirect_uris.is_empty() {
        vars.push((
            "APPRISE_MCP_AUTH_ALLOWED_REDIRECT_URIS".into(),
            auth.allowed_client_redirect_uris.join(","),
        ));
    }

    let auth_config = lab_auth::config::AuthConfigBuilder::new()
        .env_prefix("APPRISE_MCP")
        .default_data_dir(default_data_dir())
        .session_cookie_name("apprise_mcp_session")
        .scopes_supported(vec!["apprise:notify".into(), "apprise:admin".into()])
        .default_scope("apprise:notify")
        .static_token_scopes(vec!["apprise:notify".into(), "apprise:admin".into()])
        .resource_path("/mcp")
        .enable_dynamic_registration(true)
        .disable_static_token_with_oauth(auth.disable_static_token_with_oauth)
        .build_from_sources(vars)
        .map_err(RuntimeError::OAuthConfig)?;

    let state = lab_auth::state::AuthState::new(auth_config)
        .await
        .map_err(RuntimeError::OAuthState)?;
    let configured = auth
        .allowed_emails
        .iter()
        .map(|email| email.to_lowercase())
        .collect::<HashSet<_>>();
    let existing = state
        .store
        .list_allowed_users()
        .await
        .map_err(RuntimeError::OAuthState)?;
    let existing_emails = existing
        .iter()
        .map(|row| row.email.clone())
        .collect::<HashSet<_>>();
    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    for email in configured.difference(&existing_emails) {
        state
            .store
            .add_allowed_user(email, "config", created_at)
            .await
            .map_err(|source| RuntimeError::OAuthAllowlist {
                email: email.clone(),
                source,
            })?;
    }
    for row in existing {
        if row.added_by == "config" && !configured.contains(&row.email) {
            state
                .store
                .remove_allowed_user(&row.email)
                .await
                .map_err(RuntimeError::OAuthState)?;
        }
    }
    Ok(state)
}

fn push_option(vars: &mut Vec<(String, String)>, key: &str, value: &Option<String>) {
    if let Some(value) = value.as_deref() {
        push_nonempty(vars, key, value);
    }
}

fn push_nonempty(vars: &mut Vec<(String, String)>, key: &str, value: &str) {
    if !value.is_empty() {
        vars.push((key.into(), value.into()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn bind_resolution_returns_the_address_that_will_be_bound() {
        assert!(resolve_bind_addrs("localhost", 40050)
            .await
            .unwrap()
            .iter()
            .all(|address| address.ip().is_loopback()));
        assert_eq!(
            resolve_bind_addrs("0.0.0.0", 40050).await.unwrap(),
            vec!["0.0.0.0:40050".parse().unwrap()]
        );
    }

    #[tokio::test]
    async fn no_auth_is_rejected_for_non_loopback_bind() {
        let mut config = Config::default();
        config.mcp.host = "0.0.0.0".into();
        config.mcp.no_auth = true;
        assert!(matches!(
            build_state(config).await,
            Err(RuntimeError::UnsafeNoAuth { .. })
        ));
    }

    #[tokio::test]
    async fn loopback_bearer_auth_remains_mounted() {
        let mut config = Config::default();
        config.mcp.host = "127.0.0.1".into();
        config.mcp.api_token = Some("secret".into());
        let prepared = build_state(config).await.unwrap();
        assert!(matches!(
            prepared.state.auth_policy,
            AuthPolicy::Mounted { auth_state: None }
        ));
        assert_eq!(
            prepared.bind_addrs,
            vec!["127.0.0.1:40050".parse().unwrap()]
        );
    }

    #[tokio::test]
    async fn loopback_oauth_remains_mounted() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = Config::default();
        config.mcp.host = "127.0.0.1".into();
        config.mcp.auth.mode = AuthMode::OAuth;
        config.mcp.auth.public_url = Some("https://apprise.example.test".into());
        config.mcp.auth.google_client_id = Some("client".into());
        config.mcp.auth.google_client_secret = Some("secret".into());
        config.mcp.auth.admin_email = "admin@example.test".into();
        config.mcp.auth.sqlite_path = directory.path().join("auth.db").display().to_string();
        config.mcp.auth.key_path = directory.path().join("auth.pem").display().to_string();

        let prepared = build_state(config).await.unwrap();
        assert!(matches!(
            prepared.state.auth_policy,
            AuthPolicy::Mounted {
                auth_state: Some(_)
            }
        ));
    }

    #[tokio::test]
    async fn oauth_settings_and_allowlist_reach_lab_auth() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = Config::default();
        let auth = &mut config.mcp.auth;
        auth.mode = AuthMode::OAuth;
        auth.public_url = Some("https://apprise.example.test".into());
        auth.google_client_id = Some("client".into());
        auth.google_client_secret = Some("secret".into());
        auth.admin_email = "admin@example.test".into();
        auth.allowed_emails = vec!["operator@example.test".into()];
        auth.sqlite_path = directory.path().join("auth.db").display().to_string();
        auth.key_path = directory.path().join("auth.pem").display().to_string();
        auth.access_token_ttl_secs = 123;
        auth.register_rpm = 7;
        auth.max_pending_oauth_states = 9;
        auth.disable_static_token_with_oauth = true;

        let state = build_oauth_state(&config).await.unwrap();
        assert_eq!(state.config.access_token_ttl.as_secs(), 123);
        assert_eq!(state.config.register_requests_per_minute, 7);
        assert_eq!(state.config.max_pending_oauth_states, 9);
        assert!(state.config.disable_static_token_with_oauth);
        let allowed = state.resolve_allowed_emails().await.unwrap();
        assert!(allowed.iter().any(|email| email == "operator@example.test"));
    }

    #[tokio::test]
    async fn configured_allowlist_removals_are_reconciled_without_removing_admin_rows() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = Config::default();
        let auth = &mut config.mcp.auth;
        auth.mode = AuthMode::OAuth;
        auth.public_url = Some("https://apprise.example.test".into());
        auth.google_client_id = Some("client".into());
        auth.google_client_secret = Some("secret".into());
        auth.admin_email = "admin@example.test".into();
        auth.allowed_emails = vec!["old@example.test".into()];
        auth.sqlite_path = directory.path().join("auth.db").display().to_string();
        auth.key_path = directory.path().join("auth.pem").display().to_string();

        let initial = build_oauth_state(&config).await.unwrap();
        initial
            .store
            .add_allowed_user("manual@example.test", "admin", 1)
            .await
            .unwrap();
        drop(initial);

        config.mcp.auth.allowed_emails = vec!["new@example.test".into()];
        let reconciled = build_oauth_state(&config).await.unwrap();
        let rows = reconciled.store.list_allowed_users().await.unwrap();
        assert!(!rows.iter().any(|row| row.email == "old@example.test"));
        assert!(rows
            .iter()
            .any(|row| { row.email == "new@example.test" && row.added_by == "config" }));
        assert!(rows
            .iter()
            .any(|row| { row.email == "manual@example.test" && row.added_by == "admin" }));
    }
}
