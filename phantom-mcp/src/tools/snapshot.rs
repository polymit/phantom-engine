use axum::http::StatusCode;
use phantom_storage::is_valid_session_id;
use phantom_storage::snapshot::{build_snapshot, SnapshotData};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::SystemTime;

use crate::engine::EngineAdapter;

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

    // Step 1: Collect cookies_json
    let cookies_json = {
        let store = adapter.cookie_store.lock().await;
        serde_json::to_vec(&*store).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "error": { "code": "cookie_error", "message": format!("failed to serialize cookies: {}", e) } }),
            )
        })?
    };

    // Step 2: Collect local_storage from session storage dir localstorage/ subdir
    let mut local_storage = HashMap::new();
    if let Ok(dir) = adapter.storage.localstorage_dir(&uuid_str) {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("sled") {
                    if let Some(hash) = path.file_stem().and_then(|s| s.to_str()) {
                        if let Ok(db) = sled::open(&path) {
                            let mut map = HashMap::new();
                            for (k, v) in db.iter().flatten() {
                                map.insert(
                                    String::from_utf8_lossy(&k).into_owned(),
                                    String::from_utf8_lossy(&v).into_owned(),
                                );
                            }
                            if let Ok(json_bytes) = serde_json::to_vec(&map) {
                                local_storage.insert(hash.to_string(), json_bytes);
                            }
                        }
                    }
                }
            }
        }
    }

    // Step 3: Collect indexeddb from indexeddb/*.sqlite
    let mut indexeddb = HashMap::new();
    if let Ok(dir) = adapter.storage.indexeddb_dir(&uuid_str) {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("sqlite") {
                    if let Some(hash) = path.file_stem().and_then(|s| s.to_str()) {
                        if let Ok(bytes) = std::fs::read(&path) {
                            indexeddb.insert(hash.to_string(), bytes);
                        }
                    }
                }
            }
        }
    }

    // Collect cache metadata and blobs
    let mut cache_blobs = HashMap::new();
    if let Ok(dir) = adapter.storage.cache_blobs_dir(&uuid_str) {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(hash) = path.file_name().and_then(|s| s.to_str()) {
                        if let Ok(bytes) = std::fs::read(&path) {
                            cache_blobs.insert(hash.to_string(), bytes);
                        }
                    }
                }
            }
        }
    }

    let cache_meta = match adapter.storage.cache_meta_path(&uuid_str) {
        Ok(path) => {
            if path.exists() {
                if let Ok(db) = sled::open(&path) {
                    let mut map = HashMap::new();
                    for (k, v) in db.iter().flatten() {
                        map.insert(
                            String::from_utf8_lossy(&k).into_owned(),
                            String::from_utf8_lossy(&v).into_owned(),
                        );
                    }
                    serde_json::to_vec(&map).ok()
                } else {
                    None
                }
            } else {
                None
            }
        }
        _ => None,
    };

    let data = SnapshotData {
        session_id: uuid_str.clone(),
        cookies_json,
        local_storage,
        indexeddb,
        cache_blobs,
        cache_meta,
    };

    // Step 4: Call build_snapshot
    let compressed = build_snapshot(&data).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "error": { "code": "storage_error", "message": format!("failed to build tar.zst snapshot: {}", e) } }),
        )
    })?;

    // Step 5: Write to snapshot-<uuid>-<timestamp>.tar.zst
    let timestamp = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let snapshot_path = session_dir.join(format!("snapshot-{}-{}.tar.zst", uuid_str, timestamp));

    tokio::fs::write(&snapshot_path, &compressed).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "error": { "code": "io_error", "message": format!("failed to write snapshot: {}", e) } }),
        )
    })?;

    // Step 6: Return json
    Ok(json!({
        "snapshot_path": snapshot_path.to_string_lossy().into_owned(),
        "size_bytes": compressed.len()
    }))
}
