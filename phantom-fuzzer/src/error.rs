use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

/// Result type used by the fuzzer crate.
pub type Result<T> = std::result::Result<T, FuzzerError>;

/// Errors returned by the fuzzer planner and corpus code.
#[derive(Debug, Error)]
pub enum FuzzerError {
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("corpus has no seeds")]
    EmptyCorpus,
    #[error("seed label must not be empty")]
    EmptyLabel,
    #[error("invalid seed: {0}")]
    BadSeed(String),
    #[error("document serialisation failed: {0}")]
    Serialize(String),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

impl FuzzerError {
    pub(crate) fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}

pub(crate) fn map_io<T>(path: &Path, res: io::Result<T>) -> Result<T> {
    res.map_err(|source| FuzzerError::io(path.to_path_buf(), source))
}
