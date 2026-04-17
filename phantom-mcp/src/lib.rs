pub mod engine;
pub mod metrics;
pub mod telemetry;
pub mod tools;

pub use engine::EngineAdapter;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::{from_fn_with_state, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use phantom_core::DEFAULT_SESSIONS_PER_HOUR;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const SESSION_LIMIT: usize = 1000;
const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60 * 60);

#[derive(Debug)]
struct ApiRateLimiter {
    sessions_per_hour: u32,
    requests: Mutex<HashMap<String, VecDeque<Instant>>>,
}

impl ApiRateLimiter {
    fn new(sessions_per_hour: u32) -> Self {
        Self {
            sessions_per_hour,
            requests: Mutex::new(HashMap::new()),
        }
    }

    fn is_rate_limited(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut guard = match self.requests.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let queue = guard.entry(key.to_string()).or_default();

        while let Some(front) = queue.front() {
            if now.duration_since(*front) <= RATE_LIMIT_WINDOW {
                break;
            }
            queue.pop_front();
        }

        if queue.len() >= self.sessions_per_hour as usize {
            return true;
        }

        queue.push_back(now);
        false
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("invalid auth key")]
    Unauthorized,
    #[error("invalid JSON-RPC request: {0}")]
    InvalidRequest(String),
}

#[derive(Clone)]
pub struct McpServer {
    api_key: Option<String>,
    rate_limiter: Arc<ApiRateLimiter>,
    pub adapter: Arc<EngineAdapter>,
}

impl McpServer {
    pub fn new_with_adapter(api_key: Option<String>, adapter: Arc<EngineAdapter>) -> Self {
        Self::new_with_adapter_and_rate_limit(api_key, adapter, DEFAULT_SESSIONS_PER_HOUR)
    }

    pub fn new_with_adapter_and_rate_limit(
        api_key: Option<String>,
        adapter: Arc<EngineAdapter>,
        sessions_per_hour: u32,
    ) -> Self {
        Self {
            api_key,
            rate_limiter: Arc::new(ApiRateLimiter::new(sessions_per_hour)),
            adapter,
        }
    }

    /// Build an axum `Router` that serves the JSON-RPC endpoint.
    pub fn router(self) -> Router {
        let protected = Router::new()
            .route("/rpc", post(rpc_endpoint))
            .route("/sse", get(crate::tools::subscribe::sse_handler))
            .route_layer(from_fn_with_state(self.clone(), require_api_key));

        Router::new()
            .route("/metrics", get(handle_metrics))
            .route("/health", get(handle_health))
            .merge(protected)
            .with_state(self)
    }

    pub fn parse_request(body: &str) -> Result<JsonRpcRequest, McpError> {
        let req: JsonRpcRequest = serde_json::from_str(body)
            .map_err(|e| McpError::InvalidRequest(format!("json parse error: {e}")))?;
        if req.jsonrpc != "2.0" {
            return Err(McpError::InvalidRequest(
                "jsonrpc must be \"2.0\"".to_string(),
            ));
        }
        if req.method.trim().is_empty() {
            return Err(McpError::InvalidRequest(
                "method must not be empty".to_string(),
            ));
        }
        Ok(req)
    }

