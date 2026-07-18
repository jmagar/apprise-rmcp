use serde_json::{json, Value};
use thiserror::Error;

use crate::{app::ServiceError, apprise::NotifyType};

use super::AppState;

/// Thin shim — parse args, call service, return Value. No logic here.
#[derive(Debug, Error)]
pub(super) enum ToolError {
    #[error("unknown tool: {0}")]
    UnknownTool(String),
    #[error("{0} is required")]
    MissingArgument(&'static str),
    #[error("invalid notification type {0:?}; expected info|success|warning|failure")]
    InvalidNotifyType(String),
    #[error("unknown apprise action: {0}; use action=help for documentation")]
    UnknownAction(String),
    #[error(transparent)]
    Service(#[from] ServiceError),
}

impl ToolError {
    pub fn is_validation(&self) -> bool {
        !matches!(self, Self::Service(_))
    }
}

pub(super) async fn execute_tool(
    state: &AppState,
    name: &str,
    args: Value,
) -> Result<Value, ToolError> {
    match name {
        "apprise" => dispatch(state, args).await,
        _ => Err(ToolError::UnknownTool(name.into())),
    }
}

async fn dispatch(state: &AppState, args: Value) -> Result<Value, ToolError> {
    let action = string_arg(&args, "action").ok_or(ToolError::MissingArgument("action"))?;

    match action.as_str() {
        "notify" => {
            let body = string_arg(&args, "body").ok_or(ToolError::MissingArgument("body"))?;
            let tag = string_arg(&args, "tag");
            let title = string_arg(&args, "title");
            let notify_type = parse_notify_type(&args)?;

            match tag.as_deref() {
                Some(t) => state
                    .service
                    .notify(t, title.as_deref(), &body, &notify_type)
                    .await
                    .map_err(Into::into),
                None => state
                    .service
                    .notify_all(title.as_deref(), &body, &notify_type)
                    .await
                    .map_err(Into::into),
            }
        }
        "notify_url" => {
            let urls = string_arg(&args, "urls").ok_or(ToolError::MissingArgument("urls"))?;
            let body = string_arg(&args, "body").ok_or(ToolError::MissingArgument("body"))?;
            let title = string_arg(&args, "title");
            let notify_type = parse_notify_type(&args)?;

            state
                .service
                .notify_url(&urls, title.as_deref(), &body, &notify_type)
                .await
                .map_err(Into::into)
        }
        "health" => state.service.health().await.map_err(Into::into),
        "status" => Ok(state.service.status()),
        "help" => Ok(json!({ "help": HELP_TEXT })),
        other => Err(ToolError::UnknownAction(other.into())),
    }
}

fn string_arg(args: &Value, name: &str) -> Option<String> {
    args.get(name).and_then(|v| v.as_str()).map(String::from)
}

fn parse_notify_type(args: &Value) -> Result<NotifyType, ToolError> {
    match string_arg(args, "type") {
        None => Ok(NotifyType::default()),
        Some(value) => NotifyType::from_str_opt(&value).ok_or(ToolError::InvalidNotifyType(value)),
    }
}

const HELP_TEXT: &str = r#"# apprise MCP Tool

Send push notifications via the Apprise API server.
Set the required `action` argument to select the operation.

## Actions

### notify
Send a notification to one or all configured Apprise services.

Required: `body`
Optional:
  - `tag`   — send only to services under this tag (omit = send to all)
  - `title` — notification title
  - `type`  — info (default) | success | warning | failure

### notify_url
Stateless one-off notification to a specific Apprise URL schema.
No pre-configuration needed on the server.

Required: `urls`, `body`
Optional:
  - `title` — notification title
  - `type`  — info (default) | success | warning | failure

Example urls: "slack://tokenA/tokenB/tokenC"
              "discord://webhook_id/webhook_token"
              "mailto://user:pass@gmail.com"

### health
Check the Apprise server health endpoint.

### status
Show runtime uptime and request/upstream counters without contacting Apprise.

### help
Show this documentation.

## Notification types
- `info`    — informational (default)
- `success` — successful operation
- `warning` — non-critical warning
- `failure` — critical failure or error
"#;
