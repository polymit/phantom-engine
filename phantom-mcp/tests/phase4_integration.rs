use cookie_store::CookieStore;
use phantom_anti_detect::{ChromeProfile, GpuProfile, PersonaPool};
use phantom_core::process_html;
use phantom_mcp::engine::{get_test_adapter, SessionPage};
use phantom_serializer::{HeadlessSerializer, SerialiserConfig};
use phantom_session::{EngineKind, ResourceBudget, SessionState};
use phantom_storage::snapshot::{
    build_snapshot, read_manifest_from_snapshot, verify_manifest, SnapshotData,
};
use phantom_storage::SessionStorageManager;
use std::collections::HashMap;
use std::fs::read_dir;
use std::path::Path;
use std::time::Instant;
use tempfile::tempdir;
use url::Url;

fn dashboard_html() -> &'static str {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Authenticated Dashboard</title>
</head>
<body style="width:1280px;height:720px;margin:0;">
    <header style="width:1280px;height:60px;">
        <nav aria-label="Main navigation" style="width:1280px;height:60px;">
            <span id="user-greeting" style="width:300px;height:20px;">Welcome, agent@example.com</span>
            <button id="logout-btn" data-testid="logout" style="width:100px;height:40px;">Log out</button>
        </nav>
    </header>
    <main role="main" style="width:1280px;height:600px;">
        <article style="width:600px;height:200px;">
            <h2 style="width:500px;height:30px;">Project Alpha</h2>
            <button data-testid="view-project-1" style="width:120px;height:40px;">View Details</button>
        </article>
        <article style="width:600px;height:200px;">
            <h2 style="width:500px;height:30px;">Project Beta</h2>
            <button data-testid="view-project-2" style="width:120px;height:40px;">View Details</button>
        </article>
    </main>
    <footer style="width:1280px;height:60px;">
        <p style="width:400px;height:20px;">&copy; 2026 Phantom Corp</p>
    </footer>
</body>
</html>"#
}

#[tokio::test]
async fn test_persona_pool_completeness() {
    let pool = PersonaPool::default_pool();
    assert_eq!(pool.len(), 5, "Pool must have 5 personas");

    let mut chrome133_count = 0u32;
    let mut chrome134_count = 0u32;
    let mut has_macos = false;
    let mut has_windows = false;

    for i in 0..5 {
        let p = pool
            .clone_persona(i)
            .expect("persona index within default pool bounds");

        assert!(!p.platform_version.is_empty());
        assert!(!p.ua_full_version.is_empty());
        assert!(!p.ua_architecture.is_empty());
        assert!(!p.chrome_major.is_empty());
        assert!(p.screen_width > 0);
        assert!(p.screen_height > 0);
        assert!(p.device_pixel_ratio > 0.0);

        assert!(matches!(p.hardware_concurrency, 4 | 6 | 8 | 12 | 16));
        assert!(matches!(p.device_memory, 4 | 8));

        assert!(!p.webgl_vendor.contains("SwiftShader"));
        assert!(!p.webgl_renderer.contains("SwiftShader"));
        assert!(!p.webgl_renderer.contains("Mesa"));

        match p.chrome_version {
            ChromeProfile::Chrome133 => chrome133_count += 1,
            ChromeProfile::Chrome134 => chrome134_count += 1,
        }

        if p.platform == "MacIntel" {
            has_macos = true;
        }
        if p.platform == "Win32" {
            has_windows = true;
        }
    }

    assert!(chrome133_count >= 3, "Chrome 133 count must be at least 3");
    assert!(chrome134_count >= 1, "Chrome 134 count must be at least 1");
    assert!(has_macos, "macOS persona missing");
    assert!(has_windows, "Windows persona missing");
}

