# Phantom CLI (`ph`) 🎮

The native Rust command-line interface for the Phantom Engine. This tool provides a human-facing control layer for engine setup, diagnostics, and session management.

## 🚀 Quick Start

### Installation
From the project root:
```bash
cargo install --path phantom-cli
```

### Initial Setup
Bootstrap your local environment (~/.phantom, .env, and cryptographic keys):
```bash
ph setup init
```

### Verify Configuration
Check if your engine and environment are wired up correctly:
```bash
ph setup doctor
```

## 🛠️ Usage Examples

### Navigation & Interaction
```bash
# Navigate to a URL
ph navigate google.com

# Click an element
ph click "#login-button"

# Type text with realistic delays
ph type "#search" "phantom engine" --delay 50
```

### Live Debugging
```bash
# Open an interactive REPL shell
ph interactive

# Stream live DOM updates (SSE)
ph watch

# Inspect DOM by text query
ph inspect "Sign In"
```

### Session & Cookie Management
```bash
# List open tabs
ph tab list

# Snapshot the current session
ph session snapshot

# List available cookies
ph cookies get
```

## 📖 Commands Reference

| Command | Description |
|---------|-------------|
| `ping` | Check server connectivity |
| `status` | Show server health and circuit breakers |
| `navigate` | Load a URL in the active tab |
| `click` | Trigger a human-like click event |
| `type` | Input text into a selector |
| `scene-graph` | Dump the current DOM state |
| `interactive` | Start the REPL shell |
| `watch` | Listen for real-time engine events |
| `setup` | Environment diagnostics (`init`, `doctor`) |

---
*Part of the Phantom Engine v0.1 Release.*
