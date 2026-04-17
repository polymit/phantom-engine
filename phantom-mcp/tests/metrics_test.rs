#![allow(clippy::unwrap_used, clippy::expect_used)]
use axum::body::Body;
use axum::http::{Request, StatusCode};
use phantom_mcp::{metrics, McpServer};
use tower::util::ServiceExt;

#[tokio::test]
async fn metrics_endpoint_exposes_blueprint_metric_names() {
    let adapter = phantom_mcp::engine::get_test_adapter().await;
    let server = McpServer::new_with_adapter(None, adapter);
    metrics::SESSIONS_ACTIVE.set(2);

    let request = Request::builder()
        .uri("/metrics")
        .body(Body::empty())
        .expect("metrics request should build");

    let response = server
        .router()
        .oneshot(request)
        .await
        .expect("metrics route should respond");
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("metrics body should be readable");
    let text = String::from_utf8_lossy(&body);

    let expected = [
        "sessions_active",
        "sessions_created_total",
        "session_duration_seconds",
        "js_runtimes_used",
        "js_evaluation_duration_seconds",
        "http_requests_total",
        "http_request_duration_seconds",
        "dom_snapshot_duration_seconds",
        "dom_nodes_serialised",
        "storage_quota_used_bytes",
    ];

    for name in expected {
        assert!(
            text.contains(name),
            "metrics output missing required name: {name}"
        );
    }
}

#[tokio::test]
async fn health_endpoint_returns_scaffold_payload() {
    let adapter = phantom_mcp::engine::get_test_adapter().await;
    let server = McpServer::new_with_adapter(None, adapter);

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
    let json: serde_json::Value =
        serde_json::from_slice(&body).expect("health response should be valid JSON");

    assert_eq!(json.get("status").and_then(|v| v.as_str()), Some("healthy"));
    assert!(
        json.get("version").and_then(|v| v.as_str()).is_some(),
        "health payload should include version"
    );
}
