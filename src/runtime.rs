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
    #[error("APPRISE_MCP_NO_AUTH is only allowed when every address for {host:?} is loopback")]
    UnsafeNoAuth { host: String },
    #[error("APPRISE_MCP_TOKEN is required for a non-loopback bearer-authenticated bind")]
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

pub async fn build_state(config: Config) -> Result<AppState, RuntimeError> {
    let is_loopback = host_is_loopback(&config.mcp.host, config.mcp.port).await?;
    if config.mcp.no_auth && !is_loopback {
        return Err(RuntimeError::UnsafeNoAuth {
            host: config.mcp.host.clone(),
        });
    }

    let client = AppriseClient::new(&config.apprise)?;
    let service =
        AppriseService::new(client, config.apprise.url.clone()).with_config(config.mcp.clone());
    let auth_policy = if is_loopback {
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
    Ok(AppState {
        config: config.mcp,
        auth_policy,
        service,
        counters,
        clock,
    })
}

pub async fn host_is_loopback(host: &str, port: u16) -> Result<bool, RuntimeError> {
    let addresses = tokio::net::lookup_host((host, port))
        .await
        .map_err(|source| RuntimeError::Resolve {
            host: host.into(),
            source,
        })?
        .collect::<Vec<_>>();
    Ok(!addresses.is_empty() && addresses.iter().all(|address| address.ip().is_loopback()))
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
    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    for email in &auth.allowed_emails {
        state
            .store
            .add_allowed_user(email, "config", created_at)
            .await
            .map_err(|source| RuntimeError::OAuthAllowlist {
                email: email.clone(),
                source,
            })?;
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
    async fn loopback_resolution_is_semantic() {
        assert!(host_is_loopback("localhost", 40050).await.unwrap());
        assert!(host_is_loopback("127.0.0.1", 40050).await.unwrap());
        assert!(!host_is_loopback("0.0.0.0", 40050).await.unwrap());
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
}
