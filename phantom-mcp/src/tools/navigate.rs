use axum::http::StatusCode;
use phantom_serializer::CctDelta;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::engine::{EngineAdapter, SessionPage};
use crate::metrics;
use phantom_net::navigate::{navigate, NavigationConfig, NavigationError};
use tracing::Instrument;

#[derive(Debug, Deserialize)]
pub struct NavigateParams {
    pub url: String,
    pub viewport_width: Option<f32>,
    pub viewport_height: Option<f32>,
    pub task_hint: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct NavigateResult {
    pub url: String,
    pub status: u16,
    pub cct: String,
    pub node_count: usize,
}

/// Handle the `browser_navigate` JSON-RPC tool call.
///
/// Called by the JSON-RPC dispatcher when `method == "browser_navigate"`.
/// Returns a serialised [`NavigateResult`] on success, or a structured
/// error value on failure. The caller is responsible for wrapping this
/// in a [`JsonRpcResponse`].
pub async fn handle_navigate(
    adapter: &EngineAdapter,
    params: Value,
) -> Result<Value, (StatusCode, Value)> {
    let span = tracing::info_span!(
        "tool.navigate",
        url = tracing::field::Empty,
        status = tracing::field::Empty,
        elapsed_ms = tracing::field::Empty,
        html_bytes = tracing::field::Empty
    );
    async move {
        let expected_key = adapter.current_page_key();
        let params: NavigateParams = serde_json::from_value(params).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                json!({ "error": { "code": "invalid_params", "message": e.to_string() } }),
            )
        })?;

        tracing::Span::current().record("url", params.url.as_str());
        let start = std::time::Instant::now();

        let budget = adapter
            .broker
            .get(adapter.session_uuid)
            .map(|s| s.budget)
            .unwrap_or_default();

        let config = NavigationConfig {
            viewport_width: params.viewport_width.unwrap_or(1280.0),
            viewport_height: params.viewport_height.unwrap_or(720.0),
            task_hint: params.task_hint,
            max_network_bytes: Some(budget.max_network_bytes),
            ..Default::default()
        };

        let result = navigate(&adapter.network, &params.url, &config)
            .await
            .map_err(|e| {
                let (code, http_status) = match &e {
                    NavigationError::HttpError { status, .. } => {
                        (format!("http_error_{}", status), StatusCode::BAD_GATEWAY)
                    }
                    NavigationError::Network { .. } => {
                        ("network_error".to_string(), StatusCode::BAD_GATEWAY)
                    }
                    NavigationError::Encoding { .. } => (
                        "encoding_error".to_string(),
                        StatusCode::UNPROCESSABLE_ENTITY,
                    ),
                    NavigationError::Pipeline { .. } => (
                        "pipeline_error".to_string(),
                        StatusCode::INTERNAL_SERVER_ERROR,
                    ),
                    NavigationError::RedirectResponse { status, .. } => (
                        format!("redirect_response_{}", status),
                        StatusCode::BAD_GATEWAY,
                    ),
                    NavigationError::AllAttemptsFailed { .. } => {
                        ("all_attempts_failed".to_string(), StatusCode::BAD_GATEWAY)
                    }
                };
                (
                    http_status,
                    json!({ "error": { "code": code, "message": e.to_string() } }),
                )
            })?;

        let response_url = result.url.clone();
        let response_status = result.status;
        let response_cct = result.cct.clone();
        let response_node_count = result.node_count;
        let delta_root = result.tree.document_root;

        tracing::Span::current().record("status", response_status);
        let elapsed = start.elapsed();
        tracing::Span::current().record("elapsed_ms", elapsed.as_millis() as u64);
        tracing::Span::current().record("html_bytes", response_cct.len() as u64);

        metrics::HTTP_REQUESTS_TOTAL
            .with_label_values(&[&response_status.to_string()])
            .inc();
        metrics::HTTP_REQUEST_DURATION_SECONDS.observe(elapsed.as_secs_f64());

        // Only charge the CPU-bound pipeline cost (parse + layout + serialise)
        // against the session budget — NOT the network I/O wait time.
        let cpu_ms = result.pipeline_ms.unwrap_or(0);
        if let Err(err) = adapter.enforce_budget_usage(
            response_cct.len(),
            cpu_ms,
            response_cct.len(),
        ) {
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                json!({ "error": { "code": "budget_exceeded", "message": err.to_string() } }),
            ));
        }

        // Persist the parsed page so browser_get_scene_graph can re-serialise
        // with different scroll/mode parameters without re-fetching.
        let stored = adapter.store_page_if_current(
            expected_key,
            SessionPage::with_viewport(
                result.tree,
                result.url.clone(),
                result.status,
                config.viewport_width,
                config.viewport_height,
            ),
        );
        if !stored {
            tracing::debug!("navigate result dropped because active tab changed during navigation");
        }
        if let Some(node_id) = delta_root {
            adapter.inject_cct_delta(CctDelta::Update {
                node_id,
                display: None,
                bounds: None,
            });
        }

        Ok(json!({
            "url":        response_url,
            "status":     response_status,
            "cct":        response_cct,
            "node_count": response_node_count,
        }))
    }
    .instrument(span)
    .await
}
