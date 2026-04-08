use axum::http::StatusCode;
use serde_json::{json, Value};
use crate::engine::EngineAdapter;

pub async fn handle_press_key(
    adapter: &EngineAdapter,
    params: Value,
) -> Result<Value, (StatusCode, Value)> {
    let key = params.get("key").and_then(|v| v.as_str());
    
    if key.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            json!({ "error": { "code": "invalid_params", "message": "key is required" } }),
        ));
    }
    
    let store = adapter.page_store.lock();
    if store.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            json!({ "error": { "code": "no_page_loaded", "message": "no page loaded" } }),
        ));
    }
    
    // Simulate pressing key
    Ok(json!({ "pressed": true, "key": key.unwrap() }))
}