    pub async fn handle_request(
        &self,
        adapter: &EngineAdapter,
        req: JsonRpcRequest,
        provided_key: Option<&str>,
    ) -> Result<JsonRpcResponse, McpError> {
        if let Some(expected) = &self.api_key {
            let actual = provided_key.unwrap_or_default();
            if actual != expected {
                return Err(McpError::Unauthorized);
            }
        }

        let req_id = req.id.clone();

        // Converts a tool handler result into a JsonRpcResponse. All
        // tools use the same {"error":{"code":...,"message":...}} error body
        // shape, so one helper eliminates 20 lines of repetition per arm.
        let tool_response = |req_id: Value,
                             outcome: Result<Value, (axum::http::StatusCode, Value)>,
                             fallback_msg: &str,
                             fallback_code: &str|
         -> JsonRpcResponse {
            match outcome {
                Ok(result) => JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req_id,
                    result: Some(result),
                    error: None,
                },
                Err((_status, err_body)) => {
                    let message = err_body
                        .get("error")
                        .and_then(|e| e.get("message"))
                        .and_then(|m| m.as_str())
                        .unwrap_or(fallback_msg)
                        .to_string();
                    let code_str = err_body
                        .get("error")
                        .and_then(|e| e.get("code"))
                        .and_then(|c| c.as_str())
                        .unwrap_or(fallback_code)
                        .to_string();
                    JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req_id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32000,
                            message: format!("{}: {}", code_str, message),
                        }),
                    }
                }
            }
        };

        let resp = match req.method.as_str() {
            "ping" => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req_id,
                result: Some(json!({ "ok": true, "pong": true })),
                error: None,
            },

            _ => {
                // All other tools require the session lock to serialize execution
                let _permit = match tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    adapter.session_lock.clone().acquire_owned(),
                )
                .await
                {
                    Ok(Ok(permit)) => permit,
                    _ => {
                        return Ok(JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id: req_id,
                            result: None,
                            error: Some(JsonRpcError {
                                code: -32002,
                                message:
                                    "Session busy: another task is currently holding the lock."
                                        .to_string(),
                            }),
                        });
                    }
                };

                match req.method.as_str() {
                    "browser_navigate" => {
                        let adapter = adapter.clone();
                        let params = req.params;
                        let rt_handle = tokio::runtime::Handle::current();
                        let outcome = tokio::task::spawn_blocking(move || {
                            rt_handle.block_on(async move {
                                tools::navigate::handle_navigate(&adapter, params).await
                            })
                        })
                        .await
                        .map_err(|join_err| {
                            McpError::InvalidRequest(format!("rpc handler task failed: {join_err}"))
                        })?;
                        tool_response(req_id, outcome, "navigation failed", "navigation_error")
                    }

                    "browser_get_scene_graph" => {
                        let adapter = adapter.clone();
                        let params = req.params;
                        let rt_handle = tokio::runtime::Handle::current();
                        let outcome = tokio::task::spawn_blocking(move || {
                            rt_handle.block_on(async move {
                                tools::scene_graph::handle_get_scene_graph(&adapter, params).await
                            })
                        })
                        .await
                        .map_err(|join_err| {
                            McpError::InvalidRequest(format!("rpc handler task failed: {join_err}"))
                        })?;
                        tool_response(req_id, outcome, "scene graph failed", "scene_graph_error")
                    }

                    "browser_click" => {
                        let adapter = adapter.clone();
                        let params = req.params;
                        let rt_handle = tokio::runtime::Handle::current();
                        let outcome = tokio::task::spawn_blocking(move || {
                            rt_handle.block_on(async move {
                                tools::click::handle_click(&adapter, params).await
                            })
                        })
                        .await
                        .map_err(|join_err| {
                            McpError::InvalidRequest(format!("rpc handler task failed: {join_err}"))
                        })?;
                        tool_response(req_id, outcome, "click failed", "click_error")
                    }

                    "browser_evaluate" => {
                        let adapter = adapter.clone();
                        let params = req.params;
                        let rt_handle = tokio::runtime::Handle::current();
                        let outcome = tokio::task::spawn_blocking(move || {
                            rt_handle.block_on(async move {
                                tools::evaluate::handle_evaluate(&adapter, params).await
                            })
                        })
                        .await
                        .map_err(|join_err| {
                            McpError::InvalidRequest(format!("rpc handler task failed: {join_err}"))
                        })?;
                        tool_response(req_id, outcome, "evaluate failed", "js_error")
                    }

                    "browser_type" => {
                        let adapter = adapter.clone();
                        let params = req.params;
                        let rt_handle = tokio::runtime::Handle::current();
                        let outcome = tokio::task::spawn_blocking(move || {
                            rt_handle.block_on(async move {
                                tools::type_text::handle_type(&adapter, params).await
                            })
                        })
                        .await
                        .map_err(|join_err| {
                            McpError::InvalidRequest(format!("rpc handler task failed: {join_err}"))
                        })?;
                        tool_response(req_id, outcome, "type failed", "type_error")
                    }

                    "browser_press_key" => {
                        let adapter = adapter.clone();
                        let params = req.params;
                        let rt_handle = tokio::runtime::Handle::current();
                        let outcome = tokio::task::spawn_blocking(move || {
                            rt_handle.block_on(async move {
                                tools::press_key::handle_press_key(&adapter, params).await
                            })
                        })
                        .await
                        .map_err(|join_err| {
                            McpError::InvalidRequest(format!("rpc handler task failed: {join_err}"))
                        })?;
                        tool_response(req_id, outcome, "press key failed", "press_key_error")
                    }

                    "browser_new_tab" => {
                        let adapter = adapter.clone();
                        let params = req.params;
                        let rt_handle = tokio::runtime::Handle::current();
                        let outcome = tokio::task::spawn_blocking(move || {
                            rt_handle.block_on(async move {
                                tools::tabs::handle_new_tab(&adapter, params).await
                            })
                        })
                        .await
                        .map_err(|join_err| {
                            McpError::InvalidRequest(format!("rpc handler task failed: {join_err}"))
                        })?;
                        tool_response(req_id, outcome, "new tab failed", "tab_error")
                    }

                    "browser_switch_tab" => {
                        let adapter = adapter.clone();
                        let params = req.params;
                        let rt_handle = tokio::runtime::Handle::current();
                        let outcome = tokio::task::spawn_blocking(move || {
                            rt_handle.block_on(async move {
                                tools::tabs::handle_switch_tab(&adapter, params).await
                            })
                        })
                        .await
                        .map_err(|join_err| {
                            McpError::InvalidRequest(format!("rpc handler task failed: {join_err}"))
                        })?;
                        tool_response(req_id, outcome, "switch tab failed", "tab_error")
                    }

                    "browser_list_tabs" => {
                        let adapter = adapter.clone();
                        let params = req.params;
                        let rt_handle = tokio::runtime::Handle::current();
                        let outcome = tokio::task::spawn_blocking(move || {
                            rt_handle.block_on(async move {
                                tools::tabs::handle_list_tabs(&adapter, params).await
                            })
                        })
                        .await
                        .map_err(|join_err| {
                            McpError::InvalidRequest(format!("rpc handler task failed: {join_err}"))
                        })?;
                        tool_response(req_id, outcome, "list tabs failed", "tab_error")
                    }

                    "browser_close_tab" => {
                        let adapter = adapter.clone();
                        let params = req.params;
                        let rt_handle = tokio::runtime::Handle::current();
                        let outcome = tokio::task::spawn_blocking(move || {
                            rt_handle.block_on(async move {
                                tools::tabs::handle_close_tab(&adapter, params).await
                            })
                        })
                        .await
                        .map_err(|join_err| {
                            McpError::InvalidRequest(format!("rpc handler task failed: {join_err}"))
                        })?;
                        tool_response(req_id, outcome, "close tab failed", "tab_error")
                    }

                    "browser_get_cookies" => {
                        let adapter = adapter.clone();
                        let params = req.params;
                        let rt_handle = tokio::runtime::Handle::current();
                        let outcome = tokio::task::spawn_blocking(move || {
                            rt_handle.block_on(async move {
                                tools::cookies::handle_get_cookies(&adapter, params).await
                            })
                        })
                        .await
                        .map_err(|join_err| {
                            McpError::InvalidRequest(format!("rpc handler task failed: {join_err}"))
                        })?;
                        tool_response(req_id, outcome, "get cookies failed", "cookie_error")
                    }

                    "browser_set_cookie" => {
                        let adapter = adapter.clone();
                        let params = req.params;
                        let rt_handle = tokio::runtime::Handle::current();
                        let outcome = tokio::task::spawn_blocking(move || {
                            rt_handle.block_on(async move {
                                tools::cookies::handle_set_cookie(&adapter, params).await
                            })
                        })
                        .await
                        .map_err(|join_err| {
                            McpError::InvalidRequest(format!("rpc handler task failed: {join_err}"))
                        })?;
                        tool_response(req_id, outcome, "set cookie failed", "cookie_error")
                    }

                    "browser_clear_cookies" => {
                        let adapter = adapter.clone();
                        let params = req.params;
                        let rt_handle = tokio::runtime::Handle::current();
                        let outcome = tokio::task::spawn_blocking(move || {
                            rt_handle.block_on(async move {
                                tools::cookies::handle_clear_cookies(&adapter, params).await
                            })
                        })
                        .await
                        .map_err(|join_err| {
                            McpError::InvalidRequest(format!("rpc handler task failed: {join_err}"))
                        })?;
                        tool_response(req_id, outcome, "clear cookies failed", "cookie_error")
                    }

                    "browser_session_snapshot" => {
                        let adapter = adapter.clone();
                        let params = req.params;
                        let rt_handle = tokio::runtime::Handle::current();
                        let outcome = tokio::task::spawn_blocking(move || {
                            rt_handle.block_on(async move {
                                tools::snapshot::handle_session_snapshot(&adapter, params).await
                            })
                        })
                        .await
                        .map_err(|join_err| {
                            McpError::InvalidRequest(format!("rpc handler task failed: {join_err}"))
                        })?;
                        tool_response(req_id, outcome, "session snapshot failed", "snapshot_error")
                    }

                    "browser_session_clone" => {
                        let adapter = adapter.clone();
                        let params = req.params;
                        let rt_handle = tokio::runtime::Handle::current();
                        let outcome = tokio::task::spawn_blocking(move || {
                            rt_handle.block_on(async move {
                                tools::clone_session::handle_session_clone(&adapter, params).await
                            })
                        })
                        .await
                        .map_err(|join_err| {
                            McpError::InvalidRequest(format!("rpc handler task failed: {join_err}"))
                        })?;
                        tool_response(req_id, outcome, "session clone failed", "clone_error")
                    }

                    _ => JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req_id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32601,
                            message: format!("method not found: {}", req.method),
                        }),
                    },
                }
            }
        };

        Ok(resp)
    }
}

