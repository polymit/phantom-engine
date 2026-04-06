use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
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

#[derive(Debug, Clone, Default)]
pub struct McpServer {
    api_key: Option<String>,
}

impl McpServer {
    pub fn new(api_key: Option<String>) -> Self {
        Self { api_key }
    }

    pub fn router(self) -> Router {
        Router::new()
            .route("/rpc", post(rpc_endpoint))
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

    pub fn handle_request(
        &self,
        req: JsonRpcRequest,
        provided_key: Option<&str>,
    ) -> Result<JsonRpcResponse, McpError> {
        if let Some(expected) = &self.api_key {
            let actual = provided_key.unwrap_or_default();
            if actual != expected {
                return Err(McpError::Unauthorized);
            }
        }

        let resp = match req.method.as_str() {
            "ping" => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: Some(json!({ "ok": true, "pong": true })),
                error: None,
            },
            _ => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
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
    let maybe_key = headers.get("x-api-key").and_then(|v| v.to_str().ok());
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

    match server.handle_request(req, maybe_key) {
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
    use serde_json::json;

    use super::{McpError, McpServer};

    #[test]
    fn parse_rejects_wrong_jsonrpc_version() {
        let raw = r#"{"jsonrpc":"1.0","id":1,"method":"ping","params":{}}"#;
        let err = McpServer::parse_request(raw).unwrap_err();
        assert!(matches!(err, McpError::InvalidRequest(_)));
    }

    #[test]
    fn handle_ping_returns_success() {
        let server = McpServer::new(None);
        let req =
            McpServer::parse_request(r#"{"jsonrpc":"2.0","id":"a","method":"ping","params":{}}"#)
                .unwrap();
        let resp = server.handle_request(req, None).unwrap();
        assert_eq!(resp.result, Some(json!({ "ok": true, "pong": true })));
        assert!(resp.error.is_none());
    }

    #[test]
    fn api_key_is_enforced() {
        let server = McpServer::new(Some("secret".to_string()));
        let req =
            McpServer::parse_request(r#"{"jsonrpc":"2.0","id":"a","method":"ping","params":{}}"#)
                .unwrap();
        let err = server.handle_request(req, Some("wrong")).unwrap_err();
        assert!(matches!(err, McpError::Unauthorized));
    }
}
