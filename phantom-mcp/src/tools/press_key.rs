use crate::engine::EngineAdapter;
use axum::http::StatusCode;
use phantom_serializer::CctDelta;
use serde_json::{json, Value};

use super::escape_js_single_quoted;

#[derive(Debug, serde::Deserialize)]
struct PressKeyParams {
    key: String,
}

pub async fn handle_press_key(
    adapter: &EngineAdapter,
    params: Value,
) -> Result<Value, (StatusCode, Value)> {
    let p: PressKeyParams = serde_json::from_value(params).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            json!({ "error": { "code": "invalid_params", "message": e.to_string() } }),
        )
    })?;
    let key = p.key.trim().to_string();
    if key.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            json!({ "error": { "code": "invalid_params", "message": "key is required" } }),
        ));
    }

    let tree = {
        let page = adapter.get_page().ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                json!({ "error": { "code": "no_page_loaded", "message": "no page loaded" } }),
            )
        })?;
        page.tree.clone()
    };
    let delta_node_id = tree.document_root;

    let mut session = adapter.tier1.acquire().await.map_err(|_| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            json!({ "error": { "code": "session_pool_exhausted", "message": "tier1 pool exhausted" } }),
        )
    })?;
    session.attach_dom(tree).await;

    let safe_key = escape_js_single_quoted(&key);
    let script = format!(
        "(() => {{
            const __target = document.activeElement || document.body || document.documentElement || document;
            const __key = '{key}';
            if (__target && typeof __target.dispatchEvent === 'function' && typeof KeyboardEvent === 'function') {{
                __target.dispatchEvent(new KeyboardEvent('keydown', {{ bubbles: true, key: __key }}));
                __target.dispatchEvent(new KeyboardEvent('keypress', {{ bubbles: true, key: __key }}));
                __target.dispatchEvent(new KeyboardEvent('keyup', {{ bubbles: true, key: __key }}));
            }}
            return 'ok';
        }})()",
        key = safe_key
    );
    if let Err(e) = session.eval(&script).await {
        adapter.tier1.release_after_use(session);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "error": { "code": "js_error", "message": e.to_string() } }),
        ));
    }

    adapter.tier1.release_after_use(session);
    if let Some(node_id) = delta_node_id {
        adapter.inject_cct_delta(CctDelta::Update {
            node_id,
            display: None,
            bounds: None,
        });
    }
    Ok(json!({ "pressed": true, "key": key }))
}