async fn rpc_endpoint(
    State(server): State<McpServer>,
    headers: HeaderMap,
    body: String,
) -> (StatusCode, Json<Value>) {
    let provided_key = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let req = match McpServer::parse_request(&body) {
        Ok(req) => req,
        Err(err) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(
                    json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32600, "message": err.to_string() } }),
                ),
            );
        }
    };

    let handled = server
        .handle_request(&server.adapter, req, provided_key.as_deref())
        .await;

    match handled {
        Ok(resp) => (
            StatusCode::OK,
            Json(serde_json::to_value(resp).unwrap_or_else(|_| {
                json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": { "code": -32603, "message": "internal serialization error" }
                })
            })),
        ),
        Err(McpError::Unauthorized) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": { "code": -32001, "message": "unauthorized" }
            })),
        ),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": { "code": -32600, "message": err.to_string() }
            })),
        ),
    }
}

async fn handle_metrics() -> impl IntoResponse {
    (
        StatusCode::OK,
        [("Content-Type", "text/plain; version=0.0.4; charset=utf-8")],
        metrics::metrics_text(),
    )
}

fn breaker_state_label(state: usize) -> &'static str {
    match state {
        0 => "closed",
        1 => "open",
        2 => "half_open",
        _ => "unknown",
    }
}

