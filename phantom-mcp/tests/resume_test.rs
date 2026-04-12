use phantom_mcp::engine::get_test_adapter;
use phantom_session::{EngineKind, ResourceBudget, SessionState};
use std::io::{Cursor, Read};
use std::time::Instant;
use url::Url;

fn corrupt_snapshot_file(path: &str, target_file: &str) -> Result<(), String> {
    let compressed = std::fs::read(path).map_err(|e| e.to_string())?;
    let decompressed = zstd::decode_all(Cursor::new(&compressed)).map_err(|e| e.to_string())?;
    let mut archive = tar::Archive::new(Cursor::new(&decompressed));

    let mut files = Vec::new();
    let mut found_target = false;

    for entry in archive.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        let path = entry
            .path()
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .into_owned();
        if !entry.header().entry_type().is_file() {
            continue;
        }

        let mode = entry.header().mode().unwrap_or(0o644);
        let mtime = entry.header().mtime().unwrap_or(0);

        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).map_err(|e| e.to_string())?;

        if path == target_file {
            found_target = true;
            if bytes.is_empty() {
                bytes.push(0xFF);
            } else {
                bytes[0] ^= 0xFF;
            }
        }

        files.push((path, bytes, mode, mtime));
    }

    if !found_target {
        return Err(format!("missing target file in snapshot: {}", target_file));
    }

    let mut tar_buf = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut tar_buf);
        for (path, bytes, mode, mtime) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(bytes.len() as u64);
            header.set_mode(mode);
            header.set_mtime(mtime);
            header.set_cksum();
            builder
                .append_data(&mut header, path, Cursor::new(bytes))
                .map_err(|e| e.to_string())?;
        }
        builder.finish().map_err(|e| e.to_string())?;
    }

    let recompressed = zstd::encode_all(Cursor::new(&tar_buf), 3).map_err(|e| e.to_string())?;
    std::fs::write(path, recompressed).map_err(|e| e.to_string())?;
    Ok(())
}

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
            .parse("csrf=xyzxyzxyz; Domain=secure.example.com; Path=/", &url)
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

    corrupt_snapshot_file(&path, "cookies.bin").unwrap();

    let result = adapter.resume(session_id).await;
    assert!(
        matches!(result, Err(ref msg) if msg.contains("checksum mismatch")),
        "tampered payload must be rejected by checksum validation, got {:?}",
        result
    );
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
        .local_storage_set(
            &session_id_str,
            "https://example.com",
            "user_id",
            "agent_99",
        )
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

    // Add session-scoped localStorage between suspends.
    let session_id_str = session_id.to_string();
    let origin = "https://example.com";
    adapter
        .storage
        .local_storage_set(
            &session_id_str,
            origin,
            "resume_marker",
            "from_second_snapshot",
        )
        .unwrap();

    // Second suspend — timestamp-named file must sort after the first
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    adapter.suspend(session_id).await.unwrap();

    // Wipe localStorage DB and resume from latest snapshot.
    let ls_path = adapter
        .storage
        .localstorage_db_path(&session_id_str, origin)
        .unwrap();
    std::fs::remove_dir_all(&ls_path).ok();

    adapter.resume(session_id).await.unwrap();

    let marker = adapter
        .storage
        .local_storage_get(&session_id_str, origin, "resume_marker")
        .unwrap();
    assert_eq!(
        marker,
        Some("from_second_snapshot".to_string()),
        "resume must use the LATEST snapshot file"
    );
}
