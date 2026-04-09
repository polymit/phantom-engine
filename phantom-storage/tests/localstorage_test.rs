use phantom_storage::{normalise_origin, origin_hash, SessionStorageManager, StorageError};
use tempfile::tempdir;

const SESSION_A: &str = "550e8400-e29b-41d4-a716-446655440000";
const SESSION_B: &str = "550e8400-e29b-41d4-a716-446655440001";
const ORIGIN: &str = "https://example.com";
const ORIGIN2: &str = "https://other.com";

fn mgr() -> (tempfile::TempDir, SessionStorageManager) {
    let tmp = tempdir().unwrap();
    let mgr = SessionStorageManager::new(tmp.path());
    (tmp, mgr)
}

#[test]
fn localstorage_set_get_roundtrip() {
    let (_tmp, mgr) = mgr();
    mgr.local_storage_set(SESSION_A, ORIGIN, "token", "abc123")
        .unwrap();
    let val = mgr.local_storage_get(SESSION_A, ORIGIN, "token").unwrap();
    assert_eq!(
        val.as_deref(),
        Some("abc123"),
        "localStorage get must return what was set"
    );
}

#[test]
fn localstorage_get_missing_returns_none() {
    let (_tmp, mgr) = mgr();
    let val = mgr
        .local_storage_get(SESSION_A, ORIGIN, "no_such_key")
        .unwrap();
    assert_eq!(val, None);
}

#[test]
fn localstorage_remove_makes_key_absent() {
    let (_tmp, mgr) = mgr();
    mgr.local_storage_set(SESSION_A, ORIGIN, "k", "v").unwrap();
    mgr.local_storage_remove(SESSION_A, ORIGIN, "k").unwrap();
    let val = mgr.local_storage_get(SESSION_A, ORIGIN, "k").unwrap();
    assert_eq!(val, None);
}

#[test]
fn localstorage_clear_removes_all() {
    let (_tmp, mgr) = mgr();
    mgr.local_storage_set(SESSION_A, ORIGIN, "k1", "v1")
        .unwrap();
    mgr.local_storage_set(SESSION_A, ORIGIN, "k2", "v2")
        .unwrap();
    mgr.local_storage_clear(SESSION_A, ORIGIN).unwrap();
    assert_eq!(
        mgr.local_storage_get(SESSION_A, ORIGIN, "k1").unwrap(),
        None
    );
    assert_eq!(
        mgr.local_storage_get(SESSION_A, ORIGIN, "k2").unwrap(),
        None
    );
}

#[test]
fn localstorage_isolated_per_session() {
    let (_tmp, mgr) = mgr();
    mgr.local_storage_set(SESSION_A, ORIGIN, "key", "a")
        .unwrap();
    mgr.local_storage_set(SESSION_B, ORIGIN, "key", "b")
        .unwrap();
    assert_eq!(
        mgr.local_storage_get(SESSION_A, ORIGIN, "key")
            .unwrap()
            .as_deref(),
        Some("a")
    );
    assert_eq!(
        mgr.local_storage_get(SESSION_B, ORIGIN, "key")
            .unwrap()
            .as_deref(),
        Some("b"),
        "cross-session isolation — blueprint Section 6.7"
    );
}

#[test]
fn localstorage_isolated_per_origin() {
    let (_tmp, mgr) = mgr();
    mgr.local_storage_set(SESSION_A, ORIGIN, "key", "val1")
        .unwrap();
    mgr.local_storage_set(SESSION_A, ORIGIN2, "key", "val2")
        .unwrap();
    let val1 = mgr.local_storage_get(SESSION_A, ORIGIN, "key").unwrap();
    let val2 = mgr.local_storage_get(SESSION_A, ORIGIN2, "key").unwrap();
    assert_ne!(val1, val2, "different origins must have isolated storage");
    assert_eq!(val1.as_deref(), Some("val1"));
    assert_eq!(val2.as_deref(), Some("val2"));
}

#[test]
fn localstorage_export_import_roundtrip() {
    let (_tmp, mgr) = mgr();
    mgr.local_storage_set(SESSION_A, ORIGIN, "k1", "v1")
        .unwrap();
    mgr.local_storage_set(SESSION_A, ORIGIN, "k2", "v2")
        .unwrap();

    let exported = mgr.local_storage_export(SESSION_A, ORIGIN).unwrap();
    assert_eq!(exported.len(), 2);

    mgr.local_storage_import(SESSION_B, ORIGIN, &exported)
        .unwrap();
    assert_eq!(
        mgr.local_storage_get(SESSION_B, ORIGIN, "k1")
            .unwrap()
            .as_deref(),
        Some("v1")
    );
    assert_eq!(
        mgr.local_storage_get(SESSION_B, ORIGIN, "k2")
            .unwrap()
            .as_deref(),
        Some("v2")
    );
}

#[test]
fn origin_hash_is_deterministic() {
    let h1 = origin_hash("https://example.com");
    let h2 = origin_hash("https://example.com");
    assert_eq!(
        h1, h2,
        "origin_hash must be deterministic — used as filename"
    );
}

#[test]
fn origin_hash_length_is_16() {
    let h = origin_hash("https://example.com");
    assert_eq!(h.len(), 16, "16 hex chars = 8 bytes SHA-256 prefix");
}

#[test]
fn origin_hash_differs_per_origin() {
    let h1 = origin_hash("https://example.com");
    let h2 = origin_hash("https://other.com");
    assert_ne!(h1, h2);
}

#[test]
fn normalise_origin_lowercases() {
    assert_eq!(
        normalise_origin("https://EXAMPLE.COM/"),
        "https://example.com"
    );
    assert_eq!(normalise_origin("HTTPS://FOO.BAR"), "https://foo.bar");
}

#[test]
fn normalise_origin_strips_trailing_slash() {
    assert_eq!(
        normalise_origin("https://example.com/"),
        "https://example.com"
    );
}

#[test]
fn session_id_traversal_rejected() {
    let mgr = SessionStorageManager::new("/tmp/phantom-test");
    let result = mgr.session_dir("../../../etc");
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        StorageError::InvalidSessionId(_)
    ));
}

#[test]
fn create_session_dir_is_under_base() {
    let (_tmp, mgr) = mgr();
    let dir = mgr.create_session_dir(SESSION_A).unwrap();
    assert!(
        dir.starts_with(mgr.base_dir()),
        "must be under base_dir — no traversal"
    );
}

#[test]
#[cfg(unix)]
fn create_session_dir_permissions_0700() {
    use std::os::unix::fs::PermissionsExt;
    let (_tmp, mgr) = mgr();
    let dir = mgr.create_session_dir(SESSION_A).unwrap();
    let mode = std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o700, "0700 required by blueprint Section 6.7");
}
