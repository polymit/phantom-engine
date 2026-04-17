#![allow(clippy::unwrap_used, clippy::expect_used)]
use cookie_store::CookieStore;
use phantom_storage::SessionStorageManager;
use tempfile::tempdir;
use url::Url;

fn make_store() -> cookie_store::CookieStore {
    let mut store = CookieStore::default();
    let url = Url::parse("https://example.com").unwrap();
    store
        .parse(
            "session_id=abc123; Domain=example.com; Path=/; Max-Age=3600",
            &url,
        )
        .unwrap();
    store
}

const SESSION: &str = "880e8400-e29b-41d4-a716-446655440000";

#[test]
fn save_load_roundtrip() {
    let tmp = tempdir().unwrap();
    let mgr = SessionStorageManager::new(tmp.path());
    mgr.save_cookies(SESSION, &make_store()).unwrap();
    let loaded = mgr.load_cookies(SESSION).unwrap();
    let url = Url::parse("https://example.com/").unwrap();
    let cookies: Vec<_> = loaded.matches(&url);
    assert_eq!(cookies.len(), 1, "one cookie must survive save+load");
    assert_eq!(cookies[0].name(), "session_id");
    assert_eq!(cookies[0].value(), "abc123");
}

#[test]
fn load_missing_file_returns_empty_store() {
    let tmp = tempdir().unwrap();
    let mgr = SessionStorageManager::new(tmp.path());
    let loaded = mgr.load_cookies(SESSION).unwrap();
    let url = Url::parse("https://example.com/").unwrap();
    assert_eq!(loaded.matches(&url).len(), 0);
}

#[test]
fn no_tmp_file_after_successful_save() {
    let tmp = tempdir().unwrap();
    let mgr = SessionStorageManager::new(tmp.path());
    mgr.save_cookies(SESSION, &make_store()).unwrap();
    let cookies_path = mgr.cookies_path(SESSION).unwrap();
    let tmp_path = cookies_path.with_extension("bin.tmp");
    assert!(!tmp_path.exists(), "atomic rename must clean up tmp file");
    assert!(cookies_path.exists(), "final file must exist");
}

#[test]
fn second_save_overwrites_first() {
    let tmp = tempdir().unwrap();
    let mgr = SessionStorageManager::new(tmp.path());
    let mut store1 = make_store();
    mgr.save_cookies(SESSION, &store1).unwrap();
    let url = Url::parse("https://example.com").unwrap();
    store1
        .parse(
            "auth_token=xyz789; Domain=example.com; Path=/; Max-Age=3600",
            &url,
        )
        .unwrap();
    mgr.save_cookies(SESSION, &store1).unwrap();
    let loaded = mgr.load_cookies(SESSION).unwrap();
    let url2 = Url::parse("https://example.com/").unwrap();
    assert_eq!(
        loaded.matches(&url2).len(),
        2,
        "overwrite must include both cookies"
    );
}

#[test]
fn delete_removes_file() {
    let tmp = tempdir().unwrap();
    let mgr = SessionStorageManager::new(tmp.path());
    mgr.save_cookies(SESSION, &make_store()).unwrap();
    mgr.delete_cookies(SESSION).unwrap();
    let path = mgr.cookies_path(SESSION).unwrap();
    assert!(!path.exists());
}

#[test]
fn delete_nonexistent_is_ok() {
    let tmp = tempdir().unwrap();
    let mgr = SessionStorageManager::new(tmp.path());
    let result = mgr.delete_cookies(SESSION);
    assert!(result.is_ok(), "delete on missing file must not error");
}

#[test]
fn save_is_isolated_per_session() {
    let tmp = tempdir().unwrap();
    let mgr = SessionStorageManager::new(tmp.path());
    let s2 = "880e8400-e29b-41d4-a716-446655440001";
    let store_a = make_store();
    let mut store_b = CookieStore::default();
    let url = Url::parse("https://example.com").unwrap();
    store_b
        .parse(
            "session_id=zzz999; Domain=example.com; Path=/; Max-Age=3600",
            &url,
        )
        .unwrap();
    mgr.save_cookies(SESSION, &store_a).unwrap();
    mgr.save_cookies(s2, &store_b).unwrap();
    let la = mgr.load_cookies(SESSION).unwrap();
    let lb = mgr.load_cookies(s2).unwrap();
    let url2 = Url::parse("https://example.com/").unwrap();
    let va: Vec<String> = la
        .matches(&url2)
        .into_iter()
        .map(|c| c.value().to_string())
        .collect();
    let vb: Vec<String> = lb
        .matches(&url2)
        .into_iter()
        .map(|c| c.value().to_string())
        .collect();
    assert!(va.contains(&"abc123".to_string()));
    assert!(vb.contains(&"zzz999".to_string()));
    assert!(
        !va.contains(&"zzz999".to_string()),
        "cross-session cookie isolation"
    );
}

#[test]
fn cookies_file_is_valid_json() {
    let tmp = tempdir().unwrap();
    let mgr = SessionStorageManager::new(tmp.path());
    mgr.save_cookies(SESSION, &make_store()).unwrap();
    let raw = std::fs::read_to_string(mgr.cookies_path(SESSION).unwrap()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert!(
        parsed.is_object() || parsed.is_array(),
        "cookies.bin must be valid JSON"
    );
}
