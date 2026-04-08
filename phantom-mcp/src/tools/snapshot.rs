use axum::http::StatusCode;
use serde_json::{json, Value};
use std::time::SystemTime;

use crate::engine::EngineAdapter;
use phantom_storage::is_valid_session_id;

pub async fn handle_session_snapshot(
    adapter: &EngineAdapter,
    _params: Value,
) -> Result<Value, (StatusCode, Value)> {
    let uuid_str = adapter.session_uuid.to_string();

    // 1. Validate session UUID
    if !is_valid_session_id(&uuid_str) {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "error": { "code": "storage_error", "message": "invalid session uuid" } }),
        ));
    }

    // 2. Create session directory
    let session_dir = adapter.storage.session_dir(&uuid_str).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "error": { "code": "storage_error", "message": e.to_string() } }),
        )
    })?;

    std::fs::create_dir_all(&session_dir).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "error": { "code": "io_error", "message": format!("failed to create session dir: {}", e) } }),
        )
    })?;

    // 3. Serialize cookies
    let cookies_json = {
        let store = adapter.cookie_store.lock().await;
        serde_json::to_string(&*store).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "error": { "code": "cookie_error", "message": format!("failed to serialize cookies: {}", e) } }),
            )
        })?
    };

    // 4. Build manifest JSON
    let timestamp = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let manifest = json!({
        "version": "1.0",
        "session_uuid": uuid_str,
        "timestamp": timestamp,
        "files": ["cookies.json"]
    });

    // 5. Build tar-like payload
    let payload = format!("{}\n{}", manifest, cookies_json);

    // 6. Compress with zstd::encode_all
    let compressed = zstd::encode_all(payload.as_bytes(), 3).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "error": { "code": "io_error", "message": format!("failed to compress snapshot: {}", e) } }),
        )
    })?;

    // 7. Write to path
    let snapshot_name = format!("snapshot-{}.tar.zst", timestamp);
    let snapshot_path = session_dir.join(&snapshot_name);

    tokio::fs::write(&snapshot_path, &compressed).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "error": { "code": "io_error", "message": format!("failed to write snapshot: {}", e) } }),
        )
    })?;

    // 8. Return result
    Ok(json!({
        "snapshot_path": snapshot_path.to_string_lossy().into_owned(),
        "size_bytes": compressed.len()
    }))
}
