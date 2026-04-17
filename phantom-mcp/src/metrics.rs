#![allow(clippy::expect_used)]
//! Prometheus metrics for Phantom Engine.
//!
//! All 13 metrics use a dedicated PHANTOM_REGISTRY to avoid polluting
//! the global default registry. Grafana dashboards depend on exact metric
//! names — do not rename without updating alert rules.

use once_cell::sync::Lazy;
use prometheus::{
    register_counter_vec_with_registry, register_histogram_vec_with_registry,
    register_histogram_with_registry, register_int_gauge_vec_with_registry,
    register_int_gauge_with_registry, CounterVec, Encoder, Histogram, HistogramOpts, HistogramVec,
    IntGauge, IntGaugeVec, Opts, Registry, TextEncoder,
};
use std::sync::Arc;

pub static PHANTOM_REGISTRY: Lazy<Arc<Registry>> = Lazy::new(|| Arc::new(Registry::new()));

// 1. sessions_active — Gauge — current live browser sessions
pub static SESSIONS_ACTIVE: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge_with_registry!(
        Opts::new("sessions_active", "Current live browser sessions"),
        PHANTOM_REGISTRY.as_ref()
    )
    .expect("sessions_active registration failed")
});

// 2. sessions_created_total — Counter — label: engine_tier
pub static SESSIONS_CREATED_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec_with_registry!(
        Opts::new("sessions_created_total", "Total sessions created"),
        &["engine_tier"],
        PHANTOM_REGISTRY.as_ref()
    )
    .expect("sessions_created_total registration failed")
});

// 3. session_duration_seconds — Histogram — session lifetime
pub static SESSION_DURATION_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram_with_registry!(
        HistogramOpts::new("session_duration_seconds", "Session lifetime in seconds")
            .buckets(vec![1.0, 5.0, 30.0, 60.0, 300.0, 900.0, 1800.0, 3600.0]),
        PHANTOM_REGISTRY.as_ref()
    )
    .expect("session_duration_seconds registration failed")
});

// 4. js_runtimes_used — Gauge — JS runtimes currently checked out
pub static JS_RUNTIMES_USED: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge_with_registry!(
        Opts::new("js_runtimes_used", "JS runtimes currently in use"),
        PHANTOM_REGISTRY.as_ref()
    )
    .expect("js_runtimes_used registration failed")
});

// 5. js_evaluation_duration_seconds — Histogram — label: tier
pub static JS_EVALUATION_DURATION_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec_with_registry!(
        HistogramOpts::new("js_evaluation_duration_seconds", "JS evaluation time")
            .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0]),
        &["tier"],
        PHANTOM_REGISTRY.as_ref()
    )
    .expect("js_evaluation_duration_seconds registration failed")
});

// 6. http_requests_total — Counter — label: status_code
pub static HTTP_REQUESTS_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec_with_registry!(
        Opts::new("http_requests_total", "HTTP requests made by engine"),
        &["status_code"],
        PHANTOM_REGISTRY.as_ref()
    )
    .expect("http_requests_total registration failed")
});

// 7. http_request_duration_seconds — Histogram
pub static HTTP_REQUEST_DURATION_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram_with_registry!(
        HistogramOpts::new("http_request_duration_seconds", "HTTP request latency")
            .buckets(vec![0.001, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0, 30.0]),
        PHANTOM_REGISTRY.as_ref()
    )
    .expect("http_request_duration_seconds registration failed")
});

// 8. dom_snapshot_duration_seconds — Histogram
pub static DOM_SNAPSHOT_DURATION_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram_with_registry!(
        HistogramOpts::new("dom_snapshot_duration_seconds", "CCT serialisation time")
            .buckets(vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1]),
        PHANTOM_REGISTRY.as_ref()
    )
    .expect("dom_snapshot_duration_seconds registration failed")
});

// 9. dom_nodes_serialised — Histogram
pub static DOM_NODES_SERIALISED: Lazy<Histogram> = Lazy::new(|| {
    register_histogram_with_registry!(
        HistogramOpts::new("dom_nodes_serialised", "Node count per CCT call")
            .buckets(vec![10.0, 50.0, 100.0, 500.0, 1000.0, 5000.0, 10000.0]),
        PHANTOM_REGISTRY.as_ref()
    )
    .expect("dom_nodes_serialised registration failed")
});

