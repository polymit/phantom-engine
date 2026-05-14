# Phantom Engine

**A real browser for AI agents.** Phantom Engine parses HTML, runs CSS layout, executes JavaScript, and persists full session state — all in Rust, exposed over JSON-RPC 2.0 so any MCP-compatible agent can drive it.

> **Alpha software.** Core flows work well; some sites may break.

---

## Quick Start

### Option A — Docker (HTTP mode)

```bash
docker run -d \
  -p 8080:8080 \
  -e PHANTOM_API_KEY=your-secret-key \
  -v phantom-data:/data \
  polymit/phantom-engine:latest

curl http://localhost:8080/health
```

Point your agent at `http://localhost:8080/rpc` with header `X-API-Key: your-secret-key`.

### Option B — Stdio mode (local agent)

Connect directly from Claude Code, Codex CLI, Cline, or any MCP-compatible host:

```bash
# Build the binary
cargo build --release --package phantom-mcp --bin phantom

# Claude Code
claude mcp add phantom -- ./target/release/phantom --stdio

# Codex CLI (~/.codex/config.toml)
# [mcp_servers.phantom]
# command = "./target/release/phantom"
# args = ["--stdio"]
```

No API key, no HTTP server, no ports. The host spawns Phantom as a child process and communicates via stdin/stdout.

### Option C — Phantom CLI (`ph`)

A native Rust CLI for engine management, diagnostics, and interactive DOM inspection.

```bash
cargo install --path phantom-cli
ph setup init       # bootstrap .env and API keys
ph up --background  # start the engine
ph navigate example.com
ph interactive      # REPL shell
```

See the [Phantom CLI documentation](phantom-cli/README.md) for the full command reference.

---

## What It Does

Most agent "browser tools" are thin wrappers around screenshots or raw HTML. Phantom Engine builds a real DOM, runs a CSS cascade, computes layout with Taffy, and serializes the result into a **Compressed Content Tree (CCT)** — a structured, token-efficient snapshot of exactly what is visible on the page.

