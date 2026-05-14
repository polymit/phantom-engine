# http-quik: High-Fidelity Stealth Transport Engine

[![Crates.io](https://img.shields.io/crates/v/http-quik.svg)](https://crates.io/crates/http-quik)
[![Docs.rs](https://docs.rs/http-quik/badge.svg)](https://docs.rs/http-quik)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE.md)

`http-quik` is a specialized HTTP transport library built in Rust, designed for absolute network identity parity with Google Chrome. It provides low-level control over the entire protocol stack—from TLS handshakes to HTTP/2 frame signaling—to ensure that every network interaction is indistinguishable from a real browser.

This crate is a core component of the [Phantom Engine](https://github.com/polymit/phantom-engine) ecosystem.

## Why http-quik?

Modern anti-bot systems (like Cloudflare, Akamai, and DataDome) use passive fingerprinting to identify automated traffic. `http-quik` bypasses these systems by replicating:

- **TLS Fingerprints (JA3/JA4)**: Replicates Chrome's ClientHello using a custom BoringSSL stack, including GREASE, extension permutation, and post-quantum key shares.
- **HTTP/2 Fingerprints (Akamai)**: Replicates Chromium's SETTINGS frame order, pseudo-header sequences, and connection window increments.
- **Behavioral Fingerprints**: Implements a Chrome-identical redirect state machine that handles `sec-fetch-*` headers and method rotation.

## Core Capabilities

- **BoringSSL Integration**: Deep FFI bindings for low-level TLS control.
- **Chrome 134 Identity**: Bit-perfect replication of the latest stable Chrome releases.
- **Connection Pooling**: Managed H2 session reuse to maintain behavioral consistency.
- **Customizable Profiles**: Easily target different platforms (macOS, Windows, Linux).

## Quick Start

Add `http-quik` to your `Cargo.toml`:

```toml
[dependencies]
http-quik = "0.1"
```

Execute a stealth request:

```rust
use http_quik::{Client, Platform};

#[tokio::main]
async fn main() -> Result<(), http_quik::Error> {
    // Create a client with a macOS Chrome 134 identity
    let client = Client::new();

    // Execute a stealth GET request
    let response = client.get("https://example.com").await?;
    println!("Status: {}", response.status());

    Ok(())
}
```

## License

This project is licensed under the [Apache License, Version 2.0](LICENSE.md).