// 10. storage_quota_used_bytes — Gauge
pub static STORAGE_QUOTA_USED_BYTES: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge_with_registry!(
        Opts::new(
            "storage_quota_used_bytes",
            "Storage used by active sessions"
        ),
        PHANTOM_REGISTRY.as_ref()
    )
    .expect("storage_quota_used_bytes registration failed")
});

// 11. circuit_breaker_state — Gauge (0=Closed, 1=Open, 2=HalfOpen), label: pool
pub static CIRCUIT_BREAKER_STATE: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec_with_registry!(
        Opts::new(
            "phantom_circuit_breaker_state",
            "Circuit breaker state (0=Closed, 1=Open, 2=HalfOpen)"
        ),
        &["pool"],
        PHANTOM_REGISTRY.as_ref()
    )
    .expect("circuit_breaker_state registration failed")
});

// 12. circuit_breaker_trips_total — Counter, label: pool
pub static CIRCUIT_BREAKER_TRIPS_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec_with_registry!(
        Opts::new(
            "phantom_circuit_breaker_trips_total",
            "Total circuit breaker trips"
        ),
        &["pool"],
        PHANTOM_REGISTRY.as_ref()
    )
    .expect("circuit_breaker_trips_total registration failed")
});

// 13. circuit_breaker_resets_total — Counter, label: pool
pub static CIRCUIT_BREAKER_RESETS_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec_with_registry!(
        Opts::new(
            "phantom_circuit_breaker_resets_total",
            "Total circuit breaker resets"
        ),
        &["pool"],
        PHANTOM_REGISTRY.as_ref()
    )
    .expect("circuit_breaker_resets_total registration failed")
});

fn ensure_registered() {
    Lazy::force(&SESSIONS_ACTIVE);
    Lazy::force(&SESSIONS_CREATED_TOTAL);
    Lazy::force(&SESSION_DURATION_SECONDS);
    Lazy::force(&JS_RUNTIMES_USED);
    Lazy::force(&JS_EVALUATION_DURATION_SECONDS);
    Lazy::force(&HTTP_REQUESTS_TOTAL);
    Lazy::force(&HTTP_REQUEST_DURATION_SECONDS);
    Lazy::force(&DOM_SNAPSHOT_DURATION_SECONDS);
    Lazy::force(&DOM_NODES_SERIALISED);
    Lazy::force(&STORAGE_QUOTA_USED_BYTES);
    Lazy::force(&CIRCUIT_BREAKER_STATE);
    Lazy::force(&CIRCUIT_BREAKER_TRIPS_TOTAL);
    Lazy::force(&CIRCUIT_BREAKER_RESETS_TOTAL);

    // Vec metrics only emit samples once a label set is materialized.
    SESSIONS_CREATED_TOTAL.with_label_values(&["tier1"]);
    SESSIONS_CREATED_TOTAL.with_label_values(&["tier2"]);
    JS_EVALUATION_DURATION_SECONDS.with_label_values(&["tier1"]);
    JS_EVALUATION_DURATION_SECONDS.with_label_values(&["tier2"]);
    HTTP_REQUESTS_TOTAL.with_label_values(&["200"]);
    CIRCUIT_BREAKER_STATE.with_label_values(&["tier1"]).set(0);
    CIRCUIT_BREAKER_STATE.with_label_values(&["tier2"]).set(0);
    CIRCUIT_BREAKER_TRIPS_TOTAL.with_label_values(&["tier1"]);
    CIRCUIT_BREAKER_TRIPS_TOTAL.with_label_values(&["tier2"]);
    CIRCUIT_BREAKER_RESETS_TOTAL.with_label_values(&["tier1"]);
    CIRCUIT_BREAKER_RESETS_TOTAL.with_label_values(&["tier2"]);
}

/// Encode all registered metrics as Prometheus text exposition format.
pub fn metrics_text() -> Vec<u8> {
    ensure_registered();
    let mut buffer = Vec::new();
    TextEncoder::new()
        .encode(&PHANTOM_REGISTRY.gather(), &mut buffer)
        .expect("metrics encoding failed");
    buffer
}
