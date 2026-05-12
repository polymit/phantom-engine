use std::{env, sync::Arc};

use phantom_mcp::{telemetry, EngineAdapter, McpServer};
use tokio::net::TcpListener;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stdio_mode = env::args().any(|a| a == "--stdio");

    if stdio_mode {
        // Stdio mode: V8 initialisation is deferred until the first
        // tools/call arrives (see stdio::StdioState::ensure_ready).
        // This keeps the `initialize` handshake under 100ms.
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;

        // Telemetry must go to stderr — stdout is reserved for JSON-RPC.
        rt.block_on(async {
            telemetry::init_stdio();
            phantom_mcp::stdio::run_stdio_loop().await
        })
    } else {
        // HTTP mode: initialise V8 eagerly on the main thread (Blueprint D-38).
        phantom_mcp::engine::init_v8();

        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;

        rt.block_on(run_http())
    }
}

/// HTTP/SSE server — the original transport mode.
async fn run_http() -> Result<(), Box<dyn std::error::Error>> {
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
    let v8_pool_size = env::var("PHANTOM_V8_POOL_SIZE")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(5);
    let storage_dir = env::var("PHANTOM_STORAGE_DIR").unwrap_or_else(|_| "./storage".to_string());

    let budget = phantom_session::ResourceBudget {
        max_cpu_ms_per_sec: cpu_quota_ms,
        ..Default::default()
    };

    let adapter = Arc::new(
        EngineAdapter::new_with_storage(
            quickjs_pool_size,
            0,
            v8_pool_size,
            0,
            budget,
            &storage_dir,
        )
        .await,
    );
    let server = McpServer::new_with_adapter_full(api_key, adapter, rate_limit, session_limit);
    let app = server.router();

    let listener = TcpListener::bind(&bind_addr).await?;
    tracing::info!(bind_addr = %bind_addr, "phantom-mcp listening");

    axum::serve(listener, app).await?;
    Ok(())
}
