use phantom_storage::SessionStorageManager;
use tempfile::tempdir;

const SESSION: &str = "660e8400-e29b-41d4-a716-446655440000";
const ORIGIN: &str = "https://app.example.com";

fn mgr() -> (tempfile::TempDir, SessionStorageManager) {
    let tmp = tempdir().unwrap();
    let mgr = SessionStorageManager::new(tmp.path());
    (tmp, mgr)
}

#[test]
fn indexeddb_put_get_roundtrip() {
    let (_tmp, mgr) = mgr();
    mgr.indexeddb_put(
        SESSION,
        ORIGIN,
        "myDB",
        "tokens",
        "auth",
        r#"{"token":"xyz"}"#,
    )
    .unwrap();
    let val = mgr
        .indexeddb_get(SESSION, ORIGIN, "myDB", "tokens", "auth")
        .unwrap();
    assert_eq!(val.as_deref(), Some(r#"{"token":"xyz"}"#));
}

#[test]
fn indexeddb_get_missing_returns_none() {
    let (_tmp, mgr) = mgr();
    let val = mgr
        .indexeddb_get(SESSION, ORIGIN, "myDB", "tokens", "no_key")
        .unwrap();
    assert_eq!(val, None);
}

#[test]
fn indexeddb_delete() {
    let (_tmp, mgr) = mgr();
    mgr.indexeddb_put(SESSION, ORIGIN, "db", "store", "k", "v")
        .unwrap();
    mgr.indexeddb_delete(SESSION, ORIGIN, "db", "store", "k")
        .unwrap();
    let val = mgr
        .indexeddb_get(SESSION, ORIGIN, "db", "store", "k")
        .unwrap();
    assert_eq!(val, None);
}

#[test]
fn indexeddb_list_keys_sorted() {
    let (_tmp, mgr) = mgr();
    mgr.indexeddb_put(SESSION, ORIGIN, "myDB", "tokens", "z", "1")
        .unwrap();
    mgr.indexeddb_put(SESSION, ORIGIN, "myDB", "tokens", "a", "2")
        .unwrap();
    mgr.indexeddb_put(SESSION, ORIGIN, "myDB", "tokens", "m", "3")
        .unwrap();
    let keys = mgr
        .indexeddb_list_keys(SESSION, ORIGIN, "myDB", "tokens")
        .unwrap();
    assert_eq!(
        keys,
        vec!["a", "m", "z"],
        "keys must be sorted — ORDER BY key"
    );
}

#[test]
fn indexeddb_isolated_per_session() {
    let (_tmp, mgr) = mgr();
    mgr.indexeddb_put(SESSION, ORIGIN, "db", "s", "k", "session_1")
        .unwrap();

    let s2 = "770e8400-e29b-41d4-a716-446655440000";
    mgr.indexeddb_put(s2, ORIGIN, "db", "s", "k", "session_2")
        .unwrap();

    assert_eq!(
        mgr.indexeddb_get(SESSION, ORIGIN, "db", "s", "k")
            .unwrap()
            .as_deref(),
        Some("session_1")
    );
    assert_eq!(
        mgr.indexeddb_get(s2, ORIGIN, "db", "s", "k")
            .unwrap()
            .as_deref(),
        Some("session_2")
    );
}

#[test]
fn indexeddb_backup_creates_file() {
    let (tmp, mgr) = mgr();
    mgr.indexeddb_put(SESSION, ORIGIN, "db", "s", "k", "v")
        .unwrap();

    let backup_path = tmp.path().join("backup.sqlite");
    mgr.indexeddb_backup(SESSION, ORIGIN, &backup_path).unwrap();

    assert!(backup_path.exists());
    assert!(backup_path.metadata().unwrap().len() > 0);
}

#[test]
fn indexeddb_wal_mode_is_set() {
    let (_tmp, mgr) = mgr();
    // Initialize DB by writing to it
    mgr.indexeddb_put(SESSION, ORIGIN, "db", "s", "k", "v")
        .unwrap();

    let path = mgr.indexeddb_db_path(SESSION, ORIGIN).unwrap();
    let conn = rusqlite::Connection::open(&path).unwrap();
    let mode: String = conn
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        mode.to_lowercase(),
        "wal",
        "WAL mode required for concurrent access"
    );
}
