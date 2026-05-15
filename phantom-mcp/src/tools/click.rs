use axum::http::StatusCode;
use phantom_core::layout::bounds::ViewportBounds;
use phantom_js::BehaviorEngine;
use phantom_serializer::CctDelta;
use serde_json::{json, Value};
use std::time::Duration;
use tracing::Instrument;

use super::escape_js_single_quoted;

#[derive(Debug, serde::Deserialize)]
pub struct ClickParams {
    pub selector: String,
    pub x: Option<f64>, // target x — defaults to element center
    pub y: Option<f64>, // target y — defaults to element center
}

pub async fn handle_click(
    adapter: &crate::engine::EngineAdapter,
    params: Value,
) -> Result<Value, (StatusCode, Value)> {
    let span = tracing::info_span!(
        "tool.click",
        selector = tracing::field::Empty,
        hesitation_ms = tracing::field::Empty,
        path_points = tracing::field::Empty
    );
    async move {
        let click_params: ClickParams = serde_json::from_value(params).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            json!({
                "error": {
                    "code": "invalid_params",
                    "message": format!("invalid click parameters: {}", e)
                }
            }),
        )
    })?;

    let selector = click_params.selector;
    tracing::Span::current().record("selector", selector.as_str());

    let (tree, default_x, default_y, target_node_id, scroll_x, scroll_y) = {
        let key = adapter.current_page_key();
        let page_data = {
            let store = adapter.page_store.lock();
            store.get(&key).cloned()
        }.ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                json!({
                    "error": {
                        "code": "no_page_loaded",
                        "message": "no page loaded to click"
                    }
                }),
            )
        })?;

        // Use to_parsed_page() to ensure we have the absolute layout map
        let page = page_data.to_parsed_page().ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({
                    "error": {
                        "code": "pipeline_error",
                        "message": "failed to rebuild page layout for click"
                    }
                }),
            )
        })?;

        let tree = page.tree.clone();
        let target_node_id = tree.query_selector(&selector).ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                json!({
                    "error": {
                        "code": "element_not_found",
                        "message": format!("element not found: '{}'", selector)
                    }
                }),
            )
        })?;

        let target_bounds = page
            .layout_map
            .get(&target_node_id)
            .cloned()
            .unwrap_or_else(ViewportBounds::zero);

        let mut current_sx = page_data.scroll_x;
        let mut current_sy = page_data.scroll_y;
        let v_width = page_data.viewport_width;
        let v_height = page_data.viewport_height;

        // Auto-scroll: if target center is off-screen, center it in the viewport
        let center_x = target_bounds.x + (target_bounds.width / 2.0);
        let center_y = target_bounds.y + (target_bounds.height / 2.0);

        let mut scroll_changed = false;
        if center_y < current_sy || center_y > (current_sy + v_height) {
            current_sy = (center_y - v_height / 2.0).max(0.0);
            scroll_changed = true;
        }
        if center_x < current_sx || center_x > (current_sx + v_width) {
            current_sx = (center_x - v_width / 2.0).max(0.0);
            scroll_changed = true;
        }

        if scroll_changed {
            adapter.update_scroll(current_sx, current_sy);
            tracing::info!(scroll_x = current_sx, scroll_y = current_sy, "auto-scrolled to element");
        }

        let default_x = if target_bounds.width > 0.0 {
            target_bounds.x as f64 + (target_bounds.width as f64 / 2.0)
        } else {
            640.0
        };
        let default_y = if target_bounds.height > 0.0 {
            target_bounds.y as f64 + (target_bounds.height as f64 / 2.0)
        } else {
            360.0
        };
        (tree, default_x, default_y, target_node_id, current_sx, current_sy)
    };

    let behavior = BehaviorEngine::new();
    let target_x = click_params.x.unwrap_or(default_x);
    let target_y = click_params.y.unwrap_or(default_y);

    // clientX/Y must be relative to the viewport origin
    let client_x = target_x - scroll_x as f64;
    let client_y = target_y - scroll_y as f64;

    let mouse_path = behavior.generate_mouse_path((0.0, 0.0), (client_x, client_y));
    tracing::Span::current().record("path_points", mouse_path.len() as u64);

    // Acquire session
    let mut session = adapter.tier1.acquire().await.map_err(|_| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            json!({
                "error": {
                    "code": "session_pool_exhausted",
                    "message": "tier1 pool exhausted"
                }
            }),
        )
    })?;

    session.attach_dom(tree).await;

    let safe_selector = escape_js_single_quoted(&selector);

    // 1-2. Dispatch mouse movement along the Bezier path
    let mut path_js = format!(
        "var __el = document.querySelector('{}'); var __events = [",
        safe_selector
    );
    for (px, py) in &mouse_path {
        path_js.push_str(&format!("[{},{}],", px, py));
    }
    path_js.push_str("];");
    path_js.push_str(
        "if (__el) {
            __events.forEach(function(p) {
                __el.dispatchEvent(new PointerEvent('pointermove', {bubbles:true,clientX:p[0],clientY:p[1]}));
                __el.dispatchEvent(new MouseEvent('mousemove', {bubbles:true,clientX:p[0],clientY:p[1]}));
            });
            'found'
        } else {
            'not_found'
        }"
    );

    let move_result = match session.eval(&path_js).await {
        Ok(v) => v,
        Err(e) => {
            adapter.tier1.release_after_use(session);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({
                    "error": {
                        "code": "js_error",
                        "message": format!("path dispatch failed: {}", e)
                    }
                }),
            ));
        }
    };

    if move_result == "not_found" {
        adapter.tier1.release_after_use(session);
        return Err((
            StatusCode::BAD_REQUEST,
            json!({
                "error": {
                    "code": "element_not_found",
                    "message": format!("element not found: '{}'", selector)
                }
            }),
        ));
    }

    // 3-8. Dispatch entry and down events
    let entry_down_js = format!(
        "__el.dispatchEvent(new MouseEvent('mouseenter',{{bubbles:false,clientX:{x},clientY:{y}}}));
         __el.dispatchEvent(new MouseEvent('mouseover',{{bubbles:true,clientX:{x},clientY:{y}}}));
         __el.dispatchEvent(new PointerEvent('pointerover',{{bubbles:true,clientX:{x},clientY:{y}}}));
         __el.dispatchEvent(new PointerEvent('pointerdown',{{bubbles:true,clientX:{x},clientY:{y}}}));
         __el.dispatchEvent(new MouseEvent('mousedown',{{bubbles:true,clientX:{x},clientY:{y}}}));
         __el.dispatchEvent(new FocusEvent('focus',{{bubbles:false}}));",
        x = client_x,
        y = client_y
    );

    if let Err(e) = session.eval(&entry_down_js).await {
        adapter.tier1.release_after_use(session);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({
                "error": {
                    "code": "js_error",
                    "message": format!("entry/down dispatch failed: {}", e)
                }
            }),
        ));
    }

    // 9. Hesitation delay
    let hesitation_ms = behavior.click_hesitation_ms();
    tracing::Span::current().record("hesitation_ms", hesitation_ms);
    tokio::time::sleep(Duration::from_millis(hesitation_ms)).await;

    // 10-12. Dispatch up and click events + Navigation Detection
    let up_click_js = format!(
        "__el.dispatchEvent(new PointerEvent('pointerup',{{bubbles:true,clientX:{x},clientY:{y}}}));
         __el.dispatchEvent(new MouseEvent('mouseup',{{bubbles:true,clientX:{x},clientY:{y}}}));
         __el.dispatchEvent(new MouseEvent('click',{{bubbles:true,clientX:{x},clientY:{y}}}));
         (function() {{
            var target = __el;
            while (target) {{
                if (target.tagName === 'A' && target.href) {{
                    return target.href;
                }}
                target = target.parentElement;
            }}
            return '';
         }})();",
        x = client_x,
        y = client_y
    );

    let nav_url = match session.eval(&up_click_js).await {
        Ok(v) => if v.is_empty() { None } else { Some(v) },
        Err(e) => {
            adapter.tier1.release_after_use(session);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({
                    "error": {
                        "code": "js_error",
                        "message": format!("up/click dispatch failed: {}", e)
                    }
                }),
            ));
        }
    };

    // Release session back to pool
    adapter.tier1.release_after_use(session);

    // Trigger navigation if a link was detected
    if let Some(url) = nav_url {
        tracing::info!(target_url = %url, "detected link click, triggering navigation");
        // Reuse the tool's own logic by calling into the navigate module or simulating the tool call
        // For simplicity and to avoid circular deps, we just return the URL and let the agent navigate,
        // OR we can trigger it here if we have access to the navigate tool.
        // The prompt specifically asked to "fix" it, which implies the engine should navigate.

        // We'll perform a minimal navigation here.
        let budget = adapter.broker.get(adapter.session_uuid).map(|s| s.budget).unwrap_or_default();
        let config = phantom_net::navigate::NavigationConfig {
            max_network_bytes: Some(budget.max_network_bytes),
            ..Default::default()
        };

        match phantom_net::navigate::navigate(&adapter.network, &url, &config).await {
            Ok(result) => {
                adapter.store_page(crate::engine::SessionPage::with_viewport(
                    result.tree,
                    result.url,
                    result.status,
                    1280.0,
                    720.0,
                ));
            }
            Err(e) => {
                return Err((
                    StatusCode::BAD_GATEWAY,
                    json!({
                        "error": {
                            "code": "navigation_failed",
                            "message": format!("failed to follow link: {}", e)
                        }
                    }),
                ));
            }
        }
    }

    adapter.inject_cct_delta(CctDelta::Update {
        node_id: target_node_id,
        display: None,
        bounds: None,
    });

    Ok(json!({
        "clicked": true,
        "selector": selector,
        "hesitation_ms": hesitation_ms,
        "x": target_x,
        "y": target_y,
        "scroll_x": scroll_x,
        "scroll_y": scroll_y
    }))
    }
    .instrument(span)
    .await
}
