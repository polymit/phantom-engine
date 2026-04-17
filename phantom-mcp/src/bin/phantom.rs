use std::{env, sync::Arc};

use phantom_mcp::{telemetry, EngineAdapter, McpServer};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    telemetry::init();

    let bind_addr = env::var("PHANTOM_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let api_key =
        env::var("PHANTOM_API_KEY")
            .ok()
            .and_then(|v| if v.trim().is_empty() { None } else { Some(v) });

    let adapter = Arc::new(EngineAdapter::new_default().await);
    let server = McpServer::new_with_adapter(api_key, adapter);
    let app = server.router();

    let listener = TcpListener::bind(&bind_addr).await?;
    tracing::info!(bind_addr = %bind_addr, "phantom-mcp listening");

    axum::serve(listener, app).await?;
    Ok(())
}
