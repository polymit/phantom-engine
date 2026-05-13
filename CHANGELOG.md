# Release Notes — v0.2.2-alpha

This release introduces the production-ready Stdio MCP Transport to enable seamless integration with agentic frameworks like Codex and Claude Code. It also addresses critical agent-engine friction points, resolving catastrophic session invalidation loops and a major zero-node rendering bug on complex websites like Wikipedia.

## New Features

### Stdio MCP Transport Integration
Implemented an asynchronous NDJSON read/write loop to support the Stdio Model Context Protocol. A new `--stdio` CLI flag bypasses the HTTP/SSE server binding entirely. To ensure sub-500ms startup handshakes required by MCP hosts, V8 and QuickJS engines are now lazily initialized upon the first tool execution.

## Bug Fixes

### Zero-Node Scene Graph on Websites
The `phantom-serializer` visibility pipeline contained a catastrophic cascading bug. If a parent element had a bounding height or width of `0` (common for absolute wrappers or un-cleared floats), it was marked invisible and incorrectly cascaded that invisibility to all children, resulting in an empty DOM tree. The logic in `visibility.rs` is now decoupled: a parent only hides its children if it explicitly has `display: none` or `opacity: 0`.

### Cascading Session Invalidation Loop (Session Expired)
When agents injected heavy discovery scripts that exhausted the QuickJS heap (`js_out_of_memory`) or exceeded the CPU budget, the `EngineAdapter` aggressively destroyed the entire browser session via `broker.remove()`. This trapped agents in a permanent "Session Expired" loop. The fail-fast logic in `engine.rs` and `evaluate.rs` was removed; the `Tier1Pool` now isolates and discards the dirty QuickJS runtime while keeping the core `Session` (URL, cookies, CCT) fully intact.

### Storage Permission Denied (os error 13)
The `PHANTOM_STORAGE_DIR` environment variable previously defaulted to `/data/storage`, an absolute path restricted by Linux user permissions. This caused `browser_session_clone` and sqlite initialization to fail immediately. The `.env.example` default is now set to `./storage`, safely isolating data within the project directory.

### The Agent-Engine Trust Gap
General-purpose agents treated Phantom as a generic browser, injecting heavy JavaScript via `browser_evaluate` to discover elements, bypassing the engine's token-efficient CCT. The tool definitions in `stdio.rs` have been completely overhauled. `browser_get_scene_graph` is now explicitly labeled as the "SOURCE OF TRUTH", commanding agents to trust the natively computed layout data and explicitly warning against using `browser_evaluate` for DOM discovery.

## Affected Files

- `phantom-mcp/src/stdio.rs` — implemented Stdio transport and overhauled tool descriptions.
- `phantom-mcp/src/bin/phantom.rs` — added `--stdio` CLI flag routing.
- `phantom-mcp/src/engine.rs` — removed session destruction on resource budget exhaustion.
- `phantom-mcp/src/tools/evaluate.rs` — removed session destruction on QuickJS timeouts and OOMs.
- `phantom-serializer/src/visibility.rs` — fixed 0-height parent visibility cascading.
- `.env.example` — updated default storage directory path.

## Upgrade Notes

No direct action is required for existing integrations, but agents using the Phantom MCP server will automatically benefit from the improved tool descriptions upon restarting their connection. If you are experiencing `os error 13`, ensure your local `.env` contains `PHANTOM_STORAGE_DIR=./storage`.
