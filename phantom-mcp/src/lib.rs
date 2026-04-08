pub mod engine;
pub mod tools;

pub use engine::EngineAdapter;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

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
    pub adapter: EngineAdapter,
}

impl McpServer {
    pub fn new_with_adapter(api_key: Option<String>, adapter: EngineAdapter) -> Self {
        Self { api_key, adapter }
    }

    /// Build an axum `Router` that serves the JSON-RPC endpoint.
    pub fn router(self) -> Router {
        Router::new()
            .route("/rpc", post(rpc_endpoint))
            .route("/sse", get(crate::tools::subscribe::sse_handler))
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

            "browser_navigate" => {
                let outcome = tools::navigate::handle_navigate(adapter, req.params).await;
                tool_response(req_id, outcome, "navigation failed", "navigation_error")
            }

            "browser_get_scene_graph" => {
                let outcome = tools::scene_graph::handle_get_scene_graph(adapter, req.params).await;
                tool_response(req_id, outcome, "scene graph failed", "scene_graph_error")
            }

            "browser_click" => {
                let outcome = tools::click::handle_click(adapter, req.params).await;
                tool_response(req_id, outcome, "click failed", "click_error")
            }

            "browser_evaluate" => {
                let outcome = tools::evaluate::handle_evaluate(adapter, req.params).await;
                tool_response(req_id, outcome, "evaluate failed", "js_error")
            }

            "browser_type" => {
                let outcome = tools::type_text::handle_type(adapter, req.params).await;
                tool_response(req_id, outcome, "type failed", "type_error")
            }

            "browser_press_key" => {
                let outcome = tools::press_key::handle_press_key(adapter, req.params).await;
                tool_response(req_id, outcome, "press key failed", "press_key_error")
            }

            "browser_new_tab" => {
                let outcome = tools::tabs::handle_new_tab(adapter, req.params).await;
                tool_response(req_id, outcome, "new tab failed", "tab_error")
            }

            "browser_switch_tab" => {
                let outcome = tools::tabs::handle_switch_tab(adapter, req.params).await;
                tool_response(req_id, outcome, "switch tab failed", "tab_error")
            }

            "browser_list_tabs" => {
                let outcome = tools::tabs::handle_list_tabs(adapter, req.params).await;
                tool_response(req_id, outcome, "list tabs failed", "tab_error")
            }

            "browser_close_tab" => {
                let outcome = tools::tabs::handle_close_tab(adapter, req.params).await;
                tool_response(req_id, outcome, "close tab failed", "tab_error")
            }

            "browser_get_cookies" => {
                let outcome = tools::cookies::handle_get_cookies(adapter, req.params).await;
                tool_response(req_id, outcome, "get cookies failed", "cookie_error")
            }

            "browser_set_cookie" => {
                let outcome = tools::cookies::handle_set_cookie(adapter, req.params).await;
                tool_response(req_id, outcome, "set cookie failed", "cookie_error")
            }

            "browser_clear_cookies" => {
                let outcome = tools::cookies::handle_clear_cookies(adapter, req.params).await;
                tool_response(req_id, outcome, "clear cookies failed", "cookie_error")
            }

            "browser_session_snapshot" => {
                let outcome = tools::snapshot::handle_session_snapshot(adapter, req.params).await;
                tool_response(req_id, outcome, "session snapshot failed", "snapshot_error")
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

    match server
        .handle_request(&server.adapter, req, provided_key.as_deref())
        .await
    {
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

#[cfg(test)]
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
        let resp = server.handle_request(adapter, req, None).await.unwrap();
        assert_eq!(resp.result, Some(json!({ "ok": true, "pong": true })));
        assert!(resp.error.is_none());
    }

    #[tokio::test]
    async fn api_key_is_enforced() {
        let adapter = crate::engine::get_test_adapter().await;
        let server = McpServer::new_with_adapter(Some("secret".to_string()), adapter.clone());
        let req =
            McpServer::parse_request(r#"{"jsonrpc":"2.0","id":"a","method":"ping","params":{}}"#)
                .unwrap();
        let err = server
            .handle_request(adapter, req, Some("wrong"))
            .await
            .unwrap_err();
        assert!(matches!(err, McpError::Unauthorized));
    }
}