#[tokio::test]
async fn test_gpu_profiles_strings() {
    let profiles = vec![
        GpuProfile::WindowsNvidiaRtx3060Ti,
        GpuProfile::WindowsNvidiaRtx4070,
        GpuProfile::WindowsAmdRx6600,
        GpuProfile::MacOsIntelIrisPro,
        GpuProfile::WindowsIntelUhd770,
        GpuProfile::MacOsAppleM3Pro,
    ];

    for profile in profiles {
        let (v, r) = profile.strings();
        assert!(v.starts_with("Google Inc"));
        assert!(r.contains("ANGLE"));
        assert!(
            !r.contains("SwiftShader"),
            "WebGL renderer must not contain SwiftShader"
        );
        assert!(!r.contains("Mesa"), "WebGL renderer must not contain Mesa");
    }
}

#[tokio::test]
async fn test_localstorage_persistence() {
    let dir = tempdir().unwrap();
    let mgr = SessionStorageManager::new(dir.path());
    let session_id = "440e8400-e29b-41d4-a716-446655440000";
    mgr.local_storage_set(session_id, "https://example.com", "auth_pref", "dark")
        .unwrap();
    let val = mgr
        .local_storage_get(session_id, "https://example.com", "auth_pref")
        .unwrap();
    assert_eq!(val, Some("dark".to_string()), "localStorage value mismatch");
}

