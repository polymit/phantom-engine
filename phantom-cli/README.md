# Phantom CLI (`ph`)

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE.md)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![Version](https://img.shields.io/badge/version-0.1.1--alpha-green.svg)](Cargo.toml)

The native command-line interface for the **Phantom Engine**. `ph` provides a powerful, human-centric control layer for engine orchestration, real-time diagnostics, and session management.

---

## 🚀 Installation

Install the CLI directly from the source:

```bash
# From the project root
cargo install --path phantom-cli
```

Ensure your `~/.cargo/bin` is in your `PATH` to use the `ph` command globally.

---

## ⚡ Server Management

Phantom CLI now includes built-in orchestration for the Phantom Engine server.

```bash
# Start the server in the background
ph up --background

# Monitor live server logs
ph logs --follow

# Gracefully shutdown the background server
ph down
```

---

## 🛠️ Usage Examples

### 🧩 Environment Setup
Prepare your local environment, generate secure keys, and initialize configurations.

```bash
# Bootstrap local directory (~/.phantom) and .env
ph setup init

# Run diagnostics to verify connectivity and keys
ph setup doctor
```

### 🖱️ Interaction & Navigation
Simulate human-like interactions with the engine's layout and execution pipelines.

```bash
# Navigate with automatic protocol normalization
ph navigate google.com

# Trigger human-like click events on CSS selectors
ph click "#login-button"

# Input text with realistic, randomized per-character delays
ph type "#search-box" "phantom engine" --delay 45
```

### 🔍 Live Debugging & Inspection
Stream real-time updates and inspect the internal state of the engine.

```bash
# Open an interactive REPL shell for manual control
ph interactive

# Stream live DOM mutation events via SSE
ph watch

# Search for specific text content within the active DOM
ph inspect "Order Success"
```

### 💾 Session Persistence
Manage cookies, tabs, and compressed session snapshots.

```bash
# List all active tab UUIDs
ph tab list

# Create a copy-on-write clone of the current session
ph session clone

# Snapshot the entire engine state to disk
ph session snapshot
```

---

## ⚙️ Configuration

`ph` automatically loads configurations from your `.env` file, but can be overridden via global flags:

| Flag | Env Variable | Description |
|------|--------------|-------------|
| `--server` | `PHANTOM_BIND_ADDR` | MCP server address (default: `127.0.0.1:8080`) |
| `--key` | `PHANTOM_API_KEY` | Your engine authentication key |

---

## 📖 Command Reference

| Command | Subcommand | Description |
|---------|------------|-------------|
| `ping` | - | Verify reachability of the engine |
| `status` | - | Show health, sessions, and circuit breaker status |
| `up` | - | Start the engine (use `--background` for detached mode) |
| `down` | - | Stop the background engine process |
| `logs` | - | View engine stdout/stderr logs |
| `tab` | `new`, `list`, `switch`, `close` | Manage browser tabs |
| `cookies` | `get`, `set`, `clear` | Manage engine cookie store |
| `setup` | `init`, `doctor` | Bootstrap and verify environment |

---

> [!NOTE]
> This tool is part of the **Phantom Engine v0.1 Release**. 