async fn handle_health(State(server): State<McpServer>) -> impl IntoResponse {
    let active = server.adapter.session_count();
    let utilization_pct = ((active as f64 / SESSION_LIMIT as f64) * 1000.0).round() / 10.0;
    let tier1_state = breaker_state_label(server.adapter.tier1.circuit_breaker_state());
    let tier2_state = breaker_state_label(server.adapter.tier2.circuit_breaker_state());
    let any_open = tier1_state == "open" || tier2_state == "open";
    let any_half_open = tier1_state == "half_open" || tier2_state == "half_open";
    let status = if utilization_pct > 95.0 || any_open {
        "unhealthy"
    } else if utilization_pct >= 80.0 || any_half_open {
        "degraded"
    } else {
        "healthy"
    };

    axum::Json(serde_json::json!({
        "status": status,
        "version": env!("CARGO_PKG_VERSION"),
        "sessions": {
            "active": active,
            "limit": SESSION_LIMIT,
            "utilization_pct": utilization_pct,
        },
        "pools": {
            "tier1": {
                "available": server.adapter.tier1.available(),
                "circuit_breaker": tier1_state,
            },
            "tier2": {
                "available": server.adapter.tier2.available(),
                "circuit_breaker": tier2_state,
            },
        },
        "storage": {
            "quota_used_bytes": metrics::STORAGE_QUOTA_USED_BYTES.get(),
        },
    }))
}

