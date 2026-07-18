# Technology choices

- Rust/Tokio: native `rapprise` for CLI, stdio, and HTTP.
- rmcp: MCP schemas, prompts, lifecycle, and transports.
- Axum/tower-http: HTTP routing, limits, CORS, and tracing.
- reqwest/rustls: outbound Apprise API calls.
- serde/TOML/dotenvy: config and canonical data-directory env.
- lab-auth: bearer and Google OAuth/JWT for HTTP MCP.
- npm launcher: downloads and launches matching native assets.

This application does not ingest syslog, index logs, or own notification
destinations; the upstream Apprise API owns destination configuration.
