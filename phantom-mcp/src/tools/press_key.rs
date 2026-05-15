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

    let (tree, delta_node_id, current_sx, current_sy, v_height, t_height) = {
        let key_active = adapter.current_page_key();
        let page_data = {
            let store = adapter.page_store.lock();
            store.get(&key_active).cloned()
        }
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                json!({ "error": { "code": "no_page_loaded", "message": "no page loaded" } }),
            )
        })?;

        let pp = page_data.to_parsed_page().ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "error": { "code": "pipeline_error", "message": "failed to layout page" } }),
            )
        })?;

        (
            pp.tree.clone(),
            pp.tree.document_root,
            page_data.scroll_x,
            page_data.scroll_y,
            page_data.viewport_height,
            pp.total_height,
        )
    };

    // Keyboard-driven scroll logic
    let mut scroll_y_delta = 0.0;
    let mut target_scroll_y: Option<f32> = None;

    match key.as_str() {
        "PageDown" | " " => scroll_y_delta = v_height * 0.9,
        "PageUp" => scroll_y_delta = -v_height * 0.9,
        "ArrowDown" => scroll_y_delta = 100.0,
        "ArrowUp" => scroll_y_delta = -100.0,
        "Home" => target_scroll_y = Some(0.0),
        "End" => target_scroll_y = Some(t_height - v_height),
        _ => {}
    }

    if scroll_y_delta != 0.0 || target_scroll_y.is_some() {
        let new_sy = if let Some(target) = target_scroll_y {
            target
        } else {
            current_sy + scroll_y_delta
        };
        let clamped_sy = new_sy.clamp(0.0, (t_height - v_height).max(0.0));
        adapter.update_scroll(current_sx, clamped_sy);
        tracing::info!(key = %key, scroll_y = clamped_sy, "keyboard-driven scroll");
    }

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
