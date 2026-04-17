#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use phantom_mcp::{engine, EngineAdapter, McpServer};
use phantom_session::ResourceBudget;
use tower::util::ServiceExt;

fn ping_body() -> Body {
    Body::from(r#"{"jsonrpc":"2.0","id":"1","method":"ping","params":{}}"#)
}

fn rpc_request(api_key: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/rpc")
        .header("content-type", "application/json");
    if let Some(key) = api_key {
        builder = builder.header("x-api-key", key);
    }
    builder.body(ping_body()).expect("rpc request should build")
}

#[tokio::test]
async fn api_key_missing_returns_401() {
    engine::init_v8();
    let adapter = Arc::new(EngineAdapter::new(1, 0, 1, 0, ResourceBudget::default()).await);
    let server = McpServer::new_with_adapter(Some("secret".to_string()), adapter);

    let response = server
        .router()
        .oneshot(rpc_request(None))
        .await
        .expect("request should complete");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn api_key_invalid_returns_401() {
    engine::init_v8();
    let adapter = Arc::new(EngineAdapter::new(1, 0, 1, 0, ResourceBudget::default()).await);
    let server = McpServer::new_with_adapter(Some("secret".to_string()), adapter);

    let response = server
        .router()
        .oneshot(rpc_request(Some("wrong")))
        .await
        .expect("request should complete");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn api_key_valid_passes_through() {
    engine::init_v8();
    let adapter = Arc::new(EngineAdapter::new(1, 0, 1, 0, ResourceBudget::default()).await);
    let server = McpServer::new_with_adapter(Some("secret".to_string()), adapter);

    let response = server
        .router()
        .oneshot(rpc_request(Some("secret")))
        .await
        .expect("request should complete");
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn rate_limit_exceeded_returns_429() {
    engine::init_v8();
    let adapter = Arc::new(EngineAdapter::new(1, 0, 1, 0, ResourceBudget::default()).await);
    let server = McpServer::new_with_adapter_and_rate_limit(Some("secret".to_string()), adapter, 2);

    let first = server
        .clone()
        .router()
        .oneshot(rpc_request(Some("secret")))
        .await
        .expect("first request should complete");
    assert_eq!(first.status(), StatusCode::OK);

    let second = server
        .clone()
        .router()
        .oneshot(rpc_request(Some("secret")))
        .await
        .expect("second request should complete");
    assert_eq!(second.status(), StatusCode::OK);

    let third = server
        .router()
        .oneshot(rpc_request(Some("secret")))
        .await
        .expect("third request should complete");
    assert_eq!(third.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn health_and_metrics_bypass_api_key_check() {
    engine::init_v8();
    let adapter = Arc::new(EngineAdapter::new(1, 0, 1, 0, ResourceBudget::default()).await);
    let server = McpServer::new_with_adapter(Some("secret".to_string()), adapter);

    let health = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .expect("health request should build");
    let health_response = server
        .clone()
        .router()
        .oneshot(health)
        .await
        .expect("health request should complete");
    assert_eq!(health_response.status(), StatusCode::OK);

    let metrics = Request::builder()
        .uri("/metrics")
        .body(Body::empty())
        .expect("metrics request should build");
    let metrics_response = server
        .router()
        .oneshot(metrics)
        .await
        .expect("metrics request should complete");
    assert_eq!(metrics_response.status(), StatusCode::OK);
}
