# Phantom CLI (`ph`)

> The native control layer for the Phantom Engine — engineered for precision orchestration and human-like web interaction.

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE.md)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![Version](https://img.shields.io/badge/version-v0.1.2--alpha-green.svg)](Cargo.toml)

`ph` is a high-performance command-line utility designed to manage the Phantom Engine lifecycle. It provides an ergonomic interface for session orchestration, real-time diagnostics, and simulating complex user behaviors across the engine's tiered execution environments.

---

## What It Does

Phantom CLI serves as the primary gateway for developers and agents to interact with the engine. It handles:
- **Server Orchestration**: Background process management for the MCP server.
- **Interactive Configuration**: Streamlined environment bootstrapping.
- **Behavioral Simulation**: Triggering human-like events (clicks, typing, navigation) with high-fidelity anti-detection.
- **Real-time Observability**: Live streaming of DOM mutation events and server logs.

## Architecture

`ph` communicates with the **Phantom MCP Server** using JSON-RPC 2.0 over HTTP. This decoupled architecture allows the CLI to manage local or remote engine instances seamlessly. It leverages `tokio` for asynchronous I/O and `hyper` for robust network transport.

---

## Getting Started

### Prerequisites
- **Rust Toolchain**: 1.75.0 or newer.
- **Phantom Engine**: The server components must be built (`cargo build --workspace`).

### Installation
Build and install the binary directly from the source:

```bash
# From the repository root
cargo install --path phantom-cli
```

### Interactive Setup
Bootstrap your local environment with a single command. The CLI will guide you through the configuration process:

```bash
ph setup init
```

> [!TIP]
> The setup command generates unique API keys and secure encryption keys for session snapshots automatically.

---

## Usage

### ⚡ Server Management
Control the background engine process without leaving your terminal.

```bash
# Start the engine in detached mode
ph up --background

# Monitor live server logs
ph logs --follow

# Gracefully terminate the engine
ph down
```

### 🖱️ Human-like Interaction
Simulate realistic user behavior to bypass sophisticated bot detection.

```bash
# Navigate to a URL with protocol normalization
ph navigate google.com

# Simulate a human-like click with randomized coordinates
ph click ".login-button"

# Type text with realistic per-character delays
ph type "#search" "phantom engine" --delay 45
```

### 🔍 Observability & Debugging
Inspect the internal state of the engine in real-time.

```bash
# Enter an interactive REPL shell
ph interactive

# Stream live DOM deltas via SSE
ph watch

# Inspect text nodes within the current scene graph
ph inspect "Checkout"
```

---

## Command Reference

| Command | Description |
|:---|:---|
| `ping` | Verify connection to the MCP server. |
| `status` | Show health, session metrics, and circuit breaker status. |
| `up` | Launch the Phantom Engine server. |
| `down` | Stop the background server process. |
| `logs` | View and follow engine process logs. |
| `tab` | Manage browser tabs (new, list, switch, close). |
| `cookies` | Manage the persistent cookie store. |
| `session` | Orchestrate snapshots and session clones. |
| `setup` | Bootstrap and verify the local environment. |
| `interactive` | Launch the Phantom interactive shell. |

---

## Configuration

`ph` loads settings from `.env` by default. These can be overridden via global flags:

| Env Variable | CLI Flag | Description |
|:---|:---|:---|
| `PHANTOM_BIND_ADDR` | `--server` | MCP server address (Default: `127.0.0.1:8080`) |
| `PHANTOM_API_KEY` | `--key` | Engine authentication key. |

---

## Project Structure

```text
phantom-cli/
├── src/
│   ├── commands/      # Subcommand implementations
│   ├── client.rs      # JSON-RPC transport layer
│   ├── errors.rs      # Unified CLI error types
│   └── main.rs        # CLI entry point and argument parsing
└── README.md          # This file
```

---

## License

This project is licensed under the **Apache-2.0 License**. See [LICENSE.md](LICENSE.md) for details.
