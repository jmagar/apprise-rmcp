use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

const DEFAULT_MAX_CONCURRENT_REQUESTS: usize = 32;
const DEFAULT_MAX_RESPONSE_BYTES: usize = 64 * 1024;
const MAX_CONCURRENT_REQUESTS: usize = 1_024;
const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read configuration at {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse configuration at {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("failed to inspect dotenv file at {path}: {source}")]
    DotenvMetadata {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("refusing to load symlinked dotenv file at {0}")]
    SymlinkedDotenv(PathBuf),
    #[error("failed to parse dotenv file at {path}: {source}")]
    Dotenv {
        path: PathBuf,
        #[source]
        source: dotenvy::Error,
    },
    #[error("{key}: invalid value {value:?}")]
    InvalidValue { key: String, value: String },
    #[error("APPRISE_URL must be an absolute http or https URL, got {0:?}")]
    InvalidAppriseUrl(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub mcp: McpConfig,
    pub apprise: AppriseConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppriseConfig {
    pub url: String,
    pub token: String,
    pub max_concurrent_requests: usize,
    pub max_response_bytes: usize,
}

impl Default for AppriseConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:8000".into(),
            token: String::new(),
            max_concurrent_requests: DEFAULT_MAX_CONCURRENT_REQUESTS,
            max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpConfig {
    #[serde(default = "default_mcp_host")]
    pub host: String,
    #[serde(default = "default_mcp_port")]
    pub port: u16,
    #[serde(default = "default_server_name")]
    pub server_name: String,
    pub no_auth: bool,
    pub api_token: Option<String>,
    pub allowed_hosts: Vec<String>,
    pub allowed_origins: Vec<String>,
    pub auth: AuthConfig,
}

impl McpConfig {
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AuthConfig {
    pub mode: AuthMode,
    pub public_url: Option<String>,
    pub google_client_id: Option<String>,
    pub google_client_secret: Option<String>,
    pub admin_email: String,
    pub allowed_emails: Vec<String>,
    pub sqlite_path: String,
    pub key_path: String,
    pub access_token_ttl_secs: u64,
    pub refresh_token_ttl_secs: u64,
    pub auth_code_ttl_secs: u64,
    pub register_rpm: u32,
    pub authorize_rpm: u32,
    pub max_pending_oauth_states: usize,
    pub disable_static_token_with_oauth: bool,
    pub allowed_client_redirect_uris: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AuthMode {
    #[default]
    Bearer,
    OAuth,
}

impl FromStr for AuthMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "bearer" => Ok(Self::Bearer),
            "oauth" => Ok(Self::OAuth),
            _ => Err(format!("expected bearer or oauth, got {value:?}")),
        }
    }
}

/// One canonical application-data directory for setup, dotenv, logging, and OAuth.
pub fn default_data_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("APPRISE_HOME").filter(|v| !v.is_empty()) {
        return PathBuf::from(path);
    }
    if Path::new("/.dockerenv").exists()
        || std::env::var("RUNNING_IN_CONTAINER").is_ok()
        || std::env::var("container").is_ok()
    {
        return PathBuf::from("/data");
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".apprise")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DotenvOutcome {
    Missing,
    Loaded(HashMap<String, String>),
}

pub fn load_dotenv() -> Result<DotenvOutcome, ConfigError> {
    load_dotenv_from(&default_data_dir().join(".env"))
}

pub fn load_dotenv_from(path: &Path) -> Result<DotenvOutcome, ConfigError> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(ConfigError::SymlinkedDotenv(path.to_path_buf()));
        }
        Ok(_) => {}
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            return Ok(DotenvOutcome::Missing);
        }
        Err(source) => {
            return Err(ConfigError::DotenvMetadata {
                path: path.to_path_buf(),
                source,
            });
        }
    }

    let iter = dotenvy::from_path_iter(path).map_err(|source| ConfigError::Dotenv {
        path: path.to_path_buf(),
        source,
    })?;
    let mut values = HashMap::new();
    for item in iter {
        let (key, value) = item.map_err(|source| ConfigError::Dotenv {
            path: path.to_path_buf(),
            source,
        })?;
        values.insert(key, value);
    }
    Ok(DotenvOutcome::Loaded(values))
}

