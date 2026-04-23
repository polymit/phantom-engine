# Phantom Engine

**Phantom Engine gives agents a real browser.** Written in Rust for high performance — HTML parsing, CSS layout, JavaScript execution, and a full storage layer — delivered over a local JSON-RPC server that any MCP-compatible agent can call.

> **Alpha software.** Core flows work well; some sites may break.

---

## What it does

Most "browser tools" for agents are thin wrappers around screenshots or raw HTML dumps. Phantom Engine is different. It builds a real DOM, runs a CSS cascade, computes layout with Taffy, and serializes the result into a **Compressed Content Tree (CCT)** — a structured, token-efficient snapshot of exactly what is visible on the page, with coordinates, roles, and interactivity hints attached.

JavaScript runs in a two-tier pool: QuickJS for fast stateless evaluation, V8/Deno Core for heavy stateful work. Cookies, localStorage, IndexedDB, and a Cache API are all persisted per session on disk. Sessions can be suspended, resumed, and cloned (copy-on-write) without reloading the page.

The entire thing speaks **JSON-RPC 2.0** over HTTP, so any agent framework that supports MCP can use it without a custom client.

---

## The Phantom CLI (`ph`)

We now ship a native Rust CLI to make engine management and debugging effortless. Use it for setup, diagnostics, and interactive DOM inspection.

```bash
cargo install --path phantom-cli
ph setup init
ph interactive
```

See the [Phantom CLI README](phantom-cli/README.md) for full documentation.

---

## Get started in 60 seconds

```bash
# Pull and run
docker run -d \
  -p 8080:8080 \
  -e PHANTOM_API_KEY=your-secret-key \
  -v phantom-data:/data \
  polymit/phantom:latest

# Health check
curl http://localhost:8080/health
```

Point your MCP client at `http://localhost:8080/rpc` with header `X-API-Key: your-secret-key`.

---

## Performance

| Operation | Phantom Engine | Headless Chrome |
|-----------|---------------|-----------------|
| JS eval (simple expression) | 5.30 µs | ~5–15 ms (CDP roundtrip) |
| Tier 1 pool acquire | 345.86 µs | ~200–500 ms (tab creation) |
| CCT full serialization (1,000 nodes) | 4.67 ms | ~50–200 ms (CDP snapshot) |
| CCT selective serialization (1,000 nodes) | 4.69 ms | ~50–200 ms |
| Delta mutation (10 nodes) | 8.46 µs | not supported natively |
| QuickJS session creation | 1.56 µs | ~500 ms–2 s (cold start) |
| V8 session creation | 1.57 µs | ~500 ms–2 s (cold start) |
| Suspend / resume | 121.07 ns | not supported natively |

---

## Docker Compose (with Prometheus + Grafana)

```bash
cp .env.example .env          # fill in PHANTOM_API_KEY
docker compose up -d
```

| Service    | URL                       |
|------------|---------------------------|
| Engine     | http://localhost:8080     |
| Metrics    | http://localhost:8080/metrics |
| Prometheus | http://localhost:9090     |
| Grafana    | http://localhost:3000     |

---

## MCP Tools

Every tool is called as a JSON-RPC 2.0 method on `POST /rpc`.

| Tool | Method | What it does |
|------|--------|--------------|
| Navigate | `browser_navigate` | Loads a URL, fetches external CSS, parses the DOM, computes layout |
| Scene Graph | `browser_get_scene_graph` | Returns the CCT — the structured visible snapshot of the current page |
| Click | `browser_click` | Clicks an element by CSS selector |
| Type | `browser_type_text` | Types text into a focused element |
| Press Key | `browser_press_key` | Sends a key event (Enter, Tab, Escape, etc.) |
| Evaluate | `browser_evaluate` | Runs arbitrary JavaScript in the page context |
| Snapshot | `browser_snapshot` | Creates a portable, HMAC-signed snapshot of session state |
| Cookies | `browser_get_cookies` / `browser_set_cookies` | Reads or writes cookies for the current session |
| Tabs | `browser_new_tab` / `browser_switch_tab` / `browser_close_tab` | Multi-tab session management |
| Clone Session | `browser_clone_session` | Forks the current session (copy-on-write) |
| Subscribe | `browser_subscribe` | Streams DOM mutation events |

### Example: navigate and read the page

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "browser_navigate",
  "params": { "url": "https://example.com" }
}
```

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "browser_get_scene_graph",
  "params": { "mode": "full" }
}
```

The scene graph response contains a `cct` field — a line-per-node text format that starts with `##PAGE` and includes each visible element's bounds, role, text, and interactivity.

---

## Configuration

All configuration is via environment variables.

| Variable | Default | Description |
|----------|---------|-------------|
| `PHANTOM_API_KEY` | *(required)* | Bearer token for all RPC requests |
| `PHANTOM_BIND_ADDR` | `0.0.0.0:8080` | Listen address |
| `PHANTOM_STORAGE_DIR` | `/data/storage` | Session persistence root |
| `PHANTOM_SESSION_LIMIT` | `1000` | Max concurrent sessions |
| `PHANTOM_RATE_LIMIT` | `100` | Requests per minute per key |
| `PHANTOM_QUICKJS_POOL_SIZE` | `10` | QuickJS runtime pool size |
| `PHANTOM_V8_POOL_SIZE` | `5` | V8 runtime pool size |
| `PHANTOM_LOG_FORMAT` | `json` | `json` or `pretty` |
| `RUST_LOG` | `phantom=info` | Log level filter |

---

## Architecture

```
phantom-mcp          JSON-RPC server, auth, metrics, session routing
├── phantom-net      HTTP transport (wreq), TLS fingerprinting, Alt-Svc/H3
├── phantom-core     HTML parse → CSS cascade → Taffy layout → visibility
├── phantom-js       QuickJS (Tier 1) + V8/Deno (Tier 2) runtime pools
├── phantom-serializer   CCT serialization, delta diffs, selective mode
├── phantom-session  Session lifecycle, resource budgets, state machine
├── phantom-storage  Cookies, localStorage, IndexedDB, Cache API, snapshots
└── phantom-anti-detect  Browser persona pool, GPU profiles, timing
```

Network requests use [wreq](https://github.com/0x676e67/wreq) — a Chrome-emulating HTTP client with correct TLS fingerprints and JA3/JA4 signatures. The default persona pool ships with five real-browser profiles across Chrome 133 and 134 on Windows and macOS.

---

## Building from source

Requires Rust 1.94+ and system packages: `pkg-config libssl-dev libsqlite3-dev clang cmake`.

```bash
git clone https://github.com/polymit/phantom-engine
cd phantom-engine
cargo build --release --package phantom-mcp --bin phantom
./target/release/phantom
```

Run the test suite:

```bash
cargo test --workspace -- --test-threads=1
```

---

## Observability

Phantom Engine exposes Prometheus metrics at `/metrics`. Key metrics:

| Metric | Description |
|--------|-------------|
| `sessions_created_total` | Total sessions opened since startup |
| `sessions_active` | Currently live sessions |
| `phantom_circuit_breaker_state` | Runtime pool health (0 = closed/healthy) |
| `storage_quota_used_bytes` | Bytes written to session storage |
| `http_request_duration_seconds` | RPC handler latency by tool |

Alerts for engine-down, circuit breaker open, and storage pressure ship in `prometheus/rules/phantom.yml`.

---

## License

[Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0)

---

Contributions are welcome. Please read [CONTRIBUTING.md](https://github.com/polymit/phantom-engine/blob/main/CONTRIBUTING.md) before opening a pull request.
