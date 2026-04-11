use axum::http::StatusCode;
use serde_json::{json, Value};

use crate::engine::EngineAdapter;

pub async fn handle_session_clone(
    adapter: &EngineAdapter,
    _params: Value,
) -> Result<Value, (StatusCode, Value)> {
    let source_id = adapter.session_uuid;

    let new_id = adapter.clone_session(source_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({
                "error": { "code": "clone_failed", "message": e }
            }),
        )
    })?;

    Ok(json!({
        "original_session_id": source_id.to_string(),
        "cloned_session_id": new_id.to_string(),
    }))
}
