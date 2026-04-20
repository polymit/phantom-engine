# Contributing to Phantom Engine

Thank you for your interest. Before you open a pull request, please read this document in full. Phantom Engine is a complex, multi-layered system and contributions that don't fit the architecture or lower the quality bar will not be merged — no matter how well-intentioned. We prefer one excellent contribution over ten mediocre ones.

---

## What Phantom Engine actually is

Understanding the system deeply is a prerequisite for contributing to it. This section explains how the pieces fit together.

### The problem it solves

Agents that browse the web today are working with bad tools. They get screenshots (expensive, imprecise), raw HTML (enormous, noisy, full of hidden elements), or simplified DOM dumps (lossy, no layout information). None of these tell the agent what a human would actually see on the screen, where the interactive elements are, or what happens when you click something.

Phantom Engine solves this by running a real browser pipeline — parse, style, layout, serialize — and producing a **Compressed Content Tree (CCT)**: a structured, line-per-node text format that describes exactly what is visible in the viewport, with coordinates, roles, text content, and interactivity hints. It is designed to be token-efficient for language model consumption while being precise enough for programmatic interaction.

### The pipeline

Every page load goes through four sequential passes:

**1. Parse** (`phantom-core/src/parser.rs`)
HTML is parsed by `html5ever` into an arena-allocated DOM tree (`indextree`). The parser follows the HTML5 spec and handles malformed markup the same way a browser would. The output is a `DomTree` — an arena of `DomNode` values connected by parent/child/sibling relationships.

**2. CSS Cascade** (`phantom-core/src/css/`)
A custom CSS engine walks the tree and computes styles for each element. It handles:
- External stylesheets (fetched concurrently in `phantom-net`)
- Internal `<style>` tags
- Inline `style` attributes
- Property inheritance (`visibility` inherits, `opacity` multiplies)
- Selector matching via the `selectors` crate (the same engine Servo uses)

The output is a set of `ComputedStyle` values written back into each `DomNode`.

**3. Layout** (`phantom-core/src/layout/`)
A Taffy flexbox/grid layout engine computes bounding boxes for every element. The tree is traversed to build a parallel Taffy tree, `compute_layout()` is called once from the root, and the resulting absolute bounds are extracted into a static `LayoutMap` (a `HashMap<NodeId, ViewportBounds>`). Making the map static (not tied to the live Taffy tree) makes `ParsedPage` `Send + Sync` and safe to move across threads.

**4. Visibility** (`phantom-core/src/pipeline.rs`)
A final pass walks the tree and sets `is_visible` on each node using six conditions simultaneously: not `display: none`, not `visibility: hidden`, not `opacity: 0`, non-zero width, non-zero height, and intersects the viewport. This is the ground truth for what a human would see.

### Serialization (`phantom-serializer/`)

The CCT serializer converts a `ParsedPage` into the text format agents consume. It has three modes:

- **Full** — every visible node
- **Selective** — filters to task-relevant nodes when a `task_hint` is provided
- **Delta** — only nodes that changed since the last snapshot (used for streaming/subscription)

The serializer also handles:
- Z-index ordering (`zindex.rs`)
- Semantic role labeling (`semantic.rs`)
- ID stabilization across re-renders (`id_stabilizer.rs`)
- Buffer pooling to avoid per-call allocation (`buffer_pool.rs`)

### JavaScript execution (`phantom-js/`)

JavaScript is a two-tier system:

**Tier 1 — QuickJS** (`phantom-js/src/tier1/`)
A pool of QuickJS runtimes (via `rquickjs`). QuickJS is fast to initialize (<10ms), low-memory, and appropriate for stateless evaluation tasks: running page scripts during navigation, evaluating simple expressions, handling timers and MutationObservers. The pool pre-warms a configurable number of runtimes.

**Tier 2 — V8/Deno Core** (`phantom-js/src/tier2/`)
A pool of V8 isolates via `deno_core`. V8 is heavier (~50ms init, ~512MB heap limit) but supports the full modern JS API surface including async/await, fetch, and complex frameworks. Used for stateful sessions where a site requires persistent JS state across interactions.

