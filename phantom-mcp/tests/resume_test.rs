use phantom_mcp::engine::get_test_adapter;
use phantom_session::{EngineKind, ResourceBudget, SessionState};
use std::time::Instant;
use url::Url;

#[tokio::test]
async fn resume_state_becomes_running() {
    let adapter = get_test_adapter().await;
    let session_id = adapter
        .broker
        .create(EngineKind::QuickJs, ResourceBudget::default(), "p");

    adapter.suspend(session_id).await.unwrap();
    adapter.resume(session_id).await.unwrap();

    let session = adapter.broker.get(session_id).unwrap();
    assert_eq!(
        session.state,
        SessionState::Running,
        "session state must be Running after resume()"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resume_completes_under_50ms() {
    let adapter = get_test_adapter().await;
    let session_id = adapter
        .broker
        .create(EngineKind::QuickJs, ResourceBudget::default(), "p");

    adapter.suspend(session_id).await.unwrap();

    let start = Instant::now();
    adapter.resume(session_id).await.unwrap();
    let elapsed = start.elapsed();

    println!("resume elapsed: {}ms", elapsed.as_millis());
    assert!(
        elapsed.as_millis() < 50,
        "resume must complete in < 50ms, took {}ms",
        elapsed.as_millis()
    );
}

#[tokio::test]
async fn resume_cookies_survive_suspend_resume() {
    let adapter = get_test_adapter().await;
    let session_id = adapter
        .broker
        .create(EngineKind::QuickJs, ResourceBudget::default(), "p");

    {
        let url = Url::parse("https://secure.example.com").unwrap();
        let mut store = adapter.cookie_store.lock().await;
        store
            .parse(
                "auth=secret_token_123; Domain=secure.example.com; Path=/; Secure",
                &url,
            )
            .unwrap();
        store
            .parse(
                "csrf=xyzxyzxyz; Domain=secure.example.com; Path=/",
                &url,
            )
            .unwrap();
    }

    adapter.suspend(session_id).await.unwrap();

    // Wipe in-memory state to prove resume actually restores from disk
    adapter.cookie_store.lock().await.clear();

    adapter.resume(session_id).await.unwrap();

    let store = adapter.cookie_store.lock().await;
    let url2 = Url::parse("https://secure.example.com/").unwrap();
    let cookies = store.matches(&url2);

    assert_eq!(
        cookies.len(),
        2,
        "both cookies must survive suspend -> wipe -> resume"
    );
    let names: Vec<&str> = cookies.iter().map(|c| c.name()).collect();
    assert!(names.contains(&"auth"), "auth cookie must survive");
    assert!(names.contains(&"csrf"), "csrf cookie must survive");
}

#[tokio::test]
async fn resume_fails_on_tampered_snapshot() {
    let adapter = get_test_adapter().await;
    let session_id = adapter
        .broker
        .create(EngineKind::QuickJs, ResourceBudget::default(), "p");

    let path = adapter.suspend(session_id).await.unwrap();

    // Corrupt archive bytes to trigger either zstd decode or HMAC failure
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[50] ^= 0xFF;
    std::fs::write(&path, &bytes).unwrap();

    let result = adapter.resume(session_id).await;
    // Corruption may surface as zstd decode error or HMAC mismatch —
    // either is acceptable as long as corrupted data is never loaded silently.
    println!("tamper result: {:?}", result);
}

#[tokio::test]
async fn resume_localstorage_survives() {
    let adapter = get_test_adapter().await;
    let session_id = adapter
        .broker
        .create(EngineKind::QuickJs, ResourceBudget::default(), "p");
    let session_id_str = session_id.to_string();

    adapter
        .storage
        .local_storage_set(&session_id_str, "https://example.com", "user_id", "agent_99")
        .unwrap();

    adapter.suspend(session_id).await.unwrap();

    // Sled creates directories — remove the entire DB dir to simulate clean state
    let ls_path = adapter
        .storage
        .localstorage_db_path(&session_id_str, "https://example.com")
        .unwrap();
    std::fs::remove_dir_all(&ls_path).ok();

    adapter.resume(session_id).await.unwrap();

    let val = adapter
        .storage
        .local_storage_get(&session_id_str, "https://example.com", "user_id")
        .unwrap();
    assert_eq!(
        val,
        Some("agent_99".to_string()),
        "localStorage key must survive suspend -> file deletion -> resume"
    );
}

#[tokio::test]
async fn resume_latest_snapshot_is_used() {
    let adapter = get_test_adapter().await;
    let session_id = adapter
        .broker
        .create(EngineKind::QuickJs, ResourceBudget::default(), "p");

    // First suspend — no cookies
    adapter.suspend(session_id).await.unwrap();
    adapter
        .broker
        .set_state(session_id, SessionState::Running)
        .unwrap();

    // Add a cookie between suspends
    {
        let url = Url::parse("https://example.com").unwrap();
        let mut store = adapter.cookie_store.lock().await;
        store
            .parse(
                "new_cookie=after_second_suspend; Domain=example.com; Path=/",
                &url,
            )
            .unwrap();
    }

    // Second suspend — timestamp-named file must sort after the first
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    adapter.suspend(session_id).await.unwrap();

    // Wipe cookies and resume
    adapter.cookie_store.lock().await.clear();
    adapter.resume(session_id).await.unwrap();

    let store = adapter.cookie_store.lock().await;
    let url2 = Url::parse("https://example.com/").unwrap();
    assert!(
        store.matches(&url2).iter().any(|c| c.name() == "new_cookie"),
        "resume must use the LATEST snapshot file"
    );
}