fn default_mcp_host() -> String {
    "0.0.0.0".into()
}
fn default_mcp_port() -> u16 {
    40050
}
fn default_server_name() -> String {
    "apprise-mcp".into()
}
fn default_auth_sqlite_path() -> String {
    default_data_dir().join("auth.db").display().to_string()
}
fn default_auth_key_path() -> String {
    default_data_dir()
        .join("auth-jwt.pem")
        .display()
        .to_string()
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            host: default_mcp_host(),
            port: default_mcp_port(),
            server_name: default_server_name(),
            no_auth: false,
            api_token: None,
            allowed_hosts: Vec::new(),
            allowed_origins: Vec::new(),
            auth: AuthConfig::default(),
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            mode: AuthMode::default(),
            public_url: None,
            google_client_id: None,
            google_client_secret: None,
            admin_email: String::new(),
            allowed_emails: Vec::new(),
            sqlite_path: default_auth_sqlite_path(),
            key_path: default_auth_key_path(),
            access_token_ttl_secs: 3600,
            refresh_token_ttl_secs: 86400 * 30,
            auth_code_ttl_secs: 300,
            register_rpm: 10,
            authorize_rpm: 60,
            max_pending_oauth_states: 1024,
            disable_static_token_with_oauth: true,
            allowed_client_redirect_uris: Vec::new(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        Self::load_with_overrides(std::iter::empty::<(String, String)>())
    }

    pub fn load_with_overrides(
        overrides: impl IntoIterator<Item = (String, String)>,
    ) -> Result<Self, ConfigError> {
        let dotenv = match load_dotenv()? {
            DotenvOutcome::Missing => HashMap::new(),
            DotenvOutcome::Loaded(values) => values,
        };
        Self::load_from_sources(
            Path::new("config.toml"),
            dotenv,
            std::env::vars(),
            overrides,
        )
    }

    pub fn load_from_sources(
        config_path: &Path,
        dotenv: impl IntoIterator<Item = (String, String)>,
        environment: impl IntoIterator<Item = (String, String)>,
        overrides: impl IntoIterator<Item = (String, String)>,
    ) -> Result<Self, ConfigError> {
        let mut config = match std::fs::read_to_string(config_path) {
            Ok(contents) => toml::from_str(&contents).map_err(|source| ConfigError::Parse {
                path: config_path.to_path_buf(),
                source,
            })?,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(source) => {
                return Err(ConfigError::Read {
                    path: config_path.to_path_buf(),
                    source,
                });
            }
        };

        let mut vars: HashMap<String, String> = dotenv.into_iter().collect();
        vars.extend(environment);
        vars.extend(overrides);
        config.apply_vars(&vars)?;
        config.validate()?;
        Ok(config)
    }

    fn apply_vars(&mut self, vars: &HashMap<String, String>) -> Result<(), ConfigError> {
        set_string(vars, "APPRISE_MCP_HOST", &mut self.mcp.host);
        set_parse(vars, "APPRISE_MCP_PORT", &mut self.mcp.port)?;
        set_bool(vars, "APPRISE_MCP_NO_AUTH", &mut self.mcp.no_auth)?;
        set_option(vars, "APPRISE_MCP_TOKEN", &mut self.mcp.api_token);
        set_list(
            vars,
            "APPRISE_MCP_ALLOWED_HOSTS",
            &mut self.mcp.allowed_hosts,
        );
        set_list(
            vars,
            "APPRISE_MCP_ALLOWED_ORIGINS",
            &mut self.mcp.allowed_origins,
        );

        set_option(
            vars,
            "APPRISE_MCP_PUBLIC_URL",
            &mut self.mcp.auth.public_url,
        );
        set_option(
            vars,
            "APPRISE_MCP_GOOGLE_CLIENT_ID",
            &mut self.mcp.auth.google_client_id,
        );
        set_option(
            vars,
            "APPRISE_MCP_GOOGLE_CLIENT_SECRET",
            &mut self.mcp.auth.google_client_secret,
        );
        set_string(
            vars,
            "APPRISE_MCP_AUTH_ADMIN_EMAIL",
            &mut self.mcp.auth.admin_email,
        );
        set_list(
            vars,
            "APPRISE_MCP_AUTH_ALLOWED_EMAILS",
            &mut self.mcp.auth.allowed_emails,
        );
        set_string(
            vars,
            "APPRISE_MCP_AUTH_SQLITE_PATH",
            &mut self.mcp.auth.sqlite_path,
        );
        set_string(
            vars,
            "APPRISE_MCP_AUTH_KEY_PATH",
            &mut self.mcp.auth.key_path,
        );
        set_parse(
            vars,
            "APPRISE_MCP_AUTH_ACCESS_TOKEN_TTL_SECS",
            &mut self.mcp.auth.access_token_ttl_secs,
        )?;
        set_parse(
            vars,
            "APPRISE_MCP_AUTH_REFRESH_TOKEN_TTL_SECS",
            &mut self.mcp.auth.refresh_token_ttl_secs,
        )?;
        set_parse(
            vars,
            "APPRISE_MCP_AUTH_CODE_TTL_SECS",
            &mut self.mcp.auth.auth_code_ttl_secs,
        )?;
        set_parse(
            vars,
            "APPRISE_MCP_AUTH_REGISTER_REQUESTS_PER_MINUTE",
            &mut self.mcp.auth.register_rpm,
        )?;
        set_parse(
            vars,
            "APPRISE_MCP_AUTH_AUTHORIZE_REQUESTS_PER_MINUTE",
            &mut self.mcp.auth.authorize_rpm,
        )?;
        set_parse(
            vars,
            "APPRISE_MCP_AUTH_MAX_PENDING_OAUTH_STATES",
            &mut self.mcp.auth.max_pending_oauth_states,
        )?;
        set_bool(
            vars,
            "APPRISE_MCP_DISABLE_STATIC_TOKEN_WITH_OAUTH",
            &mut self.mcp.auth.disable_static_token_with_oauth,
        )?;
        set_list(
            vars,
            "APPRISE_MCP_AUTH_ALLOWED_REDIRECT_URIS",
            &mut self.mcp.auth.allowed_client_redirect_uris,
        );
        if let Some(value) = nonempty(vars, "APPRISE_MCP_AUTH_MODE") {
            self.mcp.auth.mode = value.parse().map_err(|_| ConfigError::InvalidValue {
                key: "APPRISE_MCP_AUTH_MODE".into(),
                value: value.into(),
            })?;
        }

        set_string(vars, "APPRISE_URL", &mut self.apprise.url);
        set_string(vars, "APPRISE_TOKEN", &mut self.apprise.token);
        set_parse(
            vars,
            "APPRISE_MAX_CONCURRENT_REQUESTS",
            &mut self.apprise.max_concurrent_requests,
        )?;
        set_parse(
            vars,
            "APPRISE_MAX_RESPONSE_BYTES",
            &mut self.apprise.max_response_bytes,
        )?;

        if nonempty(vars, "APPRISE_MCP_DISABLE_HTTP_AUTH").is_some_and(|value| {
            matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes")
        }) {
            self.mcp.no_auth = true;
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), ConfigError> {
        let url = url::Url::parse(&self.apprise.url)
            .map_err(|_| ConfigError::InvalidAppriseUrl(self.apprise.url.clone()))?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            return Err(ConfigError::InvalidAppriseUrl(self.apprise.url.clone()));
        }
        if !(1..=MAX_CONCURRENT_REQUESTS).contains(&self.apprise.max_concurrent_requests) {
            return Err(ConfigError::InvalidValue {
                key: "APPRISE_MAX_CONCURRENT_REQUESTS".into(),
                value: self.apprise.max_concurrent_requests.to_string(),
            });
        }
        if !(1..=MAX_RESPONSE_BYTES).contains(&self.apprise.max_response_bytes) {
            return Err(ConfigError::InvalidValue {
                key: "APPRISE_MAX_RESPONSE_BYTES".into(),
                value: self.apprise.max_response_bytes.to_string(),
            });
        }
        Ok(())
    }
}

