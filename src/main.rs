use anyhow::Result;
use rmcp::{transport::stdio, ServiceExt};
use tracing::info;

use apprise_mcp::{
    app::AppriseService,
    apprise::AppriseClient,
    config::Config,
    mcp::{self, AppState, AuthPolicy},
    runtime::build_state,
};

mod cli;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.as_slice() {
        [f] if matches!(f.as_str(), "--help" | "-h" | "help") => {
            print_usage();
            return Ok(());
        }
        [f] if matches!(f.as_str(), "--version" | "-V" | "version") => {
            println!("apprise-mcp {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        _ => {}
    }

    let stdio_mode = matches!(args.as_slice(), [c] if c == "mcp");
    let serve_mode = args.is_empty()
        || matches!(args.as_slice(), [c] if c == "serve")
        || matches!(args.as_slice(), [a, b] if a == "serve" && b == "mcp");

    if serve_mode {
        apprise_mcp::logging::init(&apprise_mcp::config::default_data_dir(), "info")?;
    } else {
        apprise_mcp::logging::init_console("warn");
    }

    if serve_mode {
        serve_mcp().await
    } else if stdio_mode {
        serve_stdio_mcp().await
    } else {
        run_cli(args).await
    }
}

async fn serve_mcp() -> Result<()> {
    let config = Config::load()?;
    let state = build_state(config).await?;

    info!(
        bind = %state.config.bind_addr(),
        server_name = %state.config.server_name,
        auth = ?state.auth_policy,
        "apprise-mcp starting"
    );

    let bind = state.config.bind_addr();
    let app = mcp::router(state).layer(tower_http::trace::TraceLayer::new_for_http());
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    info!(bind = %bind, "MCP HTTP server listening");

    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn serve_stdio_mcp() -> Result<()> {
    // Stdio is always LoopbackDev — trusted local pipe, no HTTP auth context.
    let config = Config::load()?;
    let service = AppriseService::new(
        AppriseClient::new(&config.apprise)?,
        config.apprise.url.clone(),
    )
    .with_config(config.mcp.clone());
    let counters = service.counters.clone();
    let clock = service.clock.clone();
    #[allow(clippy::useless_conversion)]
    let state = AppState {
        config: config.mcp,
        auth_policy: AuthPolicy::LoopbackDev,
        service,
        counters,
        clock,
    };
    let _ = (); // appease compiler
    let svc = mcp::rmcp_server(state).serve(stdio()).await?;
    svc.waiting().await?;
    Ok(())
}

async fn run_cli(args: Vec<String>) -> Result<()> {
    let json = args.iter().any(|a| a == "--json");
    let filtered: Vec<String> = args.into_iter().filter(|a| a != "--json").collect();

    // doctor/setup run before service construction.
    if matches!(filtered.as_slice(), [c] if c == "doctor") {
        let config = Config::load()?;
        return cli::run_doctor(&config, json).await;
    }

    let parsed = cli::CliCommand::parse(&filtered)?;
    if let cli::CliCommand::Setup(command) = parsed {
        // Translate CLAUDE_PLUGIN_OPTION_* into APPRISE_* env vars BEFORE
        // Config::load() so the plugin hook can call the binary directly (no
        // plugin-setup.sh wrapper). apprise is template-style: the setup check
        // validates the pre-loaded &Config, so this must precede the load.
        let config = Config::load_with_overrides(cli::plugin_overrides())?;
        return cli::run_setup(&config, command).await;
    }

    let config = Config::load()?;
    let service = AppriseService::new(
        AppriseClient::new(&config.apprise)?,
        config.apprise.url.clone(),
    );
    cli::run(&service, parsed, json).await
}

fn print_usage() {
    eprintln!(
        "Usage:
  rapprise [serve]                   Start MCP HTTP server (default)
  rapprise mcp                       Start MCP stdio transport
  rapprise doctor                    Pre-flight environment validation
  rapprise setup check               Check plugin setup without mutating appdata
  rapprise setup repair              Create missing appdata/env setup files
  rapprise setup plugin-hook [--no-repair] Plugin hook JSON contract

Notification:
  rapprise notify <body> [--tag TAG] [--title T] [--type info|success|warning|failure]
  rapprise notify-url <urls> <body> [--title T] [--type ...]

Server:
  rapprise health [--json]           Check Apprise server health
  rapprise doctor [--json]           Validate environment before deployment

Options:
  --json                             Output raw JSON

Environment:
  APPRISE_URL                  Apprise API server URL (default: http://localhost:8000)
  APPRISE_TOKEN                API token (optional for open installs)
  APPRISE_MCP_HOST             Bind host (default: 0.0.0.0)
  APPRISE_MCP_PORT             Bind port (default: 40050)
  APPRISE_MCP_NO_AUTH          Disable MCP HTTP auth (loopback only)
  APPRISE_MCP_TOKEN            Static bearer token for MCP HTTP auth
  APPRISE_MCP_AUTH_MODE        Auth mode: bearer (default) | oauth
  APPRISE_MCP_PUBLIC_URL       Public URL for OAuth metadata endpoints
  APPRISE_MCP_GOOGLE_CLIENT_ID     Google OAuth client ID
  APPRISE_MCP_GOOGLE_CLIENT_SECRET Google OAuth client secret
  APPRISE_MCP_AUTH_ADMIN_EMAIL     Admin email for OAuth
  RUST_LOG                     Log filter"
    );
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!(error = %e, "CTRL+C handler failed");
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            Err(e) => {
                tracing::error!(error = %e, "SIGTERM handler failed");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
    tracing::info!("Shutdown signal received");
}