A **circuit breaker** (`phantom-js/src/circuit_breaker.rs`) sits in front of both pools. If a pool starts failing (OOM, panics, timeouts), the breaker opens and new requests fast-fail rather than queuing behind a broken runtime. It transitions through Closed → Open → Half-Open → Closed states.

JavaScript shims (`phantom-js/js/`) polyfill browser APIs that headless environments lack: `EventTarget`, `Location`, `MutationObserver`, `fetch`, and various `window.*` properties.

### Network layer (`phantom-net/`)

HTTP is handled by `wreq` — not `reqwest`. This is intentional and non-negotiable. `wreq` emulates Chrome's TLS fingerprint (JA3/JA4), ALPN negotiation, and HTTP/2 settings frames. This makes Phantom Engine's network requests indistinguishable from a real browser to bot-detection systems. `reqwest`, `hyper-rustls`, and `native-tls` are explicitly banned in `deny.toml` because they produce detectable TLS fingerprints.

The `SmartNetworkClient` also tracks `Alt-Svc` headers to upgrade connections to HTTP/3 where available, and supports per-persona client construction so TLS fingerprints stay consistent with the JS-level `User-Agent`.

### Anti-detection (`phantom-anti-detect/`)

The `Persona` type encodes a complete browser identity: User-Agent, platform, screen dimensions, hardware concurrency, device memory, language, timezone, WebGL vendor/renderer strings, canvas noise seed, and all `navigator.userAgentData` fields.

The `PersonaPool` ships five profiles covering Chrome 133 and 134 on Windows 10, Windows 11, and macOS Sonoma. Profiles rotate round-robin. Canvas noise seeds are unique per persona. SwiftShader, Mesa, and llvmpipe are explicitly banned from GPU strings because they are instant bot-detection signals.

`BehaviorTiming` generates human-plausible inter-action delays using a log-normal distribution, matching observed human timing variance rather than fixed intervals.

### Session management (`phantom-session/`)

A `Session` is a lightweight record: UUID, engine kind (V8 or QuickJS), persona ID, resource budget (heap bytes, CPU ms, network bytes), and lifecycle state. States are: `Idle → Running → Suspended → Cloned → Destroyed`.

The `SessionBroker` is an in-memory registry backed by a `Mutex<HashMap<Uuid, Session>>`. It enforces budget limits on every operation. Sessions can be forked via copy-on-write clone (`Session::with_uuid`), which creates a new session sharing the same initial state without re-executing the page.

### Storage (`phantom-storage/`)

All storage is session-isolated and path-validated. Session IDs must be well-formed UUID v4 strings; anything else is rejected before a path is constructed (protecting against path traversal). Session directories are created with `chmod 700` on Unix.

- **Cookies** — serialized as JSON via `cookie_store`, written atomically via rename-from-temp
- **localStorage** — per-origin `sled` embedded databases
- **IndexedDB** — per-origin SQLite databases via `rusqlite` (WAL mode, parameterized queries throughout)
- **Cache API** — blob files addressed by SHA-256, with a `sled`-backed metadata index
- **Snapshots** — full session state archived as tar, compressed with zstd level 1, with an HMAC-SHA256 signed manifest for tamper detection

### The RPC server (`phantom-mcp/`)

The entry point (`phantom-mcp/src/bin/phantom.rs`) starts a `tower`/`axum`-based HTTP server with:
- `POST /rpc` — JSON-RPC 2.0 dispatch
- `GET /health` — liveness probe
- `GET /metrics` — Prometheus text exposition

Authentication is bearer-token via `X-API-Key` header, validated on every request before routing. Rate limiting and session limits are enforced at the engine layer (`phantom-mcp/src/engine.rs`).

Prometheus metrics (`phantom-mcp/src/metrics.rs`) and structured tracing (`phantom-mcp/src/telemetry.rs`) are initialized at startup. The `prometheus` crate (not `metrics-rs`) is used directly, keeping the metric definitions explicit and grep-able.

---

## Quality standards

### Error handling