fn nonempty<'a>(vars: &'a HashMap<String, String>, key: &str) -> Option<&'a str> {
    vars.get(key)
        .map(String::as_str)
        .filter(|value| !value.is_empty())
}

fn set_string(vars: &HashMap<String, String>, key: &str, target: &mut String) {
    if let Some(value) = nonempty(vars, key) {
        *target = value.to_string();
    }
}

fn set_option(vars: &HashMap<String, String>, key: &str, target: &mut Option<String>) {
    if let Some(value) = nonempty(vars, key) {
        *target = Some(value.to_string());
    }
}

fn set_list(vars: &HashMap<String, String>, key: &str, target: &mut Vec<String>) {
    if let Some(value) = nonempty(vars, key) {
        *target = value
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(str::to_string)
            .collect();
    }
}

fn set_parse<T: FromStr>(
    vars: &HashMap<String, String>,
    key: &str,
    target: &mut T,
) -> Result<(), ConfigError> {
    if let Some(value) = nonempty(vars, key) {
        *target = value.parse().map_err(|_| ConfigError::InvalidValue {
            key: key.into(),
            value: value.into(),
        })?;
    }
    Ok(())
}

fn set_bool(
    vars: &HashMap<String, String>,
    key: &str,
    target: &mut bool,
) -> Result<(), ConfigError> {
    if let Some(value) = nonempty(vars, key) {
        *target = match value.to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" => true,
            "0" | "false" | "no" => false,
            _ => {
                return Err(ConfigError::InvalidValue {
                    key: key.into(),
                    value: value.into(),
                });
            }
        };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_precedence_is_config_then_dotenv_then_env_then_override() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        std::fs::write(&path, "[apprise]\nurl = 'http://config.example'\n").unwrap();
        let config = Config::load_from_sources(
            &path,
            [("APPRISE_URL".into(), "http://dotenv.example".into())],
            [("APPRISE_URL".into(), "http://env.example".into())],
            [("APPRISE_URL".into(), "http://override.example".into())],
        )
        .unwrap();
        assert_eq!(config.apprise.url, "http://override.example");
    }

    #[test]
    fn invalid_auth_mode_and_url_are_rejected() {
        let missing = Path::new("/definitely/missing/config.toml");
        assert!(Config::load_from_sources(
            missing,
            [],
            [("APPRISE_MCP_AUTH_MODE".into(), "typo".into())],
            []
        )
        .is_err());
        assert!(Config::load_from_sources(
            missing,
            [],
            [("APPRISE_URL".into(), "file:///tmp/apprise".into())],
            []
        )
        .is_err());
    }

    #[test]
    fn malformed_dotenv_is_reported() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(".env");
        std::fs::write(&path, "BROKEN='unterminated\n").unwrap();
        assert!(matches!(
            load_dotenv_from(&path),
            Err(ConfigError::Dotenv { .. })
        ));
    }

    #[test]
    fn excessive_concurrency_is_rejected_during_config_load() {
        let missing = Path::new("/definitely/missing/config.toml");
        let result = Config::load_from_sources(
            missing,
            [],
            [(
                "APPRISE_MAX_CONCURRENT_REQUESTS".into(),
                usize::MAX.to_string(),
            )],
            [],
        );
        assert!(
            matches!(result, Err(ConfigError::InvalidValue { key, .. }) if key == "APPRISE_MAX_CONCURRENT_REQUESTS")
        );
    }

    #[test]
    fn excessive_response_limit_is_rejected_during_config_load() {
        let missing = Path::new("/definitely/missing/config.toml");
        let result = Config::load_from_sources(
            missing,
            [],
            [("APPRISE_MAX_RESPONSE_BYTES".into(), usize::MAX.to_string())],
            [],
        );
        assert!(
            matches!(result, Err(ConfigError::InvalidValue { key, .. }) if key == "APPRISE_MAX_RESPONSE_BYTES")
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_dotenv_is_refused() {
        use std::os::unix::fs::symlink;
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("real.env");
        let link = directory.path().join(".env");
        std::fs::write(&target, "APPRISE_URL=http://example.test\n").unwrap();
        symlink(&target, &link).unwrap();
        assert!(matches!(
            load_dotenv_from(&link),
            Err(ConfigError::SymlinkedDotenv(_))
        ));
    }
}