- **Two-tier JS execution**: QuickJS for fast stateless evaluation, V8/Deno Core for heavy stateful work.
- **Full session persistence**: Cookies, localStorage, IndexedDB, and a Cache API — all on disk per session.
- **Session lifecycle**: Suspend, resume, and clone sessions (copy-on-write) without reloading.
- **Stealth networking**: All traffic goes through [http-quik](https://github.com/polymit/quik), a proprietary transport engine with Chrome 134 TLS and HTTP/2 fingerprint parity.

---

## MCP Tools

Every tool is available via JSON-RPC 2.0 (`POST /rpc` for HTTP, or `tools/call` for Stdio).

| Tool | Method | Description |
|:-----|:-------|:------------|
| Navigate | `browser_navigate` | Load a URL, parse DOM, compute layout, return CCT |
| Scene Graph | `browser_get_scene_graph` | Re-serialize the current page as CCT |
| Click | `browser_click` | Click an element by CSS selector with human-like movement |
| Type | `browser_type` | Type text into an element with realistic keystroke timing |
| Press Key | `browser_press_key` | Send a key event (Enter, Tab, Escape, etc.) |
| Evaluate | `browser_evaluate` | Run JavaScript in the page context |
| Get Cookies | `browser_get_cookies` | Read all session cookies |
| Set Cookie | `browser_set_cookie` | Write a cookie |
| Clear Cookies | `browser_clear_cookies` | Delete all session cookies |
| New Tab | `browser_new_tab` | Open a tab, optionally navigating to a URL |
| Switch Tab | `browser_switch_tab` | Activate a tab by UUID |
| List Tabs | `browser_list_tabs` | List all open tabs |
| Close Tab | `browser_close_tab` | Close a tab by UUID |
| Snapshot | `browser_session_snapshot` | Create a compressed archive of session state |
| Clone | `browser_session_clone` | Fork the session with copy-on-write semantics |

### Example

```json
{"jsonrpc":"2.0","id":1,"method":"browser_navigate","params":{"url":"https://example.com"}}
```
```json
{"jsonrpc":"2.0","id":2,"method":"browser_get_scene_graph","params":{"mode":"full"}}
```

The response contains a `cct` field — a line-per-node text format starting with `##PAGE`, including each visible element's bounds, role, text, and interactivity hints.

---

## Performance

| Operation | Phantom Engine | Headless Chrome |
|:----------|:--------------|:----------------|
| JS eval (simple expression) | 5.30 µs | ~5–15 ms (CDP roundtrip) |
| Tier 1 pool acquire | 345.86 µs | ~200–500 ms (tab creation) |
| CCT full serialization (1K nodes) | 4.67 ms | ~50–200 ms (CDP snapshot) |
| CCT selective serialization (1K nodes) | 4.69 ms | ~50–200 ms |
| Delta mutation (10 nodes) | 8.46 µs | not supported natively |
| QuickJS session allocation | 1.56 µs | ~500 ms–2 s (cold start) |
| V8 session allocation | 1.57 µs | ~500 ms–2 s (cold start) |
| Suspend / resume | 121.07 ns | not supported natively |

---

## Configuration

All configuration is via environment variables (loaded from `.env`).

| Variable | Default | Description |
|:---------|:--------|:------------|
| `PHANTOM_API_KEY` | *(required for HTTP)* | Bearer token for RPC requests |
| `PHANTOM_BIND_ADDR` | `0.0.0.0:8080` | HTTP listen address |
| `PHANTOM_STORAGE_DIR` | `./storage` | Session persistence directory |
| `PHANTOM_SESSION_LIMIT` | `1000` | Max concurrent sessions |
| `PHANTOM_RATE_LIMIT` | `100` | Requests per hour per key |
| `PHANTOM_QUICKJS_POOL_SIZE` | `5` | QuickJS runtime pool size |
| `PHANTOM_V8_POOL_SIZE` | `5` | V8 runtime pool size |
| `PHANTOM_LOG_FORMAT` | `compact` | `json`, `pretty`, or `compact` |
| `RUST_LOG` | `phantom=info` | Log level filter |

---

## Architecture

```
phantom-mcp             JSON-RPC server + Stdio transport, auth, metrics
├── phantom-net          Navigation orchestration and protocol negotiation
├── phantom-core         HTML parse → CSS cascade → Taffy layout → visibility
├── phantom-js           QuickJS (Tier 1) + V8/Deno (Tier 2) runtime pools
├── phantom-serializer   CCT serialization, delta diffs, selective mode
├── phantom-session      Session lifecycle, resource budgets, state machine
├── phantom-storage      Cookies, localStorage, IndexedDB, Cache API, snapshots
├── phantom-anti-detect  Browser persona pool, GPU profiles, timing
└── phantom-cli          Native CLI for management and debugging
```

> [!NOTE]
> **Transport Layer**: High-fidelity Chrome transport is handled by the standalone [http-quik](https://github.com/polymit/quik) engine. This decoupling ensures that the core engine remains focused on DOM/JS execution while benefiting from bit-perfect network identity parity.

---

## Docker Compose (with Prometheus + Grafana)

```bash
cp .env.example .env
docker compose up -d
```

| Service | URL |
|:--------|:----|
| Engine | http://localhost:8080 |
| Metrics | http://localhost:8080/metrics |
| Prometheus | http://localhost:9090 |
| Grafana | http://localhost:3000 |

---

## Observability

Prometheus metrics are exposed at `/metrics`.

| Metric | Description |
|:-------|:------------|
| `sessions_created_total` | Total sessions opened since startup |
| `sessions_active` | Currently live sessions |
| `phantom_circuit_breaker_state` | Runtime pool health (0 = healthy) |
| `storage_quota_used_bytes` | Bytes written to session storage |
| `http_request_duration_seconds` | RPC handler latency by tool |

Alert rules ship in `prometheus/rules/phantom.yml`.

---

## Building from Source

Requires Rust 1.94+ and system packages: `pkg-config libssl-dev libsqlite3-dev clang cmake`.

```bash
git clone https://github.com/polymit/phantom-engine
cd phantom-engine
cargo build --release --package phantom-mcp --bin phantom
./target/release/phantom          # HTTP mode
./target/release/phantom --stdio  # Stdio mode
```

Run the test suite:

```bash
cargo test --workspace -- --test-threads=1
```

---

## License

[Apache 2.0](LICENSE.md)

---

Contributions welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request.
