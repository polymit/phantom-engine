pub mod snapshot;

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("invalid session id: {0}")]
    InvalidSessionId(String),
    #[error("sled error: {0}")]
    Sled(String),
    #[error("sqlite error: {0}")]
    Sqlite(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("path traversal attempt blocked: {0}")]
    PathTraversal(String),
    #[error("serialisation error: {0}")]
    Serialise(String),
}

pub fn normalise_origin(origin: &str) -> String {
    let trimmed = origin.trim_end_matches('/');
    if let Some(idx) = trimmed.find("://") {
        let (scheme_colon_slash_slash, rest) = trimmed.split_at(idx + 3);
        if let Some(path_idx) = rest.find('/') {
            let (host, path) = rest.split_at(path_idx);
            format!(
                "{}{}{}",
                scheme_colon_slash_slash.to_lowercase(),
                host.to_lowercase(),
                path
            )
        } else {
            format!(
                "{}{}",
                scheme_colon_slash_slash.to_lowercase(),
                rest.to_lowercase()
            )
        }
    } else {
        trimmed.to_lowercase()
    }
}

pub fn origin_hash(origin: &str) -> String {
    let normalised = normalise_origin(origin);
    let mut hasher = Sha256::new();
    hasher.update(normalised.as_bytes());
    let result = hasher.finalize();
    hex::encode(&result[..8])
}

fn blob_sha256(body: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(body);
    let result = hasher.finalize();
    hex::encode(result)
}

/// Session-isolated storage path builder.
///
/// This keeps all storage location derivation in one place and enforces
/// strict session id validation before generating paths.
#[derive(Debug, Clone)]
pub struct SessionStorageManager {
    base_dir: PathBuf,
}

impl SessionStorageManager {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    pub fn session_dir(&self, session_id: &str) -> Result<PathBuf, StorageError> {
        if !is_valid_session_id(session_id) {
            return Err(StorageError::InvalidSessionId(session_id.to_string()));
        }
        Ok(self.base_dir.join(session_id))
    }

    pub fn cookies_path(&self, session_id: &str) -> Result<PathBuf, StorageError> {
        Ok(self.session_dir(session_id)?.join("cookies.bin"))
    }

    pub fn manifest_path(&self, session_id: &str) -> Result<PathBuf, StorageError> {
        Ok(self.session_dir(session_id)?.join("manifest.json"))
    }

