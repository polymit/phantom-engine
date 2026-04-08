use axum::http::StatusCode;
use serde_json::{json, Value};

use crate::engine::EngineAdapter;

pub async fn handle_get_cookies(
    adapter: &EngineAdapter,
    _params: Value,
) -> Result<Value, (StatusCode, Value)> {
    let store = adapter.cookie_store.lock().await;

    let mut cookies = Vec::new();
    for c in store.iter_unexpired() {
        cookies.push(json!({
            "name": c.name(),
            "value": c.value(),
            "domain": c.domain().unwrap_or(""),
            "path": c.path().unwrap_or("/")
        }));
    }

    Ok(json!({ "cookies": cookies }))
}

pub async fn handle_set_cookie(
    adapter: &EngineAdapter,
    params: Value,
) -> Result<Value, (StatusCode, Value)> {
    let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let value = params.get("value").and_then(|v| v.as_str()).unwrap_or("");
    let domain = params.get("domain").and_then(|v| v.as_str()).unwrap_or("");
    let path = params.get("path").and_then(|v| v.as_str()).unwrap_or("/");

    if name.is_empty() || value.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            json!({ "error": { "code": "invalid_params", "message": "name and value are required" } }),
        ));
    }

    let header_str = format!("{}={}; Domain={}; Path={}", name, value, domain, path);

    let url_str = adapter.get_page_url().unwrap_or_else(|| {
        if domain.starts_with('.') {
            format!("https://www{}", domain)
        } else {
            format!("https://{}", domain)
        }
    });

    let url = url::Url::parse(&url_str).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            json!({ "error": { "code": "invalid_url", "message": format!("invalid url: {}", e) } }),
        )
    })?;

    let mut store = adapter.cookie_store.lock().await;

    // Use insert_raw instead of parse according to docs, wait - the prompt says "store.parse(header_str, &url)"
    // Wait, cookie_store::CookieStore has `parse` which handles Set-Cookie parsing.
    store.parse(&header_str, &url).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "error": { "code": "cookie_error", "message": "failed to parse cookie" } }),
        )
    })?;

    Ok(json!({ "set": true }))
}

pub async fn handle_clear_cookies(
    adapter: &EngineAdapter,
    _params: Value,
) -> Result<Value, (StatusCode, Value)> {
    let mut store = adapter.cookie_store.lock().await;
    store.clear();
    Ok(json!({ "cleared": true }))
}
