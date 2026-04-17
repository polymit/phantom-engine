#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::sync::atomic::Ordering;
use std::sync::Arc;

use phantom_core::{BrowserError, BrowserSessionError};
use phantom_mcp::{engine, EngineAdapter};
use phantom_session::ResourceBudget;

#[tokio::test]
async fn budget_exceeded_destroys_session() {
    engine::init_v8();
    let budget = ResourceBudget {
        max_heap_bytes: 8,
        max_cpu_ms_per_sec: 10_000,
        max_network_bytes: 8,
    };
    let adapter = Arc::new(EngineAdapter::new(1, 0, 1, 0, budget).await);

    let err = adapter
        .enforce_budget_usage(16, 0, 0)
        .expect_err("budget should be exceeded");
    assert!(matches!(
        err,
        BrowserError::Session(BrowserSessionError::BudgetExceeded { .. })
    ));
    assert!(adapter.broker.get(adapter.session_uuid).is_err());
    assert!(!adapter.session_active.load(Ordering::Acquire));
}

#[tokio::test]
async fn budget_exceeded_returns_typed_error_not_string() {
    engine::init_v8();
    let budget = ResourceBudget {
        max_heap_bytes: 1024,
        max_cpu_ms_per_sec: 1,
        max_network_bytes: 1024,
    };
    let adapter = Arc::new(EngineAdapter::new(1, 0, 1, 0, budget).await);

    let err = adapter
        .enforce_budget_usage(0, 2, 0)
        .expect_err("cpu budget should be exceeded");
    match err {
        BrowserError::Session(BrowserSessionError::BudgetExceeded {
            resource,
            used,
            limit,
        }) => {
            assert_eq!(resource, "cpu_ms_per_sec");
            assert!(used > limit);
        }
        other => panic!("expected typed budget error, got: {other}"),
    }
}