    pub fn save_cookies(
        &self,
        session_id: &str,
        store: &cookie_store::CookieStore,
    ) -> Result<(), StorageError> {
        let final_path = self.cookies_path(session_id)?;
        if let Some(parent) = final_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| StorageError::Io(e.to_string()))?;
        }

        let tmp_path = final_path.with_extension("bin.tmp");

        {
            let file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&tmp_path)
                .map_err(|e| StorageError::Io(e.to_string()))?;

            let mut writer = std::io::BufWriter::new(&file);
            serde_json::to_writer(&mut writer, store)
                .map_err(|e| StorageError::Serialise(e.to_string()))?;

            use std::io::Write;
            writer
                .flush()
                .map_err(|e| StorageError::Io(e.to_string()))?;
            file.sync_all()
                .map_err(|e| StorageError::Io(e.to_string()))?;
        }

        std::fs::rename(&tmp_path, &final_path).map_err(|e| StorageError::Io(e.to_string()))?;

        Ok(())
    }

    pub fn load_cookies(
        &self,
        session_id: &str,
    ) -> Result<cookie_store::CookieStore, StorageError> {
        let path = self.cookies_path(session_id)?;
        if !path.exists() {
            return Ok(cookie_store::CookieStore::default());
        }

        let file = std::fs::File::open(&path).map_err(|e| StorageError::Io(e.to_string()))?;
        let reader = std::io::BufReader::new(file);

        serde_json::from_reader::<_, cookie_store::CookieStore>(reader)
            .map_err(|e| StorageError::Serialise(format!("cookie deserialise: {}", e)))
    }

    pub fn delete_cookies(&self, session_id: &str) -> Result<(), StorageError> {
        let path = self.cookies_path(session_id)?;
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| StorageError::Io(e.to_string()))?;
        }
        Ok(())
    }

    pub fn create_session_dir(&self, session_id: &str) -> Result<PathBuf, StorageError> {
        std::fs::create_dir_all(&self.base_dir).map_err(|e| StorageError::Io(e.to_string()))?;
        let dir = self.session_dir(session_id)?;
        std::fs::create_dir_all(&dir).map_err(|e| StorageError::Io(e.to_string()))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
                .map_err(|e| StorageError::Io(e.to_string()))?;
        }

        let canonical = dir
            .canonicalize()
            .map_err(|e| StorageError::Io(e.to_string()))?;
        let base_canonical = self
            .base_dir
            .canonicalize()
            .map_err(|e| StorageError::Io(e.to_string()))?;

        if !canonical.starts_with(&base_canonical) {
            return Err(StorageError::PathTraversal(
                "Session dir is not under base_dir".to_string(),
            ));
        }

        Ok(canonical)
    }

    // localStorage
    pub fn localstorage_dir(&self, session_id: &str) -> Result<PathBuf, StorageError> {
        let dir = self.session_dir(session_id)?.join("localstorage");
        std::fs::create_dir_all(&dir).map_err(|e| StorageError::Io(e.to_string()))?;
        Ok(dir)
    }

    pub fn localstorage_db_path(
        &self,
        session_id: &str,
        origin: &str,
    ) -> Result<PathBuf, StorageError> {
        let dir = self.localstorage_dir(session_id)?;
        Ok(dir.join(format!("{}.sled", origin_hash(origin))))
    }

    fn open_localstorage_db(
        &self,
        session_id: &str,
        origin: &str,
    ) -> Result<sled::Db, StorageError> {
        let path = self.localstorage_db_path(session_id, origin)?;
        sled::open(path).map_err(|e| StorageError::Sled(e.to_string()))
    }

    pub fn local_storage_set(
        &self,
        session_id: &str,
        origin: &str,
        key: &str,
        value: &str,
    ) -> Result<(), StorageError> {
        let db = self.open_localstorage_db(session_id, origin)?;
        db.insert(key.as_bytes(), value.as_bytes())
            .map_err(|e| StorageError::Sled(e.to_string()))?;
        db.flush().map_err(|e| StorageError::Sled(e.to_string()))?;
        Ok(())
    }

    pub fn local_storage_get(
        &self,
        session_id: &str,
        origin: &str,
        key: &str,
    ) -> Result<Option<String>, StorageError> {
        let db = self.open_localstorage_db(session_id, origin)?;
        if let Some(ivec) = db
            .get(key.as_bytes())
            .map_err(|e| StorageError::Sled(e.to_string()))?
        {
            Ok(Some(String::from_utf8_lossy(&ivec).into_owned()))
        } else {
            Ok(None)
        }
    }

    pub fn local_storage_remove(
        &self,
        session_id: &str,
        origin: &str,
        key: &str,
    ) -> Result<(), StorageError> {
        let db = self.open_localstorage_db(session_id, origin)?;
        db.remove(key.as_bytes())
            .map_err(|e| StorageError::Sled(e.to_string()))?;
        db.flush().map_err(|e| StorageError::Sled(e.to_string()))?;
        Ok(())
    }

    pub fn local_storage_clear(&self, session_id: &str, origin: &str) -> Result<(), StorageError> {
        let db = self.open_localstorage_db(session_id, origin)?;
        db.clear().map_err(|e| StorageError::Sled(e.to_string()))?;
        db.flush().map_err(|e| StorageError::Sled(e.to_string()))?;
        Ok(())
    }

    pub fn local_storage_export(
        &self,
        session_id: &str,
        origin: &str,
    ) -> Result<HashMap<String, String>, StorageError> {
        let db = self.open_localstorage_db(session_id, origin)?;
        let mut map = HashMap::new();
        for result in db.iter() {
            let (k, v) = result.map_err(|e| StorageError::Sled(e.to_string()))?;
            map.insert(
                String::from_utf8_lossy(&k).into_owned(),
                String::from_utf8_lossy(&v).into_owned(),
            );
        }
        Ok(map)
    }

    pub fn local_storage_import(
        &self,
        session_id: &str,
        origin: &str,
        data: &HashMap<String, String>,
    ) -> Result<(), StorageError> {
        let db = self.open_localstorage_db(session_id, origin)?;
        db.clear().map_err(|e| StorageError::Sled(e.to_string()))?;
        for (k, v) in data {
            db.insert(k.as_bytes(), v.as_bytes())
                .map_err(|e| StorageError::Sled(e.to_string()))?;
        }
        db.flush().map_err(|e| StorageError::Sled(e.to_string()))?;
        Ok(())
    }

    // IndexedDB
    pub fn indexeddb_dir(&self, session_id: &str) -> Result<PathBuf, StorageError> {
        let dir = self.session_dir(session_id)?.join("indexeddb");
        std::fs::create_dir_all(&dir).map_err(|e| StorageError::Io(e.to_string()))?;
        Ok(dir)
    }

    pub fn indexeddb_db_path(
        &self,
        session_id: &str,
        origin: &str,
    ) -> Result<PathBuf, StorageError> {
        let dir = self.indexeddb_dir(session_id)?;
        Ok(dir.join(format!("{}.sqlite", origin_hash(origin))))
    }

    fn open_indexeddb(
        &self,
        session_id: &str,
        origin: &str,
    ) -> Result<rusqlite::Connection, StorageError> {
        let path = self.indexeddb_db_path(session_id, origin)?;
        let conn =
            rusqlite::Connection::open(&path).map_err(|e| StorageError::Sqlite(e.to_string()))?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             CREATE TABLE IF NOT EXISTS kv_store (
                 db_name TEXT NOT NULL,
                 store   TEXT NOT NULL,
                 key     TEXT NOT NULL,
                 value   TEXT NOT NULL,
                 PRIMARY KEY (db_name, store, key)
             );",
        )
        .map_err(|e| StorageError::Sqlite(e.to_string()))?;
        Ok(conn)
    }

    pub fn indexeddb_put(
        &self,
        session_id: &str,
        origin: &str,
        db_name: &str,
        store_name: &str,
        key: &str,
        value: &str,
    ) -> Result<(), StorageError> {
        let conn = self.open_indexeddb(session_id, origin)?;
        conn.execute(
            "INSERT OR REPLACE INTO kv_store (db_name, store, key, value) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![db_name, store_name, key, value],
        )
        .map_err(|e| StorageError::Sqlite(e.to_string()))?;
        Ok(())
    }

    pub fn indexeddb_get(
        &self,
        session_id: &str,
        origin: &str,
        db_name: &str,
        store_name: &str,
        key: &str,
    ) -> Result<Option<String>, StorageError> {
        let conn = self.open_indexeddb(session_id, origin)?;
        let mut stmt = conn
            .prepare("SELECT value FROM kv_store WHERE db_name = ?1 AND store = ?2 AND key = ?3")
            .map_err(|e| StorageError::Sqlite(e.to_string()))?;
        let result = stmt.query_row(rusqlite::params![db_name, store_name, key], |row| {
            row.get(0)
        });
        match result {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StorageError::Sqlite(e.to_string())),
        }
    }

    pub fn indexeddb_delete(
        &self,
        session_id: &str,
        origin: &str,
        db_name: &str,
        store_name: &str,
        key: &str,
    ) -> Result<(), StorageError> {
        let conn = self.open_indexeddb(session_id, origin)?;
        conn.execute(
            "DELETE FROM kv_store WHERE db_name = ?1 AND store = ?2 AND key = ?3",
            rusqlite::params![db_name, store_name, key],
        )
        .map_err(|e| StorageError::Sqlite(e.to_string()))?;
        Ok(())
    }

    pub fn indexeddb_list_keys(
        &self,
        session_id: &str,
        origin: &str,
        db_name: &str,
        store_name: &str,
    ) -> Result<Vec<String>, StorageError> {
        let conn = self.open_indexeddb(session_id, origin)?;
        let mut stmt = conn
            .prepare("SELECT key FROM kv_store WHERE db_name = ?1 AND store = ?2 ORDER BY key")
            .map_err(|e| StorageError::Sqlite(e.to_string()))?;
        let keys = stmt
            .query_map(rusqlite::params![db_name, store_name], |row| row.get(0))
            .map_err(|e| StorageError::Sqlite(e.to_string()))?
            .collect::<Result<Vec<String>, _>>()
            .map_err(|e| StorageError::Sqlite(e.to_string()))?;
        Ok(keys)
    }

    pub fn indexeddb_backup(
        &self,
        session_id: &str,
        origin: &str,
        dest_path: &Path,
    ) -> Result<(), StorageError> {
        let src = self.open_indexeddb(session_id, origin)?;
        let mut dst = rusqlite::Connection::open(dest_path)
            .map_err(|e| StorageError::Sqlite(e.to_string()))?;
        let backup = rusqlite::backup::Backup::new(&src, &mut dst)
            .map_err(|e| StorageError::Sqlite(e.to_string()))?;
        backup
            .run_to_completion(5, std::time::Duration::from_millis(250), None)
            .map_err(|e| StorageError::Sqlite(e.to_string()))?;
        Ok(())
    }

    // Cache API
    pub fn cache_blobs_dir(&self, session_id: &str) -> Result<PathBuf, StorageError> {
        let dir = self.session_dir(session_id)?.join("cache").join("blobs");
        std::fs::create_dir_all(&dir).map_err(|e| StorageError::Io(e.to_string()))?;
        Ok(dir)
    }

    pub fn cache_meta_path(&self, session_id: &str) -> Result<PathBuf, StorageError> {
        let dir = self.session_dir(session_id)?.join("cache");
        std::fs::create_dir_all(&dir).map_err(|e| StorageError::Io(e.to_string()))?;
        Ok(dir.join("meta.sled"))
    }

    fn open_cache_meta(&self, session_id: &str) -> Result<sled::Db, StorageError> {
        let path = self.cache_meta_path(session_id)?;
        sled::open(path).map_err(|e| StorageError::Sled(e.to_string()))
    }

    pub fn cache_put(
        &self,
        session_id: &str,
        cache_name: &str,
        request_url: &str,
        body: &[u8],
        headers_json: &str,
    ) -> Result<String, StorageError> {
        let key = blob_sha256(body);
        let blob_path = self.cache_blobs_dir(session_id)?.join(&key);
        if !blob_path.exists() {
            std::fs::write(&blob_path, body).map_err(|e| StorageError::Io(e.to_string()))?;
        }

        let meta = self.open_cache_meta(session_id)?;
        let meta_key = format!("{}|{}", cache_name, request_url).into_bytes();
        let meta_val = serde_json::json!({
            "blob_key": key,
            "headers": headers_json
        })
        .to_string();

        meta.insert(meta_key, meta_val.as_bytes())
            .map_err(|e| StorageError::Sled(e.to_string()))?;
        meta.flush()
            .map_err(|e| StorageError::Sled(e.to_string()))?;

        Ok(key)
    }

    pub fn cache_get(
        &self,
        session_id: &str,
        cache_name: &str,
        request_url: &str,
    ) -> Result<Option<Vec<u8>>, StorageError> {
        let meta = self.open_cache_meta(session_id)?;
        let meta_key = format!("{}|{}", cache_name, request_url).into_bytes();
        let raw = meta
            .get(meta_key)
            .map_err(|e| StorageError::Sled(e.to_string()))?;
        let meta_bytes = match raw {
            Some(b) => b,
            None => return Ok(None),
        };

        let meta_val: serde_json::Value = serde_json::from_slice(&meta_bytes)
            .map_err(|e| StorageError::Serialise(e.to_string()))?;

        let blob_key = meta_val
            .get("blob_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| StorageError::Serialise("missing blob_key in cache meta".to_string()))?;

        let blob_path = self.cache_blobs_dir(session_id)?.join(blob_key);
        let body = std::fs::read(&blob_path).map_err(|e| StorageError::Io(e.to_string()))?;

        Ok(Some(body))
    }
}

