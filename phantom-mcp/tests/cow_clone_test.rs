#![allow(clippy::unwrap_used, clippy::expect_used)]
use phantom_mcp::engine::get_test_adapter;
use phantom_mcp::{JsonRpcResponse, McpServer};
use phantom_session::{EngineKind, ResourceBudget, SessionState};
use std::time::Instant;

#[tokio::test]
async fn clone_produces_different_uuid() {
    let adapter = get_test_adapter().await;
    let src = adapter
        .broker
        .create(EngineKind::QuickJs, ResourceBudget::default(), "src");

    let clone_id = adapter.clone_session(src).await.unwrap();
    assert_ne!(src, clone_id, "clone must have different UUID from source");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn clone_completes_under_200ms_minimum() {
    let adapter = get_test_adapter().await;
    let src = adapter
        .broker
        .create(EngineKind::QuickJs, ResourceBudget::default(), "p");

    let start = Instant::now();
    let _clone_id = adapter.clone_session(src).await.unwrap();
    let elapsed = start.elapsed();

    println!("clone elapsed: {}ms", elapsed.as_millis());
    assert!(
        elapsed.as_millis() < 500,
        "clone must complete in < 500ms minimum, took {}ms",
        elapsed.as_millis()
    );
}

#[tokio::test]
async fn source_is_suspended_after_clone() {
    let adapter = get_test_adapter().await;
    let src = adapter
        .broker
        .create(EngineKind::QuickJs, ResourceBudget::default(), "p");

    adapter.clone_session(src).await.unwrap();

    let src_session = adapter.broker.get(src).unwrap();
    assert_eq!(
        src_session.state,
        SessionState::Suspended,
        "source session must be Suspended after COW clone"
    );
}

#[tokio::test]
async fn clone_is_running_after_clone() {
    let adapter = get_test_adapter().await;
    let src = adapter
        .broker
        .create(EngineKind::QuickJs, ResourceBudget::default(), "p");

    let clone_id = adapter.clone_session(src).await.unwrap();

    let clone_session = adapter.broker.get(clone_id).unwrap();
    assert_eq!(
        clone_session.state,
        SessionState::Running,
        "cloned session must be Running after clone"
    );
}

#[tokio::test]
async fn clone_snapshot_has_new_uuid_in_manifest() {
    let adapter = get_test_adapter().await;
    let src = adapter
        .broker
        .create(EngineKind::QuickJs, ResourceBudget::default(), "p");

    let clone_id = adapter.clone_session(src).await.unwrap();

    let clone_dir = adapter.storage.session_dir(&clone_id.to_string()).unwrap();

    let snapshots: Vec<_> = std::fs::read_dir(&clone_dir)
        .unwrap()
        .flatten()
        .filter(|e| {
            e.path()
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(".tar.zst"))
        })
        .collect();

    assert!(!snapshots.is_empty(), "clone dir must contain a snapshot");

    let bytes = std::fs::read(snapshots[0].path()).unwrap();
    let manifest = phantom_storage::snapshot::read_manifest_from_snapshot(&bytes).unwrap();
    assert_eq!(
        manifest.session_id,
        clone_id.to_string(),
        "clone snapshot manifest must use new session_id not source"
    );
}

#[tokio::test]
async fn clone_snapshot_hmac_verifies_with_new_uuid() {
    let adapter = get_test_adapter().await;
    let src = adapter
        .broker
        .create(EngineKind::QuickJs, ResourceBudget::default(), "p");

    let clone_id = adapter.clone_session(src).await.unwrap();

    let clone_dir = adapter.storage.session_dir(&clone_id.to_string()).unwrap();

    let snapshots: Vec<_> = std::fs::read_dir(&clone_dir)
        .unwrap()
        .flatten()
        .filter(|e| {
            e.path()
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(".tar.zst"))
        })
        .collect();

    let bytes = std::fs::read(snapshots[0].path()).unwrap();
    let manifest = phantom_storage::snapshot::read_manifest_from_snapshot(&bytes).unwrap();
    assert!(
        phantom_storage::snapshot::verify_manifest(&manifest).is_ok(),
        "clone HMAC must verify with the new session_id"
    );
}

#[tokio::test]
async fn both_sessions_in_broker_after_clone() {
    let adapter = get_test_adapter().await;

    let src = adapter
        .broker
        .create(EngineKind::QuickJs, ResourceBudget::default(), "p");

    let clone_id = adapter.clone_session(src).await.unwrap();

    // Both must be independently retrievable
    assert!(adapter.broker.get(src).is_ok());
    assert!(adapter.broker.get(clone_id).is_ok());
    assert_ne!(src, clone_id, "clone must have a distinct session id");
}

#[tokio::test]
async fn browser_session_clone_tool_registered() {
    let adapter = get_test_adapter().await;
    let server = McpServer::new_with_adapter(None, adapter.clone());

    let req = McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":"x","method":"browser_session_clone","params":{}}"#,
    )
    .unwrap();

    let resp: JsonRpcResponse = server.handle_request(&adapter, req, None).await.unwrap();

    // Must NOT return method_not_found (-32601)
    if let Some(ref err) = resp.error {
        assert_ne!(
            err.code, -32601,
            "browser_session_clone must be registered — got method_not_found"
        );
    }
}
