//! Structured logging and tracing initialisation for Phantom Engine.
//!
//! Call telemetry::init() once in main() before starting Tokio.
//! Call telemetry::init_test() in integration tests (idempotent).
//!
//! RUST_LOG controls log level: RUST_LOG=phantom=debug,tower_http=warn
//! LOG_FORMAT controls output: json (production) | pretty | compact (default)

use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let format = std::env::var("LOG_FORMAT").unwrap_or_else(|_| "compact".to_string());

    let base_subscriber = tracing_subscriber::registry().with(env_filter);

    match format.as_str() {
        "json" => {
            base_subscriber
                .with(fmt::layer().json().with_current_span(true))
                .init();
        }
        "pretty" => {
            base_subscriber.with(fmt::layer().pretty()).init();
        }
        _ => {
            base_subscriber.with(fmt::layer().compact()).init();
        }
    }
}

pub fn init_test() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"));

    let _ = tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt::layer().compact().with_test_writer())
        .try_init();
}
