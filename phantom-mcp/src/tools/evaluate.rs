use axum::http::StatusCode;
use serde_json::{json, Value};

use crate::engine::EngineAdapter;

#[derive(Debug, serde::Deserialize)]
struct EvaluateParams {
    pub script: String,
    pub timeout_ms: Option<u64>,
}

/// Map a `serde_json::Value` to the JSON type name string.
fn json_type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

pub async fn handle_evaluate(
    adapter: &EngineAdapter,
    params: Value,
) -> Result<Value, (StatusCode, Value)> {
    let p: EvaluateParams = serde_json::from_value(params).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            json!({ "error": { "code": "invalid_params", "message": e.to_string() } }),
        )
    })?;

    // timeout_ms is accepted in the schema but QuickJS enforces its own 10s hard
    // limit via the interrupt handler. We surface it in the error message if provided.
    let _timeout_ms = p.timeout_ms;

    let tree = {
        let page = adapter.get_page().await.ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                json!({ "error": { "code": "no_page_loaded", "message": "no page loaded" } }),
            )
        })?;
        page.tree.clone()
    };

    let mut session = adapter.tier1.acquire().await.map_err(|_| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            json!({ "error": { "code": "session_pool_exhausted", "message": "tier1 pool exhausted" } }),
        )
    })?;

    session.attach_dom(tree).await;

    let raw = match session.eval(&p.script).await {
        Ok(v) => v,
        Err(e) => {
            adapter.tier1.release_after_use(session);
            // The interrupt handler terminates scripts and the error surfaces
            // as JsEvaluation with "interrupted" in the message rather than
            // a distinct JsTimeout variant from this eval path.
            let code = if e.to_string().to_lowercase().contains("interrupt") {
                "js_timeout"
            } else {
                "js_error"
            };
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "error": { "code": code, "message": e.to_string() } }),
            ));
        }
    };

    adapter.tier1.release_after_use(session);

    // Try to parse the result string as a JSON value. If it succeeds, carry the
    // type through the type-name helper. If not, fall back to raw string output.
    let (result_val, type_str) = match serde_json::from_str::<Value>(&raw) {
        Ok(parsed) => {
            let type_name = json_type_name(&parsed);
            (parsed, type_name)
        }
        Err(_) => (Value::String(raw), "string"),
    };

    Ok(json!({
        "result": result_val,
        "type":   type_str,
    }))
}
