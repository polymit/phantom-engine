use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum StorageError {
    #[error("invalid session id: {0}")]
    InvalidSessionId(String),
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
