use phantom_storage::{SessionStorageManager, StorageError};
use tempfile::tempdir;

const SESSION: &str = "550e8400-e29b-41d4-a716-446655440000";

fn mgr() -> (tempfile::TempDir, SessionStorageManager) {
    let tmp = tempdir().unwrap();
    let mgr = SessionStorageManager::new(tmp.path());
    (tmp, mgr)
}

#[test]
fn cache_put_get_roundtrip() {
    let (_tmp, mgr) = mgr();
    let body = b"<html>hello</html>";
    let key = mgr
        .cache_put(SESSION, "v1", "https://example.com/index.html", body, "{}")
        .unwrap();
    let retrieved = mgr
        .cache_get(SESSION, "v1", "https://example.com/index.html")
        .unwrap();

    assert_eq!(retrieved, Some(body.to_vec()));
}

#[test]
fn cache_put_returns_sha256_key() {
    let (_tmp, mgr) = mgr();
    let key = mgr.cache_put(SESSION, "v1", "url", b"data", "{}").unwrap();

    assert_eq!(key.len(), 64, "SHA-256 = 64 hex chars");
    assert!(key.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn cache_miss_returns_none() {
    let (_tmp, mgr) = mgr();
    let retrieved = mgr
        .cache_get(SESSION, "v1", "https://never-cached.example.com")
        .unwrap();
    assert_eq!(retrieved, None);
}

#[test]
fn identical_bodies_produce_same_blob_key() {
    let (_tmp, mgr) = mgr();
    let k1 = mgr
        .cache_put(SESSION, "c1", "https://a.com/f", b"same", "{}")
        .unwrap();
    let k2 = mgr
        .cache_put(SESSION, "c2", "https://b.com/f", b"same", "{}")
        .unwrap();
    assert_eq!(k1, k2, "content addressing — same content = same key");
}

#[test]
fn blob_file_exists_after_put() {
    let (_tmp, mgr) = mgr();
    let key = mgr.cache_put(SESSION, "v1", "url", b"data", "{}").unwrap();
    let blob_path = mgr.cache_blobs_dir(SESSION).unwrap().join(&key);
    assert!(blob_path.exists(), "blob must be written to disk");
}