async fn require_api_key(
    State(server): State<McpServer>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let Some(expected_key) = server.api_key.as_deref() else {
        return Ok(next.run(request).await);
    };

    let provided_key = request
        .headers()
        .get("x-api-key")
        .or_else(|| request.headers().get("X-API-Key"))
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if provided_key != expected_key {
        let digest = Sha256::digest(provided_key.as_bytes());
        let key_hash = hex::encode(&digest[..4]);
        tracing::warn!(
            key_hash = %key_hash,
            "Rejected: invalid API key (key_hash={})",
            key_hash
        );
        return Err(StatusCode::UNAUTHORIZED);
    }

    if server.rate_limiter.is_rate_limited(provided_key) {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(request).await)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::{McpError, McpServer};

    #[test]
    fn parse_rejects_wrong_jsonrpc_version() {
        let raw = r#"{"jsonrpc":"1.0","id":1,"method":"ping","params":{}}"#;
        let err = McpServer::parse_request(raw).unwrap_err();
        assert!(matches!(err, McpError::InvalidRequest(_)));
    }

    #[tokio::test]
    async fn handle_ping_returns_success() {
        use serde_json::json;
        let adapter = crate::engine::get_test_adapter().await;
        let server = McpServer::new_with_adapter(None, adapter.clone());
        let req =
            McpServer::parse_request(r#"{"jsonrpc":"2.0","id":"a","method":"ping","params":{}}"#)
                .unwrap();
        let resp = server.handle_request(&adapter, req, None).await.unwrap();
        assert_eq!(resp.result, Some(json!({ "ok": true, "pong": true })));
        assert!(resp.error.is_none());
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        use axum::body::Body;
        use tower::util::ServiceExt;

        let adapter = crate::engine::get_test_adapter().await;
        let server = McpServer::new_with_adapter(None, adapter);
        let request = axum::http::Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();
        let resp = server
            .router()
            .oneshot(request)
            .await
            .expect("health route should respond");
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_metrics_endpoint() {
        use axum::response::IntoResponse;
        crate::metrics::SESSIONS_ACTIVE.set(1);
        let resp = super::handle_metrics().await.into_response();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);

        let body_bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let body_str = String::from_utf8_lossy(&body_bytes);
        assert!(body_str.contains("sessions_active"));
    }

    #[tokio::test]
    async fn api_key_is_enforced() {
        let adapter = crate::engine::get_test_adapter().await;
        let server = McpServer::new_with_adapter(Some("secret".to_string()), adapter.clone());
        let req =
            McpServer::parse_request(r#"{"jsonrpc":"2.0","id":"a","method":"ping","params":{}}"#)
                .unwrap();
        let err = server
            .handle_request(&adapter, req, Some("wrong"))
            .await
            .unwrap_err();
        assert!(matches!(err, McpError::Unauthorized));
    }
}
