use crate::engine::EngineAdapter;
use axum::http::StatusCode;
use phantom_js::BehaviorEngine;
use phantom_serializer::CctDelta;
use serde_json::{json, Value};

use super::escape_js_single_quoted;

#[derive(Debug, serde::Deserialize)]
struct TypeParams {
    selector: String,
    text: String,
    delay_ms: Option<u64>,
}

pub async fn handle_type(
    adapter: &EngineAdapter,
    params: Value,
) -> Result<Value, (StatusCode, Value)> {
    let p: TypeParams = serde_json::from_value(params).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            json!({ "error": { "code": "invalid_params", "message": e.to_string() } }),
        )
    })?;

    if p.selector.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            json!({ "error": { "code": "invalid_params", "message": "selector is required" } }),
        ));
    }
    let TypeParams {
        selector,
        text,
        delay_ms,
    } = p;

    let page_key = adapter.current_page_key();
    let tree = {
        let store = adapter.page_store.lock();
        let page = store.get(&page_key).ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                json!({ "error": { "code": "no_page_loaded", "message": "no page loaded" } }),
            )
        })?;
        page.to_parsed_page().map(|p| p.tree).ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                json!({ "error": { "code": "no_page_loaded", "message": "no page loaded" } }),
            )
        })?
    };
    let delta_node_id = tree.query_selector(&selector).or(tree.document_root);

    let mut session = adapter.tier1.acquire().await.map_err(|_| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            json!({ "error": { "code": "session_pool_exhausted", "message": "tier1 pool exhausted" } }),
        )
    })?;
    session.attach_dom(tree).await;

    let safe_selector = escape_js_single_quoted(&selector);
    let focus_script = format!(
        "(() => {{
            const __el = document.querySelector('{selector}');
            if (!__el) return 'not_found';
            if (typeof __el.focus === 'function') __el.focus();
            return 'ok';
        }})()",
        selector = safe_selector
    );

    let focus_res = match session.eval(&focus_script).await {
        Ok(v) => v,
        Err(e) => {
            adapter.tier1.release_after_use(session);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "error": { "code": "js_error", "message": e.to_string() } }),
            ));
        }
    };
    if focus_res == "not_found" {
        adapter.tier1.release_after_use(session);
        return Err((
            StatusCode::BAD_REQUEST,
            json!({ "error": { "code": "element_not_found", "message": format!("element not found: '{}'", selector) } }),
        ));
    }

    let behavior = BehaviorEngine::new();
    let typing_delay_ms = delay_ms;

    for ch in text.chars() {
        let ch_str = ch.to_string();
        let safe_ch = escape_js_single_quoted(&ch_str);
        let type_script = format!(
            "(() => {{
                const __el = document.querySelector('{selector}');
                if (!__el) return 'not_found';
                const __ch = '{ch}';
                if (typeof __el.dispatchEvent === 'function' && typeof KeyboardEvent === 'function') {{
                    __el.dispatchEvent(new KeyboardEvent('keydown', {{ bubbles: true, key: __ch }}));
                    __el.dispatchEvent(new KeyboardEvent('keypress', {{ bubbles: true, key: __ch }}));
                }}
                if (typeof __el.value === 'string') {{
                    __el.value += __ch;
                }} else if (__el.isContentEditable) {{
                    __el.textContent = (__el.textContent || '') + __ch;
                }}
                if (typeof __el.dispatchEvent === 'function' && typeof Event === 'function') {{
                    __el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                }}
                if (typeof __el.dispatchEvent === 'function' && typeof KeyboardEvent === 'function') {{
                    __el.dispatchEvent(new KeyboardEvent('keyup', {{ bubbles: true, key: __ch }}));
                }}
                return 'ok';
            }})()",
            selector = safe_selector,
            ch = safe_ch
        );

        let type_res = match session.eval(&type_script).await {
            Ok(v) => v,
            Err(e) => {
                adapter.tier1.release_after_use(session);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({ "error": { "code": "js_error", "message": e.to_string() } }),
                ));
            }
        };

        if type_res == "not_found" {
            adapter.tier1.release_after_use(session);
            return Err((
                StatusCode::BAD_REQUEST,
                json!({ "error": { "code": "element_not_found", "message": format!("element not found: '{}'", selector) } }),
            ));
        }

        let delay = typing_delay_ms.unwrap_or_else(|| behavior.char_typing_delay_ms());
        if delay > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
        }
    }

    if let Some(updated_tree) = session
        .dom_handle
        .as_ref()
        .map(|dom| dom.inner.read().clone())
    {
        if let Some(stored_page) = adapter.page_store.lock().get_mut(&page_key) {
            stored_page.tree = updated_tree;
        }
    }

    adapter.tier1.release_after_use(session);
    if let Some(node_id) = delta_node_id {
        adapter.inject_cct_delta(CctDelta::Update {
            node_id,
            display: None,
            bounds: None,
        });
    }
    Ok(json!({ "typed": true, "characters": text.chars().count() }))
}
