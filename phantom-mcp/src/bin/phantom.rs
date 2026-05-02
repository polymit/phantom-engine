use std::{env, sync::Arc};

use phantom_mcp::{telemetry, EngineAdapter, McpServer};
use tokio::net::TcpListener;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize V8 Platform on the MAIN OS THREAD (Blueprint D-38)
    // This MUST happen before any worker threads (Tokio) are spawned.
    phantom_js::init_v8_platform();

    // 2. Build and run the Tokio runtime
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(run())
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    telemetry::init();

    let bind_addr = env::var("PHANTOM_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let api_key =
        env::var("PHANTOM_API_KEY")
            .ok()
            .and_then(|v| if v.trim().is_empty() { None } else { Some(v) });

    let rate_limit = env::var("PHANTOM_RATE_LIMIT")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(100);
    let session_limit = env::var("PHANTOM_SESSION_LIMIT")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(1000);

    let cpu_quota_ms = env::var("PHANTOM_CPU_QUOTA_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(1000);
    let quickjs_pool_size = env::var("PHANTOM_QUICKJS_POOL_SIZE")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(5);

    let budget = phantom_session::ResourceBudget {
        max_cpu_ms_per_sec: cpu_quota_ms,
        ..Default::default()
    };

    let adapter = Arc::new(EngineAdapter::new(quickjs_pool_size, 0, 5, 0, budget).await);
    let server = McpServer::new_with_adapter_full(api_key, adapter, rate_limit, session_limit);
    let app = server.router();

    let listener = TcpListener::bind(&bind_addr).await?;
    tracing::info!(bind_addr = %bind_addr, "phantom-mcp listening");

    axum::serve(listener, app).await?;
    Ok(())
}
