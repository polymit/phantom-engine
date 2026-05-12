//! MCP Stdio transport for Phantom Engine.
//!
//! Implements the "Zero-Noise" NDJSON read loop per the MCP specification.
//! Every byte on stdout is a valid JSON-RPC 2.0 frame. All telemetry goes
//! to stderr via `telemetry::init_stdio()`.
//!
//! Compatible with: Claude Code, Codex CLI, Gemini CLI, Cline, Roo Code,
//! Devin, Goose, Crush, Warp AI, Kiro CLI, Qwen Code, OpenCode, Hermes,
//! Amp Code, Continue CLI, and any MCP-compliant host.

use std::io::Write;
use std::sync::Arc;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::OnceCell;

use crate::engine::{self, EngineAdapter};
use crate::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, McpServer};

// ---------------------------------------------------------------------------
// Lazy engine state — V8 and the adapter are created on first `tools/call`,
// keeping the `initialize` handshake under 100ms.
// ---------------------------------------------------------------------------

struct StdioState {
    adapter: OnceCell<Arc<EngineAdapter>>,
    server: OnceCell<McpServer>,
}

impl StdioState {
    fn new() -> Self {
        Self {
            adapter: OnceCell::new(),
            server: OnceCell::new(),
        }
    }

    /// Initialise V8 platform and create the EngineAdapter on first call.
    /// Subsequent calls return the cached instances immediately.
    async fn ensure_ready(&self) -> (&Arc<EngineAdapter>, &McpServer) {
        let adapter = self
            .adapter
            .get_or_init(|| async {
                // Idempotent via std::sync::Once inside engine::init_v8()
                engine::init_v8();

                let storage_dir = std::env::var("PHANTOM_STORAGE_DIR")
                    .unwrap_or_else(|_| "./storage".to_string());
                let qjs_pool = std::env::var("PHANTOM_QUICKJS_POOL_SIZE")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(5);
                let v8_pool = std::env::var("PHANTOM_V8_POOL_SIZE")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(5);
                let budget = phantom_session::ResourceBudget::default();

                tracing::info!("initialising engine (first tool call)");
                Arc::new(
                    EngineAdapter::new_with_storage(qjs_pool, 0, v8_pool, 0, budget, &storage_dir)
                        .await,
                )
            })
            .await;

        let server = self
            .server
            .get_or_init(|| async { McpServer::new_with_adapter(None, adapter.clone()) })
            .await;

        (adapter, server)
    }
}

// ---------------------------------------------------------------------------
// NDJSON writer — one line per frame, immediate flush (Rule 4 & 5)
// ---------------------------------------------------------------------------

/// Write a JSON-RPC response as a single NDJSON line, then flush stdout.
fn write_response(id: Value, result: Option<Value>, error: Option<JsonRpcError>) {
    let resp = JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result,
        error,
    };

    let line = match serde_json::to_string(&resp) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(err = %e, "failed to serialise response");
            return;
        }
    };

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();

    if let Err(e) = writeln!(handle, "{}", line) {
        tracing::error!(err = %e, "stdout write failed");
    }
    if let Err(e) = handle.flush() {
        tracing::error!(err = %e, "stdout flush failed");
    }
}

// ---------------------------------------------------------------------------
// Main event loop
// ---------------------------------------------------------------------------

