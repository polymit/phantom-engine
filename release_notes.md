# Phantom Engine v0.1.1-alpha

We're moving fast. This release is all about making the engine actually usable for humans, not just agents. We finally built a real CLI so you can stop wrestling with raw JSON-RPC if you just want to test something.

### 🎮 The New Phantom CLI (`ph`)
We added a native Rust CLI to the workspace. No more `curl` madness.
* **Instant Setup**: `ph setup init` handles your `.env` and directory tree.
* **Interactive Mode**: A proper REPL shell for live engine debugging.
* **DOM Inspection**: `ph scene-graph` and `ph inspect` to see what the engine sees.
* **Live Stream**: `ph watch` lets you tap into the SSE delta stream directly.

### 🛡️ Stability & Audit
After a massive line-by-line audit, we've hardened the core. 
* **Zero Unwraps**: Cleaned up the remaining panic points in the navigation pipeline.
* **Budget Enforcement**: Fixed the CPU budget leaks that were causing session hangs on heavy sites.
* **Storage Isolation**: Tightened unix permissions on session data (0o700 by default).

### 🚀 What's New
* Updated all workspace crates to `0.1.1-alpha`.
* Added `phantom-cli` as a first-class citizen in the repo.
* Fixed the circuit breaker logic for Tier 1 (QuickJS) execution.

---
*To get started with the new CLI:*
```bash
cargo install --path phantom-cli
ph setup doctor
```
