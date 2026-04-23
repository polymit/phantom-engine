use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Method, Request, Response};
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use serde_json::{json, Value};

use crate::errors::CliError;

/// Coordinates for reaching the MCP server. Loaded from env or CLI flags.
pub struct ServerConfig {
    pub base_url: String,
    pub api_key: Option<String>,
}

impl ServerConfig {
    /// Build config from environment, with optional CLI overrides.
    pub fn resolve(server_override: Option<&str>, key_override: Option<&str>) -> Self {
        let bind_addr = server_override
            .map(String::from)
            .or_else(|| std::env::var("PHANTOM_BIND_ADDR").ok())
            .unwrap_or_else(|| "127.0.0.1:8080".to_string());

        let api_key = key_override
            .map(String::from)
            .or_else(|| std::env::var("PHANTOM_API_KEY").ok());

        Self {
            base_url: format!("http://{}", bind_addr),
            api_key,
        }
    }
}

type HyperClient = Client<hyper_util::client::legacy::connect::HttpConnector, Full<Bytes>>;

fn build_client() -> HyperClient {
    Client::builder(TokioExecutor::new()).build_http()
}

/// Fire a JSON-RPC 2.0 call and return the `result` field on success.
pub async fn rpc_call(
    config: &ServerConfig,
    method: &str,
    params: Value,
) -> Result<Value, CliError> {
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
    });

    let body_bytes =
        serde_json::to_vec(&payload).map_err(|e| CliError::Serialization(e.to_string()))?;

    let mut builder = Request::builder()
        .method(Method::POST)
        .uri(format!("{}/rpc", config.base_url))
        .header("content-type", "application/json");

    if let Some(key) = &config.api_key {
        builder = builder.header("x-api-key", key.as_str());
    }

    let req = builder
        .body(Full::new(Bytes::from(body_bytes)))
        .map_err(|e| CliError::Http {
            status: 0,
            detail: e.to_string(),
        })?;

    let resp: Response<Incoming> =
        build_client()
            .request(req)
            .await
            .map_err(|_| CliError::Connection {
                addr: config.base_url.clone(),
            })?;

    let status = resp.status().as_u16();
    let body = resp
        .collect()
        .await
        .map_err(|e| CliError::Http {
            status,
            detail: e.to_string(),
        })?
        .to_bytes();

    let json_resp: Value = serde_json::from_slice(&body).map_err(|e| CliError::Http {
        status,
        detail: format!("invalid JSON: {}", e),
    })?;

    if let Some(err) = json_resp.get("error") {
        let message = err
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown server error");
        return Err(CliError::Rpc(message.to_string()));
    }

    json_resp
        .get("result")
        .cloned()
        .ok_or_else(|| CliError::Rpc("response missing 'result' field".to_string()))
}

/// GET a plain endpoint (health, metrics) and return the body as a string.
pub async fn http_get(config: &ServerConfig, path: &str) -> Result<String, CliError> {
    let mut builder = Request::builder()
        .method(Method::GET)
        .uri(format!("{}{}", config.base_url, path));

    if let Some(key) = &config.api_key {
        builder = builder.header("x-api-key", key.as_str());
    }

    let req = builder
        .body(Full::new(Bytes::new()))
        .map_err(|e| CliError::Http {
            status: 0,
            detail: e.to_string(),
        })?;

    let resp = build_client()
        .request(req)
        .await
        .map_err(|_| CliError::Connection {
            addr: config.base_url.clone(),
        })?;

    let status = resp.status().as_u16();
    let body = resp
        .collect()
        .await
        .map_err(|e| CliError::Http {
            status,
            detail: e.to_string(),
        })?
        .to_bytes();

    String::from_utf8(body.to_vec()).map_err(|e| CliError::Http {
        status,
        detail: format!("non-UTF8 response: {}", e),
    })
}