/// Entry point for the stdio transport. Reads NDJSON from stdin, dispatches
/// MCP protocol methods, writes responses to stdout.
pub async fn run_stdio_loop() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let state = StdioState::new();
    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();

    tracing::info!("phantom-mcp stdio transport ready");

    loop {
        tokio::select! {
            line = lines.next_line() => {
                match line {
                    Ok(Some(ref text)) if !text.trim().is_empty() => {
                        dispatch(&state, text).await;
                    }
                    Ok(Some(_)) => continue, // blank line
                    Ok(None) => {
                        // stdin closed — host terminated the session
                        tracing::info!("stdin closed, shutting down");
                        break;
                    }
                    Err(e) => {
                        tracing::error!(err = %e, "stdin read error");
                        break;
                    }
                }
            }
            // Graceful shutdown on SIGTERM / SIGINT (Rule 6)
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("received shutdown signal");
                break;
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// MCP protocol dispatcher
// ---------------------------------------------------------------------------

async fn dispatch(state: &StdioState, line: &str) {
    let raw: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => {
            write_response(
                Value::Null,
                None,
                Some(JsonRpcError {
                    code: -32700,
                    message: format!("parse error: {e}"),
                }),
            );
            return;
        }
    };

    // Notifications carry no `id` — never respond to them.
    let is_notification = raw.get("id").is_none();
    let method = raw.get("method").and_then(|m| m.as_str()).unwrap_or("");

    if is_notification {
        tracing::debug!(method = %method, "notification (no response)");
        return;
    }

    let id = raw.get("id").cloned().unwrap_or(Value::Null);
    let params = raw.get("params").cloned().unwrap_or_else(|| json!({}));

    match method {
        "initialize" => handle_initialize(id, &params),
        "tools/list" => handle_tools_list(id),
        "tools/call" => handle_tools_call(state, id, &params).await,
        "ping" => write_response(id, Some(json!({})), None),
        other => {
            write_response(
                id,
                None,
                Some(JsonRpcError {
                    code: -32601,
                    message: format!("method not found: {other}"),
                }),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// MCP method handlers
// ---------------------------------------------------------------------------

fn handle_initialize(id: Value, params: &Value) {
    let client = params
        .get("clientInfo")
        .and_then(|c| c.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("unknown");

    tracing::info!(client = %client, "MCP initialize");

    write_response(
        id,
        Some(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": {
                "name": "phantom-engine",
                "version": env!("CARGO_PKG_VERSION")
            }
        })),
        None,
    );
}

fn handle_tools_list(id: Value) {
    write_response(id, Some(json!({ "tools": tool_definitions() })), None);
}

async fn handle_tools_call(state: &StdioState, id: Value, params: &Value) {
    let tool_name = match params.get("name").and_then(|n| n.as_str()) {
        Some(name) => name,
        None => {
            write_response(
                id,
                None,
                Some(JsonRpcError {
                    code: -32602,
                    message: "missing required field: name".to_string(),
                }),
            );
            return;
        }
    };

    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    // Lazy-init V8 and adapter on first tool call (Rule 3)
    let (adapter, server) = state.ensure_ready().await;

    // Translate MCP tools/call → internal JsonRpcRequest
    let internal_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: id.clone(),
        method: tool_name.to_string(),
        params: arguments,
    };

    // Dispatch through existing handler (no API key for stdio — Rule 6)
    match server.handle_request(adapter, internal_req, None).await {
        Ok(resp) => {
            let content = if let Some(result) = resp.result {
                let text = serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string());
                json!({
                    "content": [{ "type": "text", "text": text }],
                    "isError": false
                })
            } else if let Some(err) = resp.error {
                json!({
                    "content": [{ "type": "text", "text": err.message }],
                    "isError": true
                })
            } else {
                json!({
                    "content": [{ "type": "text", "text": "{}" }],
                    "isError": false
                })
            };
            write_response(id, Some(content), None);
        }
        Err(e) => {
            write_response(
                id,
                Some(json!({
                    "content": [{ "type": "text", "text": e.to_string() }],
                    "isError": true
                })),
                None,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Tool schema definitions for tools/list
// ---------------------------------------------------------------------------

fn tool_definitions() -> Value {
    json!([
        {
            "name": "browser_navigate",
            "description": "Navigate to a URL. Returns the page as a Compressed Content Tree (CCT).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "url":             { "type": "string", "description": "URL to navigate to" },
                    "viewport_width":  { "type": "number", "description": "Viewport width in px" },
                    "viewport_height": { "type": "number", "description": "Viewport height in px" },
                    "task_hint":       { "type": "string", "description": "Hint for selective serialisation" }
                },
                "required": ["url"]
            }
        },
        {
            "name": "browser_get_scene_graph",
            "description": "Re-serialise the current page as CCT without re-navigating.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "mode":      { "type": "string", "description": "'full' or 'selective'" },
                    "task_hint": { "type": "string", "description": "Hint for selective filtering" },
                    "scroll_x":  { "type": "number", "description": "Horizontal scroll offset" },
                    "scroll_y":  { "type": "number", "description": "Vertical scroll offset" }
                }
            }
        },
        {
            "name": "browser_click",
            "description": "Click an element with human-like mouse movement and timing.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "selector": { "type": "string", "description": "CSS selector" },
                    "x":        { "type": "number", "description": "Target X (defaults to center)" },
                    "y":        { "type": "number", "description": "Target Y (defaults to center)" }
                },
                "required": ["selector"]
            }
        },
        {
            "name": "browser_evaluate",
            "description": "Evaluate JavaScript in the current page context.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "script":     { "type": "string", "description": "JavaScript to evaluate" },
                    "timeout_ms": { "type": "number", "description": "Timeout in ms" }
                },
                "required": ["script"]
            }
        },
        {
            "name": "browser_type",
            "description": "Type text into an element with human-like keystroke timing.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "selector": { "type": "string", "description": "CSS selector of the input" },
                    "text":     { "type": "string", "description": "Text to type" },
                    "delay_ms": { "type": "number", "description": "Delay between keystrokes in ms" }
                },
                "required": ["selector", "text"]
            }
        },
        {
            "name": "browser_press_key",
            "description": "Press a keyboard key on the focused element.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "key": { "type": "string", "description": "Key name (Enter, Tab, Escape, etc.)" }
                },
                "required": ["key"]
            }
        },
        {
            "name": "browser_new_tab",
            "description": "Open a new browser tab, optionally navigating to a URL.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "URL for the new tab" }
                }
            }
        },
        {
            "name": "browser_switch_tab",
            "description": "Switch to a different browser tab by its ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "tab_id": { "type": "string", "description": "UUID of the tab" }
                },
                "required": ["tab_id"]
            }
        },
        {
            "name": "browser_list_tabs",
            "description": "List all open tabs with IDs, URLs, and titles.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "browser_close_tab",
            "description": "Close a browser tab by its ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "tab_id": { "type": "string", "description": "UUID of the tab to close" }
                },
                "required": ["tab_id"]
            }
        },
        {
            "name": "browser_get_cookies",
            "description": "Retrieve all cookies from the current session.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "browser_set_cookie",
            "description": "Set a cookie in the current session.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name":   { "type": "string", "description": "Cookie name" },
                    "value":  { "type": "string", "description": "Cookie value" },
                    "domain": { "type": "string", "description": "Cookie domain" },
                    "path":   { "type": "string", "description": "Cookie path" }
                },
                "required": ["name", "value"]
            }
        },
        {
            "name": "browser_clear_cookies",
            "description": "Clear all cookies from the current session.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "browser_session_snapshot",
            "description": "Create a compressed snapshot of the session (cookies, localStorage, IndexedDB, cache).",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "browser_session_clone",
            "description": "Clone the current session with copy-on-write semantics.",
            "inputSchema": { "type": "object", "properties": {} }
        }
    ])
}
