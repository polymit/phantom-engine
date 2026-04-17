use axum::http::StatusCode;
use phantom_serializer::{HeadlessSerializer, SerialiserConfig, SerialiserMode};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::engine::EngineAdapter;
use crate::metrics;
use tracing::Instrument;

#[derive(Debug, Deserialize)]
pub struct SceneGraphParams {
    pub mode: Option<String>,
    pub task_hint: Option<String>,
    pub scroll_x: Option<f32>,
    pub scroll_y: Option<f32>,
}

/// Handle the `browser_get_scene_graph` JSON-RPC tool call.
///
/// Re-serialises the stored ParsedPage into CCT v0.2 with the caller's
/// specified mode, scroll offset, and task hint. Requires a prior
/// `browser_navigate` call — returns `no_page_loaded` otherwise.
pub async fn handle_get_scene_graph(
    adapter: &EngineAdapter,
    params: Value,
) -> Result<Value, (StatusCode, Value)> {
    let span = tracing::info_span!(
        "tool.scene_graph",
        mode = tracing::field::Empty,
        task_hint = tracing::field::Empty,
        node_count = tracing::field::Empty,
        cct_bytes = tracing::field::Empty,
        elapsed_ms = tracing::field::Empty
    );
    async move {
        let params: SceneGraphParams = serde_json::from_value(params).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                json!({ "error": { "code": "invalid_params", "message": e.to_string() } }),
            )
        })?;

        let (page, url, viewport_width, viewport_height) =
            adapter.get_page_with_viewport().await.ok_or_else(|| {
                (
                    StatusCode::CONFLICT,
                    json!({ "error": {
                        "code": "no_page_loaded",
                        "message": "Call browser_navigate before browser_get_scene_graph"
                    }}),
                )
            })?;

        let mode = match params.mode.as_deref() {
            Some("selective") => SerialiserMode::Selective,
            _ => SerialiserMode::Full,
        };

        let start = std::time::Instant::now();

        // Record task_hint before it moves into config
        if let Some(hint) = &params.task_hint {
            tracing::Span::current().record("task_hint", hint.as_str());
        }

        let config = SerialiserConfig {
            url: url.clone(),
            scroll_x: params.scroll_x.unwrap_or(0.0),
            scroll_y: params.scroll_y.unwrap_or(0.0),
            viewport_width,
            viewport_height,
            total_height: viewport_height,
            mode: mode.clone(),
            task_hint: params.task_hint,
        };

        let cct = HeadlessSerializer::serialise(&page, &config);

        let node_count = cct.lines().filter(|l| !l.starts_with("##")).count();

        let mode_str = match mode {
            SerialiserMode::Full => "full",
            SerialiserMode::Selective => "selective",
        };

        let elapsed = start.elapsed();
        metrics::DOM_SNAPSHOT_DURATION_SECONDS.observe(elapsed.as_secs_f64());
        metrics::DOM_NODES_SERIALISED.observe(node_count as f64);
        if let Err(err) = adapter.enforce_budget_usage(cct.len(), elapsed.as_millis() as u64, 0) {
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                json!({ "error": { "code": "budget_exceeded", "message": err.to_string() } }),
            ));
        }

        tracing::Span::current().record("mode", mode_str);
        tracing::Span::current().record("node_count", node_count as u64);
        tracing::Span::current().record("cct_bytes", cct.len() as u64);
        tracing::Span::current().record("elapsed_ms", elapsed.as_millis() as u64);

        Ok(json!({
            "cct":        cct,
            "node_count": node_count,
            "mode":       mode_str,
            "url":        url,
        }))
    }
    .instrument(span)
    .await
}
