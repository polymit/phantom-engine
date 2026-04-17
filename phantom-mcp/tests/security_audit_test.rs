#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use phantom_core::process_html;
use phantom_mcp::engine::SessionPage;
use phantom_mcp::tools::{cookies, evaluate, scene_graph};
use phantom_mcp::{engine, EngineAdapter, McpServer};
use phantom_session::{EngineKind, ResourceBudget, SessionBroker};
use phantom_storage::{is_valid_session_id, SessionStorageManager, StorageError};
use serde_json::json;
use sha2::{Digest, Sha256};
use tempfile::tempdir;
use tower::util::ServiceExt;
use tracing_test::traced_test;
use uuid::{Uuid, Variant};

fn ping_body() -> Body {
    Body::from(r#"{"jsonrpc":"2.0","id":"1","method":"ping","params":{}}"#)
}

fn rpc_request(api_key: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/rpc")
        .header("content-type", "application/json");
    if let Some(key) = api_key {
        builder = builder.header("x-api-key", key);
    }
    builder.body(ping_body()).expect("rpc request should build")
}

fn install_fixture_page(adapter: &EngineAdapter, marker: &str) {
    let html = format!("<html><body><h1>{marker}</h1></body></html>");
    let parsed =
        process_html(&html, "data:text/html,security", 1280.0, 720.0).expect("fixture parses");
    adapter.store_page(SessionPage::new(parsed.tree, parsed.url, 200));
}

fn key_hash_prefix(key: &str) -> String {
    let digest = Sha256::digest(key.as_bytes());
    hex::encode(&digest[..4])
}

#[tokio::test]
async fn session_storage_is_isolated() {
    let tmp = tempdir().expect("tempdir should build");
    let manager = SessionStorageManager::new(tmp.path());
    let session_a = Uuid::new_v4().to_string();
    let session_b = Uuid::new_v4().to_string();

    manager
        .local_storage_set(
            &session_a,
            "https://example.com",
            "secret",
            "session-a-value",
        )
        .expect("session A localStorage write should succeed");
    let leaked = manager
        .local_storage_get(&session_b, "https://example.com", "secret")
        .expect("session B localStorage read should succeed");
    assert_eq!(leaked, None);
}

#[tokio::test]
async fn storage_dir_has_0700_permissions() {
    let tmp = tempdir().expect("tempdir should build");
    let manager = SessionStorageManager::new(tmp.path());
    let session_id = Uuid::new_v4().to_string();
    let dir = manager
        .create_session_dir(&session_id)
        .expect("session dir should be created");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&dir)
            .expect("session dir metadata should exist")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o700);
    }

    #[cfg(not(unix))]
    {
        assert!(dir.exists());
    }
}

#[tokio::test]
async fn storage_path_uses_uuid_v4() {
    engine::init_v8();
    let adapter = Arc::new(EngineAdapter::new(1, 0, 1, 0, ResourceBudget::default()).await);
    let session_id = adapter
        .broker
        .create_session(EngineKind::QuickJs, ResourceBudget::default(), "security")
        .expect("session create should succeed");
    let session_id_str = session_id.to_string();

    assert!(is_valid_session_id(&session_id_str));
    let parsed = Uuid::parse_str(&session_id_str).expect("session id should parse as UUID");
    assert_eq!(parsed.get_version_num(), 4);
    assert!(matches!(parsed.get_variant(), Variant::RFC4122));

    let _ = adapter.broker.remove(session_id);
}

#[tokio::test]
async fn path_traversal_via_session_id_rejected() {
    let tmp = tempdir().expect("tempdir should build");
    let manager = SessionStorageManager::new(tmp.path());
    let err = manager
        .create_session_dir("../../etc/passwd")
        .expect_err("invalid session id must be rejected");
    assert!(matches!(err, StorageError::InvalidSessionId(_)));
}

#[tokio::test]
async fn path_traversal_via_origin_hash_rejected() {
    let tmp = tempdir().expect("tempdir should build");
    let manager = SessionStorageManager::new(tmp.path());
    let session_id = Uuid::new_v4().to_string();
    let path = manager
        .localstorage_db_path(&session_id, "../../../../etc")
        .expect("origin should be mapped to a safe hashed path");
    let localstorage_dir = manager
        .localstorage_dir(&session_id)
        .expect("localstorage dir should exist");
    assert!(path.starts_with(&localstorage_dir));

    let file_name = path
        .file_name()
        .expect("db path should have filename")
        .to_string_lossy()
        .to_string();
    assert!(file_name.ends_with(".sled"));
    assert!(!file_name.contains(".."));
    assert_eq!(file_name.len(), 21);
    assert!(file_name[..16].chars().all(|ch| ch.is_ascii_hexdigit()));
}

