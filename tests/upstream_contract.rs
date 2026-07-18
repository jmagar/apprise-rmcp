use std::sync::{Arc, Mutex};
use std::time::Duration;

use apprise_mcp::apprise::{AppriseClient, NotifyType, UpstreamError};
use apprise_mcp::config::AppriseConfig;
use apprise_mcp::mcp::{build_auth_layer, AuthPolicy};
use axum::body::{to_bytes, Body};
use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{any, get};
use axum::Router;
use serde_json::{json, Value};
use tower::ServiceExt;

#[derive(Debug)]
struct Captured {
    uri: String,
    headers: HeaderMap,
    body: Value,
}

async fn spawn(router: Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    format!("http://{address}")
}

fn config(url: String) -> AppriseConfig {
    AppriseConfig {
        url,
        token: "secret".into(),
        max_concurrent_requests: 2,
        max_response_bytes: 1024,
    }
}

#[tokio::test]
async fn notify_encodes_tag_and_sends_payload_and_auth_headers() {
    let captured = Arc::new(Mutex::new(None));
    let router = Router::new()
        .fallback(any(
            |State(captured): State<Arc<Mutex<Option<Captured>>>>, request: Request| async move {
                let uri = request.uri().to_string();
                let headers = request.headers().clone();
                let body = to_bytes(request.into_body(), 4096).await.unwrap();
                *captured.lock().unwrap() = Some(Captured {
                    uri,
                    headers,
                    body: serde_json::from_slice(&body).unwrap(),
                });
                axum::Json(json!({ "delivered": true }))
            },
        ))
        .with_state(captured.clone());
    let client = AppriseClient::new(&config(spawn(router).await)).unwrap();
    let response = client
        .notify("ops/critical", Some("Alert"), "body", &NotifyType::Warning)
        .await
        .unwrap();
    assert_eq!(response["delivered"], true);

    let request = captured.lock().unwrap().take().unwrap();
    assert_eq!(request.uri, "/notify/ops%2Fcritical");
    assert_eq!(request.headers["authorization"], "Bearer secret");
    assert_eq!(request.headers["x-apprise-api-key"], "secret");
    assert_eq!(request.body["title"], "Alert");
    assert_eq!(request.body["type"], "warning");
}

#[tokio::test]
async fn response_size_and_http_status_are_typed_errors() {
    let oversized = spawn(Router::new().route("/health", get(|| async { "x".repeat(128) }))).await;
    let mut small = config(oversized);
    small.max_response_bytes = 32;
    assert!(matches!(
        AppriseClient::new(&small).unwrap().health().await,
        Err(UpstreamError::ResponseTooLarge { limit: 32 })
    ));

    let failed = spawn(Router::new().route(
        "/health",
        get(|| async { (StatusCode::SERVICE_UNAVAILABLE, "maintenance") }),
    ))
    .await;
    assert!(matches!(
        AppriseClient::new(&config(failed)).unwrap().health().await,
        Err(UpstreamError::HttpStatus {
            status: StatusCode::SERVICE_UNAVAILABLE,
            ..
        })
    ));
}

#[tokio::test]
async fn concurrency_limit_load_sheds_without_queueing() {
    let url = spawn(Router::new().route(
        "/health",
        get(|| async {
            tokio::time::sleep(Duration::from_millis(250)).await;
            "ok"
        }),
    ))
    .await;
    let mut limited = config(url);
    limited.max_concurrent_requests = 1;
    let client = AppriseClient::new(&limited).unwrap();
    let first = {
        let client = client.clone();
        tokio::spawn(async move { client.health().await })
    };
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(matches!(
        client.health().await,
        Err(UpstreamError::Overloaded)
    ));
    assert!(first.await.unwrap().is_ok());
}

#[tokio::test]
async fn readiness_reports_unavailable_upstream() {
    let app = apprise_mcp::mcp::router(apprise_mcp::testing::stub_state());
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn bearer_auth_rejects_missing_and_accepts_matching_token() {
    let layer = build_auth_layer(
        &AuthPolicy::Mounted { auth_state: None },
        Some(Arc::<str>::from("mcp-secret")),
        None,
    )
    .unwrap();
    let app = Router::new()
        .route("/protected", get(|| async { "ok" }))
        .layer(layer);

    let missing = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/protected")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::UNAUTHORIZED);

    let accepted = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/protected")
                .header("authorization", "Bearer mcp-secret")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(accepted.status(), StatusCode::OK);
}

#[tokio::test]
async fn oauth_default_rejects_static_bearer_token() {
    use apprise_mcp::config::{AuthMode, Config};

    let directory = tempfile::tempdir().unwrap();
    let mut config = Config::default();
    config.mcp.auth.mode = AuthMode::OAuth;
    config.mcp.auth.public_url = Some("https://apprise.example.test".into());
    config.mcp.auth.google_client_id = Some("client".into());
    config.mcp.auth.google_client_secret = Some("secret".into());
    config.mcp.auth.admin_email = "admin@example.test".into();
    config.mcp.auth.sqlite_path = directory.path().join("auth.db").display().to_string();
    config.mcp.auth.key_path = directory.path().join("auth.pem").display().to_string();
    let auth_state = apprise_mcp::runtime::build_oauth_state(&config)
        .await
        .unwrap();
    let policy = AuthPolicy::Mounted {
        auth_state: Some(Arc::new(auth_state)),
    };
    let layer = build_auth_layer(
        &policy,
        Some(Arc::<str>::from("legacy-static-token")),
        Some(Arc::<str>::from("https://apprise.example.test/mcp")),
    )
    .unwrap();
    let response = Router::new()
        .route("/protected", get(|| async { "ok" }))
        .layer(layer)
        .oneshot(
            axum::http::Request::builder()
                .uri("/protected")
                .header("authorization", "Bearer legacy-static-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
