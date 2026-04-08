use axum::http::StatusCode;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::engine::EngineAdapter;

pub async fn handle_new_tab(
    adapter: &EngineAdapter,
    params:  Value,
) -> Result<Value, (StatusCode, Value)> {
    // url is optional — an empty tab is valid
    let url: Option<String> = params
        .get("url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let stored_url = url.clone().unwrap_or_default();
    let tab_id = adapter.open_tab(url).await;

    Ok(json!({
        "tab_id": tab_id.to_string(),
        "url":    stored_url,
    }))
}

pub async fn handle_switch_tab(
    adapter: &EngineAdapter,
    params:  Value,
) -> Result<Value, (StatusCode, Value)> {
    let id_str = params
        .get("tab_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                json!({ "error": { "code": "invalid_params", "message": "tab_id is required" } }),
            )
        })?;

    let tab_id = Uuid::parse_str(id_str).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            json!({ "error": { "code": "invalid_params", "message": "tab_id must be a valid UUID" } }),
        )
    })?;

    let tab = adapter.switch_tab(tab_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            json!({ "error": { "code": "tab_not_found", "message": format!("no tab with id {}", tab_id) } }),
        )
    })?;

    Ok(json!({
        "tab_id": tab.id.to_string(),
        "url":    tab.url,
    }))
}

pub async fn handle_list_tabs(
    adapter: &EngineAdapter,
    _params: Value,
) -> Result<Value, (StatusCode, Value)> {
    let tabs = adapter.list_tabs().await;
    let tab_list: Vec<Value> = tabs
        .into_iter()
        .map(|t| {
            json!({
                "id":     t.id.to_string(),
                "url":    t.url,
                "title":  t.title,
                "active": t.active,
            })
        })
        .collect();

    Ok(json!({ "tabs": tab_list }))
}

pub async fn handle_close_tab(
    adapter: &EngineAdapter,
    params:  Value,
) -> Result<Value, (StatusCode, Value)> {
    let id_str = params
        .get("tab_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                json!({ "error": { "code": "invalid_params", "message": "tab_id is required" } }),
            )
        })?;

    let tab_id = Uuid::parse_str(id_str).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            json!({ "error": { "code": "invalid_params", "message": "tab_id must be a valid UUID" } }),
        )
    })?;

    let remaining = adapter.close_tab(tab_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            json!({ "error": { "code": "tab_not_found", "message": format!("no tab with id {}", tab_id) } }),
        )
    })?;

    Ok(json!({
        "closed":         true,
        "remaining_tabs": remaining,
    }))
}