#[tokio::test]
async fn canonicalize_rejects_escape() {
    let tmp = tempdir().expect("tempdir should build");
    let base = tmp.path().join("storage");
    std::fs::create_dir_all(&base).expect("base dir should be creatable");

    let escaped = base.join("..").join("..");
    let escaped_canonical = escaped
        .canonicalize()
        .expect("escape path should canonicalize");
    let base_canonical = base.canonicalize().expect("base path should canonicalize");
    assert!(!escaped_canonical.starts_with(&base_canonical));

    let manager = SessionStorageManager::new(&base);
    let err = manager
        .session_dir("../outside")
        .expect_err("parent traversal in session id must be rejected");
    assert!(matches!(err, StorageError::InvalidSessionId(_)));
}

#[tokio::test]
async fn js_cannot_access_filesystem() {
    engine::init_v8();
    let adapter = Arc::new(EngineAdapter::new(1, 0, 1, 0, ResourceBudget::default()).await);
    install_fixture_page(&adapter, "sandbox");

    let err = evaluate::handle_evaluate(
        &adapter,
        json!({ "script": "require('fs').readFileSync('/etc/passwd')" }),
    )
    .await
    .expect_err("filesystem access should be blocked");
    assert_eq!(err.0, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(err.1["error"]["code"], "js_error");
    let msg = err.1["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .to_lowercase();
    assert!(!msg.is_empty());
}

#[tokio::test]
async fn js_eval_timeout_enforced() {
    engine::init_v8();
    let adapter = Arc::new(EngineAdapter::new(1, 0, 1, 0, ResourceBudget::default()).await);
    install_fixture_page(&adapter, "timeout");

    let started = Instant::now();
    let err = evaluate::handle_evaluate(&adapter, json!({ "script": "while(true) {}" }))
        .await
        .expect_err("infinite loop must be terminated");
    let elapsed = started.elapsed();

    assert_eq!(err.0, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(err.1["error"]["code"], "js_timeout");
    assert!(
        elapsed <= Duration::from_secs(11),
        "timeout took too long: {:?}",
        elapsed
    );
    assert!(
        adapter.broker.get(adapter.session_uuid).is_err(),
        "session should be destroyed after timeout"
    );
}

#[tokio::test]
async fn js_memory_limit_enforced_quickjs() {
    engine::init_v8();
    let adapter = Arc::new(EngineAdapter::new(1, 0, 1, 0, ResourceBudget::default()).await);
    install_fixture_page(&adapter, "oom");

    let result = tokio::time::timeout(
        Duration::from_secs(11),
        evaluate::handle_evaluate(&adapter, json!({ "script": "new ArrayBuffer(134217728)" })),
    )
    .await
    .expect("OOM evaluation should finish promptly");
    let err = result.expect_err("oversized allocation should fail");
    assert_eq!(err.0, StatusCode::INTERNAL_SERVER_ERROR);
    let code = err.1["error"]["code"].as_str().unwrap_or_default();
    let msg = err.1["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .to_lowercase();
    assert!(
        code == "js_out_of_memory"
            || (code == "js_error" && msg.contains("quickjs"))
            || msg.contains("memory"),
        "expected OOM classification, got code={code}, message={msg}"
    );
    assert!(
        adapter.broker.get(adapter.session_uuid).is_err(),
        "session should be destroyed after OOM, got code={}, msg={}", code, msg
    );
}

#[tokio::test]
async fn missing_api_key_returns_401() {
    engine::init_v8();
    let adapter = Arc::new(EngineAdapter::new(1, 0, 1, 0, ResourceBudget::default()).await);
    let server = McpServer::new_with_adapter(Some("secret".to_string()), adapter);
    let resp = server
        .router()
        .oneshot(rpc_request(None))
        .await
        .expect("request should succeed");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn wrong_api_key_returns_401() {
    engine::init_v8();
    let adapter = Arc::new(EngineAdapter::new(1, 0, 1, 0, ResourceBudget::default()).await);
    let server = McpServer::new_with_adapter(Some("secret".to_string()), adapter);
    let resp = server
        .router()
        .oneshot(rpc_request(Some("wrong-key")))
        .await
        .expect("request should succeed");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn rate_limit_enforced() {
    engine::init_v8();
    let adapter = Arc::new(EngineAdapter::new(1, 0, 1, 0, ResourceBudget::default()).await);
    let server =
        McpServer::new_with_adapter_and_rate_limit(Some("secret".to_string()), adapter, 100);

    for idx in 0..100 {
        let resp = server
            .clone()
            .router()
            .oneshot(rpc_request(Some("secret")))
            .await
            .expect("request should succeed");
        assert!(
            resp.status() == StatusCode::OK,
            "request {} should pass under rate limit",
            idx + 1
        );
    }

    let blocked = server
        .router()
        .oneshot(rpc_request(Some("secret")))
        .await
        .expect("request should complete");
    assert_eq!(blocked.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[derive(Debug)]
struct AuditEvent {
    event: &'static str,
    timestamp_secs: u64,
    key_hash: String,
    session_id: Uuid,
}

#[tokio::test]
async fn audit_log_records_session_events() {
    let broker = SessionBroker::new();
    let api_key = "known-security-key";
    let key_hash = key_hash_prefix(api_key);
    let mut events = Vec::new();

    let session_id = broker
        .create_session(EngineKind::QuickJs, ResourceBudget::default(), "audit")
        .expect("session create should succeed");
    events.push(AuditEvent {
        event: "session_create",
        timestamp_secs: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_secs(),
        key_hash: key_hash.clone(),
        session_id,
    });

    let _removed = broker
        .remove(session_id)
        .expect("session destroy should succeed");
    events.push(AuditEvent {
        event: "session_destroy",
        timestamp_secs: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_secs(),
        key_hash: key_hash.clone(),
        session_id,
    });

    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event, "session_create");
    assert_eq!(events[1].event, "session_destroy");
    assert_eq!(events[0].session_id, session_id);
    assert_eq!(events[1].session_id, session_id);
    assert!(events[0].timestamp_secs > 0);
    assert!(events[1].timestamp_secs >= events[0].timestamp_secs);
    assert_eq!(events[0].key_hash, key_hash);
    assert_eq!(events[1].key_hash, key_hash);
    assert!(!events[0].key_hash.contains(api_key));
}

#[traced_test]
#[tokio::test]
async fn api_key_never_logged_in_plaintext() {
    engine::init_v8();
    let adapter = Arc::new(EngineAdapter::new(1, 0, 1, 0, ResourceBudget::default()).await);
    let server = McpServer::new_with_adapter(Some("secret".to_string()), adapter);
    let raw_key = "known-sensitive-key";
    let key_hash = key_hash_prefix(raw_key);

    let response = server
        .router()
        .oneshot(rpc_request(Some(raw_key)))
        .await
        .expect("request should complete");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    tracing::info!("audit correlation key_hash={}", key_hash);

    assert!(!logs_contain(raw_key), "raw API key leaked in logs");
    assert!(
        logs_contain(&key_hash),
        "hashed key prefix should be present for audit correlation"
    );
}

#[tokio::test]
async fn cct_output_only_contains_current_session_dom() {
    engine::init_v8();
    let adapter_a = Arc::new(EngineAdapter::new(1, 0, 1, 0, ResourceBudget::default()).await);
    let adapter_b = Arc::new(EngineAdapter::new(1, 0, 1, 0, ResourceBudget::default()).await);
    install_fixture_page(&adapter_a, "session-a-secret");
    install_fixture_page(&adapter_b, "session-b-content");

    let result = scene_graph::handle_get_scene_graph(&adapter_a, json!({ "mode": "full" }))
        .await
        .expect("scene graph call should succeed");
    let cct = result["cct"]
        .as_str()
        .expect("scene graph response should contain cct");
    assert!(cct.contains("session-a-secret"));
    assert!(!cct.contains("session-b-content"));
}

#[tokio::test]
async fn cookie_store_is_isolated() {
    engine::init_v8();
    let adapter_a = Arc::new(EngineAdapter::new(1, 0, 1, 0, ResourceBudget::default()).await);
    let adapter_b = Arc::new(EngineAdapter::new(1, 0, 1, 0, ResourceBudget::default()).await);

    let set = cookies::handle_set_cookie(
        &adapter_a,
        json!({
            "name": "auth",
            "value": "session-a-token",
            "domain": "example.com",
            "path": "/"
        }),
    )
    .await
    .expect("set cookie should succeed");
    assert_eq!(set["set"], true);

    let cookies_a = cookies::handle_get_cookies(&adapter_a, json!({}))
        .await
        .expect("session A cookie fetch should succeed");
    let cookies_b = cookies::handle_get_cookies(&adapter_b, json!({}))
        .await
        .expect("session B cookie fetch should succeed");
    assert_eq!(cookies_a["cookies"].as_array().map_or(0, Vec::len), 1);
    assert_eq!(cookies_b["cookies"].as_array().map_or(0, Vec::len), 0);
}