#[tokio::test]
async fn test_indexeddb_persistence() {
    let dir = tempdir().unwrap();
    let mgr = SessionStorageManager::new(dir.path());
    let session_id = "440e8400-e29b-41d4-a716-446655440000";
    mgr.indexeddb_put(
        session_id,
        "https://example.com",
        "testDB",
        "auth",
        "token_key",
        r#"{"jwt":"eyJ..."}"#,
    )
    .unwrap();
    let val = mgr
        .indexeddb_get(
            session_id,
            "https://example.com",
            "testDB",
            "auth",
            "token_key",
        )
        .unwrap();
    assert_eq!(val, Some(r#"{"jwt":"eyJ..."}"#.to_string()));
}

#[tokio::test]
async fn test_cookie_persistence() {
    let dir = tempdir().unwrap();
    let mgr = SessionStorageManager::new(dir.path());
    let session_id = "440e8400-e29b-41d4-a716-446655440000";

    let mut store = CookieStore::default();
    let url = Url::parse("https://secure.example.com").unwrap();
    store
        .parse(
            "auth=token_abc; Domain=secure.example.com; Path=/; Secure",
            &url,
        )
        .unwrap();
    mgr.save_cookies(session_id, &store).unwrap();

    let loaded = mgr.load_cookies(session_id).unwrap();
    let cookies = loaded.matches(&Url::parse("https://secure.example.com/").unwrap());
    assert_eq!(cookies.len(), 1);
    assert_eq!(cookies[0].name(), "auth");
}

#[tokio::test]
async fn test_snapshot_integrity() {
    let mut local_storage = HashMap::new();
    local_storage.insert("ls_data".to_string(), br#"{"k":"v"}"#.to_vec());

    let data = SnapshotData {
        session_id: "550e8400-e29b-41d4-a716-446655440003".to_string(),
        cookies_json: serde_json::to_vec(&CookieStore::default()).unwrap(),
        local_storage,
        indexeddb: HashMap::new(),
        cache_blobs: HashMap::new(),
        cache_meta: None,
    };
    let compressed = build_snapshot(&data).unwrap();

    assert_eq!(
        &compressed[..4],
        &[0x28, 0xB5, 0x2F, 0xFD],
        "Snapshot must be a zstd archive"
    );

    // Verify manifest
    let manifest = read_manifest_from_snapshot(&compressed).unwrap();
    assert_eq!(manifest.version, "1.0");
    assert_eq!(
        manifest.hmac_sig.len(),
        64,
        "HMAC-SHA256 must be 64 hex chars"
    );

    // Verify HMAC
    assert!(
        verify_manifest(&manifest).is_ok(),
        "HMAC must verify on fresh snapshot"
    );

    assert!(
        manifest.checksums.contains_key("localstorage/ls_data.json"),
        "localstorage entry missing from manifest"
    );
}

#[tokio::test]
async fn test_authenticated_session_lifecycle() {
    let adapter = get_test_adapter().await;
    let origin = "https://dashboard.example.com";
    let session_id = adapter.broker.create(
        EngineKind::QuickJs,
        ResourceBudget::default(),
        "integration_test",
    );
    let session_id_str = session_id.to_string();

    // Set authenticated state
    {
        let url = Url::parse(origin).unwrap();
        let mut store = adapter.cookie_store.lock().await;
        store
            .parse(
                "session_token=test_token; Domain=dashboard.example.com; Path=/; Secure",
                &url,
            )
            .unwrap();
        store
            .parse(
                "csrf_token=csrf_123; Domain=dashboard.example.com; Path=/",
                &url,
            )
            .unwrap();
    }
    adapter
        .storage
        .local_storage_set(&session_id_str, origin, "user_id", "agent_007")
        .unwrap();
    adapter
        .storage
        .local_storage_set(&session_id_str, origin, "theme", "dark")
        .unwrap();
    adapter
        .storage
        .indexeddb_put(&session_id_str, origin, "appDB", "prefs", "lang", "en-US")
        .unwrap();

    // Parse dashboard HTML into CCT
    let page = process_html(
        dashboard_html(),
        &format!("{}/dashboard", origin),
        1280.0,
        720.0,
    )
    .expect("parse dashboard HTML");
    adapter.store_page(SessionPage::new(
        page.tree,
        format!("{}/dashboard", origin),
        200,
    ));

    // SUSPEND
    let suspend_start = Instant::now();
    let snapshot_path = adapter
        .suspend(session_id)
        .await
        .expect("suspend must succeed");
    let suspend_ms = suspend_start.elapsed().as_millis();

    assert!(Path::new(&snapshot_path).exists(), "snapshot file missing");
    assert!(
        snapshot_path.contains(&session_id_str),
        "snapshot filename mismatch"
    );
    assert!(
        snapshot_path.ends_with(".tar.zst"),
        "snapshot must be .tar.zst"
    );
    assert!(
        suspend_ms < 500,
        "suspend timeout exceeded ({}ms)",
        suspend_ms
    );
    assert_eq!(
        adapter.broker.get(session_id).unwrap().state,
        SessionState::Suspended
    );

    // WIPE IN-MEMORY STATE (simulate restart)
    adapter.cookie_store.lock().await.clear();

    // RESUME
    let resume_start = Instant::now();
    adapter
        .resume(session_id)
        .await
        .expect("resume must succeed");
    let resume_ms = resume_start.elapsed().as_millis();

    assert!(resume_ms < 500, "resume timeout exceeded ({}ms)", resume_ms);
    assert_eq!(
        adapter.broker.get(session_id).unwrap().state,
        SessionState::Running
    );

    // VERIFY COOKIES SURVIVED
    {
        let store = adapter.cookie_store.lock().await;
        let url2 = Url::parse(&format!("{}/dashboard", origin)).unwrap();
        let cookies = store.matches(&url2);
        assert_eq!(
            cookies.len(),
            2,
            "both cookies must survive suspend -> wipe -> resume, got {}",
            cookies.len()
        );
        let names: Vec<&str> = cookies.into_iter().map(|c| c.name()).collect();
        assert!(
            names.contains(&"session_token"),
            "session_token must survive"
        );
        assert!(names.contains(&"csrf_token"), "csrf_token must survive");
    }

    // VERIFY LOCALSTORAGE SURVIVED
    let user_id = adapter
        .storage
        .local_storage_get(&session_id_str, origin, "user_id")
        .unwrap();
    assert_eq!(
        user_id,
        Some("agent_007".to_string()),
        "localStorage user_id must survive suspend/resume"
    );
    let theme = adapter
        .storage
        .local_storage_get(&session_id_str, origin, "theme")
        .unwrap();
    assert_eq!(
        theme,
        Some("dark".to_string()),
        "localStorage theme must survive suspend/resume"
    );

    // VERIFY INDEXEDDB SURVIVED
    let lang = adapter
        .storage
        .indexeddb_get(&session_id_str, origin, "appDB", "prefs", "lang")
        .unwrap();
    assert_eq!(
        lang,
        Some("en-US".to_string()),
        "IndexedDB lang preference must survive suspend/resume"
    );

    // VERIFY HMAC
    let bytes = std::fs::read(&snapshot_path).unwrap();
    let manifest = read_manifest_from_snapshot(&bytes).unwrap();
    assert!(
        verify_manifest(&manifest).is_ok(),
        "snapshot HMAC must verify after suspend"
    );

    assert!(
        verify_manifest(&manifest).is_ok(),
        "HMAC verification failed"
    );
}

#[tokio::test]
async fn test_session_cloning_cow() {
    let adapter = get_test_adapter().await;
    let origin = "https://clone.example.com";
    let src = adapter.broker.create(
        EngineKind::QuickJs,
        ResourceBudget::default(),
        "integration_test",
    );
    let src_str = src.to_string();

    // Set state before clone
    {
        let url = Url::parse(origin).unwrap();
        let mut store = adapter.cookie_store.lock().await;
        store
            .parse(
                "shared=before_clone_value; Domain=clone.example.com; Path=/",
                &url,
            )
            .unwrap();
    }
    adapter
        .storage
        .local_storage_set(&src_str, origin, "shared_data", "clone_test_123")
        .unwrap();

    // Clone
    let clone_start = Instant::now();
    let clone_id = adapter
        .clone_session(src)
        .await
        .expect("clone must succeed");
    let clone_ms = clone_start.elapsed().as_millis();

    assert_ne!(src, clone_id, "clone must have different UUID");
    // Full-suite CI load can cause occasional scheduling spikes; keep this as an
    // integration guardrail, not a micro-benchmark threshold.
    assert!(clone_ms < 5000, "clone timeout exceeded ({}ms)", clone_ms);
    assert_eq!(
        adapter.broker.get(src).unwrap().state,
        SessionState::Suspended,
        "source must be Suspended after COW clone"
    );
    assert_eq!(
        adapter.broker.get(clone_id).unwrap().state,
        SessionState::Running,
        "clone must be Running"
    );

    // Clone manifest must use new UUID
    let clone_dir = adapter.storage.session_dir(&clone_id.to_string()).unwrap();
    let snapshots: Vec<_> = read_dir(clone_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().to_string_lossy().ends_with(".tar.zst"))
        .collect();

    assert!(!snapshots.is_empty());
    let bytes = std::fs::read(snapshots[0].path()).unwrap();
    let manifest = read_manifest_from_snapshot(&bytes).unwrap();
    assert_eq!(
        manifest.session_id,
        clone_id.to_string(),
        "clone manifest must use new session_id"
    );
    assert!(
        verify_manifest(&manifest).is_ok(),
        "clone manifest HMAC error"
    );
}

#[tokio::test]
async fn test_cct_serialization_authenticated() {
    let page = process_html(
        dashboard_html(),
        "https://dashboard.example.com/home",
        1280.0,
        720.0,
    )
    .expect("parse dashboard HTML");

    let config = SerialiserConfig {
        url: "https://dashboard.example.com/home".to_string(),
        viewport_width: 1280.0,
        viewport_height: 720.0,
        ..Default::default()
    };

    let cct = HeadlessSerializer::serialise(&page, &config);
    assert!(cct.starts_with("##PAGE"), "CCT must start with ##PAGE");
    assert!(cct.contains("url=https%3A%2F%2Fdashboard.example.com%2Fhome"));

    let node_lines: Vec<&str> = cct
        .lines()
        .filter(|l| l.contains('|') && !l.starts_with('#'))
        .collect();
    assert!(!node_lines.is_empty(), "dashboard must produce CCT nodes");

    let has_logout = node_lines
        .iter()
        .any(|l| l.contains("logout") || l.contains("Log out"));
    assert!(has_logout, "logout button missing from CCT");
}
