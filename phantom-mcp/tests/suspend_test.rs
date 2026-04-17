#![allow(clippy::unwrap_used, clippy::expect_used)]
use phantom_mcp::engine::get_test_adapter;
use phantom_session::{EngineKind, ResourceBudget, SessionState};
use std::path::Path;
use std::time::Instant;
use url::Url;

#[tokio::test]
async fn suspend_creates_snapshot_file() {
    let adapter = get_test_adapter().await;
    let session_id = adapter
        .broker
        .create(EngineKind::QuickJs, ResourceBudget::default(), "p");

    let path = adapter.suspend(session_id).await.unwrap();
    assert!(
        Path::new(&path).exists(),
        "suspend must create snapshot file"
    );
    assert!(path.ends_with(".tar.zst"), "must end with .tar.zst");
}

#[tokio::test]
async fn suspend_naming_contains_session_uuid() {
    let adapter = get_test_adapter().await;
    let session_id = adapter
        .broker
        .create(EngineKind::QuickJs, ResourceBudget::default(), "p");

    let path = adapter.suspend(session_id).await.unwrap();
    assert!(
        path.contains(&session_id.to_string()),
        "snapshot path must contain session UUID — blueprint Section 6.7"
    );
}

#[tokio::test]
async fn suspend_state_becomes_suspended() {
    let adapter = get_test_adapter().await;
    let session_id = adapter
        .broker
        .create(EngineKind::QuickJs, ResourceBudget::default(), "p");

    adapter.suspend(session_id).await.unwrap();
    let session = adapter.broker.get(session_id).unwrap();
    assert_eq!(
        session.state,
        SessionState::Suspended,
        "session state must be Suspended after suspend()"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn suspend_completes_under_200ms() {
    let adapter = get_test_adapter().await;
    let session_id = adapter
        .broker
        .create(EngineKind::QuickJs, ResourceBudget::default(), "p");

    let start = Instant::now();
    adapter.suspend(session_id).await.unwrap();
    let elapsed = start.elapsed();

    println!("suspend elapsed: {}ms", elapsed.as_millis());
    assert!(
        elapsed.as_millis() < 200,
        "suspend must complete in < 200ms — blueprint Section 6.5.2, took {}ms",
        elapsed.as_millis()
    );
}

#[tokio::test]
async fn suspend_snapshot_has_valid_zstd_magic() {
    let adapter = get_test_adapter().await;
    let session_id = adapter
        .broker
        .create(EngineKind::QuickJs, ResourceBudget::default(), "p");

    let path = adapter.suspend(session_id).await.unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert!(bytes.len() >= 4);
    assert_eq!(
        &bytes[..4],
        &[0x28, 0xB5, 0x2F, 0xFD],
        "snapshot must be valid zstd: 0x28 0xB5 0x2F 0xFD magic bytes"
    );
}

#[tokio::test]
async fn suspend_snapshot_hmac_verifies() {
    let adapter = get_test_adapter().await;
    let session_id = adapter
        .broker
        .create(EngineKind::QuickJs, ResourceBudget::default(), "p");

    let path = adapter.suspend(session_id).await.unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let manifest = phantom_storage::snapshot::read_manifest_from_snapshot(&bytes).unwrap();
    assert!(
        phantom_storage::snapshot::verify_manifest(&manifest).is_ok(),
        "snapshot HMAC must verify — blueprint Section 6.7"
    );
}

#[tokio::test]
async fn suspend_cookies_appear_in_snapshot() {
    let adapter = get_test_adapter().await;
    let session_id = adapter
        .broker
        .create(EngineKind::QuickJs, ResourceBudget::default(), "p");

    {
        let url = Url::parse("https://test.example.com").unwrap();
        let mut store = adapter.cookie_store.lock().await;
        store
            .parse(
                "test_cookie=sentinel_value; Domain=test.example.com; Path=/",
                &url,
            )
            .unwrap();
    }

    let path = adapter.suspend(session_id).await.unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let manifest = phantom_storage::snapshot::read_manifest_from_snapshot(&bytes).unwrap();
    assert!(
        manifest.checksums.contains_key("cookies.bin"),
        "cookies.bin must appear in snapshot manifest"
    );
}

#[tokio::test]
async fn suspend_called_twice_creates_two_files() {
    let adapter = get_test_adapter().await;
    let session_id = adapter
        .broker
        .create(EngineKind::QuickJs, ResourceBudget::default(), "p");

    let p1 = adapter.suspend(session_id).await.unwrap();

    // Must update state back to allow second suspend
    adapter
        .broker
        .set_state(session_id, SessionState::Running)
        .unwrap();

    // Slight delay to ensure timestamps differ if measured in secs
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    let p2 = adapter.suspend(session_id).await.unwrap();

    assert_ne!(
        p1, p2,
        "each suspend call must produce a distinct snapshot file"
    );
}
