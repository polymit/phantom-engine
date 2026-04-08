use axum::http::StatusCode;
use phantom_js::BehaviorEngine;
use serde_json::{json, Value};
use std::time::Duration;

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

    let tree = {
        let page = adapter.get_page().ok_or_else(|| {
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
        page.tree.clone()
    };

    let behavior = BehaviorEngine::new();
    let target_x = click_params.x.unwrap_or(640.0);
    let target_y = click_params.y.unwrap_or(360.0);
    let mouse_path = behavior.generate_mouse_path((0.0, 0.0), (target_x, target_y));

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

    let safe_selector = selector.replace('\'', "\\'");

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
    // We bundle these to minimize eval overhead
    let entry_down_js = format!(
        "__el.dispatchEvent(new MouseEvent('mouseenter',{{bubbles:false,clientX:{x},clientY:{y}}}));
         __el.dispatchEvent(new MouseEvent('mouseover',{{bubbles:true,clientX:{x},clientY:{y}}}));
         __el.dispatchEvent(new PointerEvent('pointerover',{{bubbles:true,clientX:{x},clientY:{y}}}));
         __el.dispatchEvent(new PointerEvent('pointerdown',{{bubbles:true,clientX:{x},clientY:{y}}}));
         __el.dispatchEvent(new MouseEvent('mousedown',{{bubbles:true,clientX:{x},clientY:{y}}}));
         __el.dispatchEvent(new FocusEvent('focus',{{bubbles:false}}));",
        x = target_x,
        y = target_y
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
    tokio::time::sleep(Duration::from_millis(hesitation_ms)).await;

    // 10-12. Dispatch up and click events
    let up_click_js = format!(
        "__el.dispatchEvent(new PointerEvent('pointerup',{{bubbles:true,clientX:{x},clientY:{y}}}));
         __el.dispatchEvent(new MouseEvent('mouseup',{{bubbles:true,clientX:{x},clientY:{y}}}));
         __el.dispatchEvent(new MouseEvent('click',{{bubbles:true,clientX:{x},clientY:{y}}}));",
        x = target_x,
        y = target_y
    );

    if let Err(e) = session.eval(&up_click_js).await {
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

    // Release session back to pool
    adapter.tier1.release_after_use(session);

    Ok(json!({
        "clicked": true,
        "selector": selector,
        "hesitation_ms": hesitation_ms,
        "x": target_x,
        "y": target_y
    }))
}
