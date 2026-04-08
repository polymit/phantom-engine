use crate::engine::EngineAdapter;
use axum::http::StatusCode;
use serde_json::{json, Value};

pub async fn handle_type(
    adapter: &EngineAdapter,
    params: Value,
) -> Result<Value, (StatusCode, Value)> {
    let selector = params.get("selector").and_then(|v| v.as_str());
    let text = params.get("text").and_then(|v| v.as_str());

    if selector.is_none() || text.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            json!({ "error": { "code": "invalid_params", "message": "selector and text are required" } }),
        ));
    }

    let store = adapter.page_store.lock();
    if store.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            json!({ "error": { "code": "no_page_loaded", "message": "no page loaded" } }),
        ));
    }

    // Simulate typing
    Ok(json!({ "typed": true, "characters": text.unwrap().chars().count() }))
}
