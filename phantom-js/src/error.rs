use thiserror::Error;

#[derive(Error, Debug)]
pub enum PhantomJsError {
    // Tier 1 — QuickJS errors
    #[error("QuickJS runtime creation failed: {0}")]
    QuickJsRuntime(String),

    #[error("QuickJS context creation failed: {0}")]
    QuickJsContext(String),

    #[error("JavaScript evaluation failed: {0}")]
    JsEvaluation(String),

    #[error("JavaScript execution timed out after {timeout_ms}ms")]
    JsTimeout { timeout_ms: u64 },

    #[error("JavaScript heap out of memory")]
    JsOutOfMemory,

    #[error("JS-DOM binding error: {0}")]
    DomBinding(String),

    // Tier 2 — V8 errors
    #[error("V8 snapshot creation failed: {0}")]
    SnapshotCreation(String),

    #[error("V8 session creation failed: {0}")]
    V8Session(String),

    // Pool errors
    #[error("Runtime pool exhausted — max {max} sessions reached")]
    PoolExhausted { max: usize },

    #[error("Runtime pool acquire timed out after {timeout_ms}ms")]
    PoolTimeout { timeout_ms: u64 },

    // Shim errors
    #[error("Browser shim injection failed: {0}")]
    ShimInjection(String),

    // Internal
    #[error("Internal phantom-js error: {0}")]
    Internal(String),
}
