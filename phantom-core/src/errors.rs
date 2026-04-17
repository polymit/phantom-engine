use std::io;

pub const JS_EVAL_TIMEOUT_MS: u64 = 10_000;
pub const QUICKJS_HEAP_LIMIT_BYTES: usize = 50 * 1024 * 1024;
pub const V8_HEAP_LIMIT_BYTES: usize = 512 * 1024 * 1024;
pub const RUNTIME_MAX_AGE_SECS: u64 = 300;
pub const DEFAULT_SESSIONS_PER_HOUR: u32 = 100;

#[derive(thiserror::Error, Debug)]
pub enum NetworkError {
    #[error("DNS resolution failed for {host}")]
    Dns {
        host: String,
        #[source]
        source: io::Error,
    },

    #[error("TLS handshake failed: {0}")]
    Tls(String),

    #[error("request timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("HTTP error {status}")]
    Http { status: u16, body: Option<String> },

    #[error("connection refused: {0}")]
    ConnectionRefused(String),
}

#[derive(thiserror::Error, Debug)]
pub enum DomError {
    #[error("element not found: '{selector}'")]
    ElementNotFound { selector: String },

    #[error("stale element reference: '{selector}'")]
    StaleElement { selector: String },

    #[error("invalid selector: {0}")]
    InvalidSelector(String),

    #[error("not interactable: {reason}")]
    NotInteractable { reason: String, selector: String },
}

#[derive(thiserror::Error, Debug)]
pub enum JsError {
    #[error("uncaught exception: {message}\nstack: {stack}")]
    UncaughtException { message: String, stack: String },

    #[error("script timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("JavaScript heap OOM")]
    OutOfMemory,
}

#[derive(thiserror::Error, Debug)]
pub enum NavigationError {
    #[error("page load failed for {url}: HTTP {status}")]
    LoadFailed { url: String, status: u16 },

    #[error("navigation timeout for {url} after {timeout_ms}ms")]
    Timeout { url: String, timeout_ms: u64 },

    #[error("too many redirects for {url}: {count} redirects (last: {location})")]
    TooManyRedirects {
        url: String,
        location: String,
        count: u32,
    },
}

#[derive(thiserror::Error, Debug)]
pub enum BrowserSessionError {
    #[error("session expired: {session_id}")]
    Expired { session_id: String },

    #[error("budget exceeded: {resource} {used}/{limit}")]
    BudgetExceeded {
        resource: String,
        used: u64,
        limit: u64,
    },

    #[error("tab not found: {tab_id}")]
    TabNotFound { tab_id: String },
}

#[derive(thiserror::Error, Debug)]
pub enum InternalError {
    #[error("runtime pool exhausted (max {max})")]
    RuntimePoolExhausted { max: usize },

    #[error("engine panicked: {0}")]
    Panic(String),
}

#[derive(thiserror::Error, Debug)]
pub enum BrowserError {
    #[error("network error: {0}")]
    Network(#[from] NetworkError),

    #[error("DOM error: {0}")]
    Dom(#[from] DomError),

    #[error("JavaScript error: {0}")]
    JavaScript(#[from] JsError),

    #[error("navigation error: {0}")]
    Navigation(#[from] NavigationError),

    #[error("session error: {0}")]
    Session(#[from] BrowserSessionError),

    #[error("internal error: {0}")]
    Internal(#[from] InternalError),
}

impl From<String> for JsError {
    fn from(msg: String) -> Self {
        JsError::UncaughtException {
            message: msg,
            stack: String::new(),
        }
    }
}

impl From<&str> for JsError {
    fn from(msg: &str) -> Self {
        JsError::UncaughtException {
            message: msg.to_string(),
            stack: String::new(),
        }
    }
}

impl From<phantom_storage::StorageError> for BrowserError {
    fn from(err: phantom_storage::StorageError) -> Self {
        BrowserError::Internal(InternalError::Panic(err.to_string()))
    }
}

impl From<phantom_session::SessionError> for BrowserSessionError {
    fn from(err: phantom_session::SessionError) -> Self {
        match err {
            phantom_session::SessionError::NotFound(id) => BrowserSessionError::Expired {
                session_id: id.to_string(),
            },
            phantom_session::SessionError::BudgetExceeded {
                resource,
                used,
                limit,
            } => BrowserSessionError::BudgetExceeded {
                resource,
                used,
                limit,
            },
        }
    }
}

impl From<phantom_session::SessionError> for BrowserError {
    fn from(err: phantom_session::SessionError) -> Self {
        BrowserError::Session(err.into())
    }
}
