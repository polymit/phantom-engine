use axum::http::StatusCode;
use serde::Deserialize;
use serde_json::{json, Value};
use phantom_serializer::{HeadlessSerializer, SerialiserConfig, SerialiserMode};

use crate::engine::EngineAdapter;

#[derive(Debug, Deserialize)]
pub struct SceneGraphParams {
    pub mode:      Option<String>,
    pub task_hint: Option<String>,
    pub scroll_x:  Option<f32>,
    pub scroll_y:  Option<f32>,
}

/// Handle the `browser_get_scene_graph` JSON-RPC tool call.
///
/// Re-serialises the stored ParsedPage into CCT v0.2 with the caller's
/// specified mode, scroll offset, and task hint. Requires a prior
/// `browser_navigate` call — returns `no_page_loaded` otherwise.
pub async fn handle_get_scene_graph(
    adapter: &EngineAdapter,
    params:  Value,
) -> Result<Value, (StatusCode, Value)> {
    let params: SceneGraphParams = serde_json::from_value(params)
        .unwrap_or(SceneGraphParams {
            mode: None, task_hint: None,
            scroll_x: None, scroll_y: None,
        });

    let page = adapter.get_page()
        .ok_or_else(|| (
            StatusCode::CONFLICT,
            json!({ "error": {
                "code": "no_page_loaded",
                "message": "Call browser_navigate before browser_get_scene_graph"
            }}),
        ))?;

    let url = adapter.get_page_url().unwrap_or_default();

    let mode = match params.mode.as_deref() {
        Some("selective") => SerialiserMode::Selective,
        _                 => SerialiserMode::Full,
    };

    let config = SerialiserConfig {
        url:             url.clone(),
        scroll_x:        params.scroll_x.unwrap_or(0.0),
        scroll_y:        params.scroll_y.unwrap_or(0.0),
        viewport_width:  1280.0,
        viewport_height: 720.0,
        total_height:    720.0,
        mode:            mode.clone(),
        task_hint:       params.task_hint,
    };

    let cct = HeadlessSerializer::serialise(&page, &config);

    let node_count = cct.lines()
        .filter(|l| !l.starts_with("##"))
        .count();

    let mode_str = match mode {
        SerialiserMode::Full      => "full",
        SerialiserMode::Selective => "selective",
    };

    Ok(json!({
        "cct":        cct,
        "node_count": node_count,
        "mode":       mode_str,
        "url":        url,
    }))
}
