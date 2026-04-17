#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use phantom_mcp::{engine, EngineAdapter, McpServer};
use phantom_session::{EngineKind, ResourceBudget};
use tower::util::ServiceExt;

async fn health_json(server: McpServer) -> serde_json::Value {
    let request = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .expect("health request should build");
    let response = server
        .router()
        .oneshot(request)
        .await
        .expect("health route should respond");
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("health body should be readable");
    serde_json::from_slice(&body).expect("health response should be valid JSON")
}

#[tokio::test]
async fn health_returns_healthy_on_empty_system() {
    engine::init_v8();
    let adapter = Arc::new(EngineAdapter::new(1, 0, 1, 0, ResourceBudget::default()).await);
    let _ = adapter.broker.remove(adapter.session_uuid);
    let server = McpServer::new_with_adapter(None, adapter);

    let health = health_json(server).await;
    assert_eq!(health["status"], "healthy");
    assert_eq!(health["sessions"]["active"], 0);
}

#[tokio::test]
async fn health_returns_degraded_above_80pct_utilization() {
    engine::init_v8();
    let adapter = Arc::new(EngineAdapter::new(1, 0, 1, 0, ResourceBudget::default()).await);
    for _ in 0..800 {
        adapter
            .broker
            .create(EngineKind::QuickJs, ResourceBudget::default(), "persona");
    }
    let server = McpServer::new_with_adapter(None, adapter);

    let health = health_json(server).await;
    assert_eq!(health["status"], "degraded");
}

#[tokio::test]
async fn health_returns_unhealthy_above_95pct_utilization() {
    engine::init_v8();
    let adapter = Arc::new(EngineAdapter::new(1, 0, 1, 0, ResourceBudget::default()).await);
    for _ in 0..950 {
        adapter
            .broker
            .create(EngineKind::QuickJs, ResourceBudget::default(), "persona");
    }
    let server = McpServer::new_with_adapter(None, adapter);

    let health = health_json(server).await;
    assert_eq!(health["status"], "unhealthy");
}

#[tokio::test]
async fn health_circuit_breaker_state_reflected_in_response() {
    engine::init_v8();
    let adapter = Arc::new(EngineAdapter::new(0, 0, 1, 0, ResourceBudget::default()).await);
    for _ in 0..5 {
        let _ = adapter.tier1.acquire().await;
    }
    let server = McpServer::new_with_adapter(None, adapter);

    let health = health_json(server).await;
    assert_eq!(health["pools"]["tier1"]["circuit_breaker"], "open");
    assert_eq!(health["status"], "unhealthy");
}
