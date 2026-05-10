# Quik: High-Fidelity Transport Layer

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Built for Phantom](https://img.shields.io/badge/Phantom%20Engine-v0.2.0--alpha-black.svg)]()

**Quik** is a high-performance, precision-engineered HTTP client developed and maintained by the **Phantom Engine Team**. Designed from the ground up for absolute network identity parity, Quik serves as the core transport layer for the Phantom ecosystem, bypassing advanced anti-automation heuristics through exact browser replication.

## Overview

Unlike generalized HTTP libraries, Quik does not compromise on network identity. By controlling the entire network stack from the TCP socket up to the HPACK encoder, Quik achieves **100% cryptographic and behavioral parity** with Google Chrome. 

This enables automated systems, researchers, and agentic workflows to interact with heavily protected web infrastructure (e.g., Cloudflare, Akamai, Datadome) without triggering network-layer anomalies.

## Core Architecture

Quik achieves its fingerprint accuracy by replacing standard networking abstractions with surgical, byte-level implementations:

*   **BoringSSL v4 Backend**: Direct FFI bindings to Google's BoringSSL, enabling exact replication of Chrome's TLS ClientHello, including ECH GREASE, Post-Quantum Key Shares (`X25519Kyber768Draft00`), and ALPS binary payloads.
*   **Deterministic HTTP/2 Engine**: Built on a customized `http2` protocol implementation to guarantee exact pseudo-header ordering (`m,a,s,p`) and identical `SETTINGS` frame sequences.
*   **Stealth Redirect Engine**: A fully stateful navigation manager that processes 301/302 redirects by adhering strictly to Chromium's cross-origin mutation rules (managing `sec-fetch-site` states and dropping `sec-fetch-user` appropriately).
*   **HPACK Literal Enforcement**: Integrated session persistence (`cookie_store`) paired with dynamic HPACK sensitivity tagging to prevent Cloudflare from detecting agents via dynamic table state analysis.
*   **End-to-End Tunneling**: Native support for pre-handshake SOCKS5 and HTTP CONNECT proxy dialing, ensuring the Server Name Indication (SNI) and TLS fingerprints remain perfectly encrypted and uncompromised.

## Integration & Usage

Quik provides a thread-safe, high-level connection pool designed as a direct replacement for traditional HTTP clients.

```rust
use quik::{Client, Proxy};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Optional: Configure residential proxy routing
    let proxy = Proxy::parse("socks5://127.0.0.1:9050")?;

    // Instantiate the Client with a guaranteed Chrome 134 identity
    let client = Client::builder()
        .proxy(proxy)
        .build()?;

    // Execute navigation. The client automatically handles connection pooling, 
    // cookie persistence, and stealth-optimized redirect chains.
    let response = client.get("https://tls.peet.ws/api/all").await?;

    println!("Status: {}", response.status());
    let body = response.bytes().await?;
    
    Ok(())
}
```

## Contributing

Quik is a critical component of the Phantom Engine ecosystem. Contributions must strictly adhere to the project's behavioral parity constraints. Any modifications to the TLS handshake, frame ordering, or header sequencing must be validated against the `chrome_134` integration test suite to ensure the Akamai and JA4 fingerprints remain pristine.

## License

Copyright © 2026 Phantom Engine Team.

This project is licensed under the **Apache License, Version 2.0**. 
You may not use this file except in compliance with the License. You may obtain a copy of the License at [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0).

Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the specific language governing permissions and limitations under the License.