pub fn is_valid_session_id(session_id: &str) -> bool {
    if session_id.len() != 36 {
        return false;
    }
    for (i, ch) in session_id.chars().enumerate() {
        let is_hyphen = matches!(i, 8 | 13 | 18 | 23);
        if is_hyphen {
            if ch != '-' {
                return false;
            }
            continue;
        }
        if !ch.is_ascii_hexdigit() {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::{is_valid_session_id, SessionStorageManager, StorageError};

    const GOOD_ID: &str = "550e8400-e29b-41d4-a716-446655440000";

    #[test]
    fn validates_uuid_v4_shape() {
        assert!(is_valid_session_id(GOOD_ID));
        assert!(!is_valid_session_id("../../../etc/passwd"));
        assert!(!is_valid_session_id("not-a-uuid"));
    }

    #[test]
    fn builds_cookie_path() {
        let mgr = SessionStorageManager::new("/tmp/phantom-storage");
        let p = mgr.cookies_path(GOOD_ID).unwrap();
        assert!(p.ends_with("cookies.bin"));
    }

    #[test]
    fn rejects_invalid_session_for_paths() {
        let mgr = SessionStorageManager::new("/tmp/phantom-storage");
        let err = mgr.session_dir("bad").unwrap_err();
        assert!(matches!(err, StorageError::InvalidSessionId(_)));
    }
}