- Use `thiserror` for all error types. `anyhow` is banned — the CI policy scan will catch it.
- Every error variant must carry enough context to diagnose the problem without a debugger. `Err("failed")` is not acceptable.
- Never use `.unwrap()` or `.expect()` in library code. Only in tests, and only with `#[allow(clippy::unwrap_used)]` at the top of the test file.
- Propagate errors with `?`. Do not swallow them with `let _ =` unless you can prove they are genuinely unrecoverable and unloggable.

### Dependencies

- All versions must be pinned with `=` in `Cargo.toml`. No floating versions.
- New dependencies must be reviewed against `deny.toml`. If you need to add a ban exception, explain why in the PR.
- Never add `reqwest`, `rquest`, `native-tls`, `openssl-sys`, or `rusty_v8`. These are banned for technical reasons (TLS fingerprinting, symbol conflicts) that are not negotiable.
- Prefer crates already in the dependency graph over new ones.

### Tests

Every non-trivial change must come with tests. The project runs tests single-threaded (`--test-threads=1`) because JS runtimes use process-global V8 initialization. Write tests that work under this constraint.

- **Unit tests** belong in `#[cfg(test)]` blocks inside the source file.
- **Integration tests** belong in the crate's `tests/` directory.
- Tests that make real network requests must handle the offline case gracefully — check the pattern in `phantom-net/tests/navigate_test.rs`.
- Performance-sensitive code should have a Criterion benchmark in `benches/`.

### Code style

- Run `cargo fmt --all` before committing.
- Run `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used -D clippy::expect_used` and fix every warning. The CI gate treats warnings as errors.
- Public items must have doc comments. `cargo doc --no-deps --document-private-items` must build without warnings.
- Prefer explicit types over `impl Trait` in public APIs.

---

## CI gates

Every pull request must pass all twelve required jobs before it can be merged:

| Job | What it checks |
|-----|----------------|
| `fmt` | `cargo fmt --all --check` |
| `clippy` | Zero warnings, unwrap/expect banned |
| `check` | Compiles, no anyhow, no restricted deps, no wildcard versions |
| `test` | Full test suite, single-threaded |
| `deny` | Bans, licenses, advisories |
| `audit` | `cargo-audit` against RustSec |
| `lock-verify` | `Cargo.lock` matches `Cargo.toml`, key crate versions pinned |
| `msrv` | Compiles on Rust 1.94 |
| `doc` | Docs build without warnings |
| `performance-gate` | Criterion benchmark regressions |
| `scale-smoke` | 100-session smoke test |
| `security-isolation` | Security audit test suite |

If any job fails, the PR cannot merge. The `ci-pass` job is the required GitHub branch protection check.

All CI checks must pass before a PR is reviewed. This is not bureaucracy — it is how we protect quality. A green CI is the minimum bar, not the finish line. We would rather wait for a correct contribution than merge a fast one that degrades the system.

---

## Opening a pull request

1. **Open an issue first** for anything non-trivial. Discuss the approach before writing code. This prevents wasted effort on directions that won't be accepted.
2. **Keep PRs focused.** One logical change per PR. A PR that fixes a bug and adds a feature and refactors a module will be asked to split.
3. **Write a clear description.** Explain what changed, why, and what you tested. Link to the relevant issue.
4. **Do not bump crate versions** unless you are a maintainer. Version management is handled at release time.

---

## Security vulnerabilities

If you find a security vulnerability — in any part of the engine, the storage layer, the snapshot system, the authentication, or the container configuration — **do not open a public GitHub issue.**

Please report it privately to **[polymit.main@gmail.com](mailto:polymit.main@gmail.com)**. Include a description of the vulnerability, reproduction steps, and your assessment of impact. We will acknowledge receipt within 48 hours and work with you on a coordinated disclosure timeline.

We take security reports seriously. The engine handles session state, credentials, and browsing data for potentially many concurrent users. A vulnerability here is not a minor inconvenience.

---

## Code of Conduct

We strictly follow the [Contributor Covenant](https://www.contributor-covenant.org/version/2/1/code_of_conduct/). By participating in this project, you are expected to uphold it. Violations can be reported to [polymit.main@gmail.com](mailto:polymit.main@gmail.com).

---

## License

By contributing to Phantom Engine, you agree that your contributions will be licensed under the [Apache 2.0 License](https://github.com/polymit/phantom-engine/blob/main/LICENSE.md).
