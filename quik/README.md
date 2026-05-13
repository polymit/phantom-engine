# Quik Transport Layer

`quik` is a high-fidelity HTTP transport engine designed for absolute network identity parity with Google Chrome 134. It serves as the foundational networking stack for the [Phantom Engine](https://github.com/polymit/phantom-engine), providing the low-level byte control necessary to bypass modern anti-automation and fingerprinting heuristics.

Unlike generalized HTTP clients, `quik` is built to be indistinguishable from a standard browser at every layer of the networking stack—from TLS handshake permutation to HTTP/2 frame signaling.

## Key Features

- **Full Chrome 134 Identity**: Replicates the exact TLS and HTTP/2 fingerprints of Chrome 134, including JA3, JA4, and Akamai-specific markers.
- **BoringSSL Integration**: Leverages BoringSSL for advanced handshake control, including ECH GREASE, certificate compression (Brotli), and specific signature algorithm ordering.
- **Post-Quantum Security**: Implements the Chrome-identical `X25519MLKEM768` hybrid key exchange group.
- **Precise H2 Signaling**: Enforces Chromium's exact HTTP/2 SETTINGS frame order, initial window deltas, and pseudo-header sequences (`m,a,s,p`).
- **Stealth Navigation Engine**: Automates the mutation of `sec-fetch-*` metadata and priority headers during complex redirect flows.
- **Advanced HPACK Management**: Explicitly marks sensitive headers (like cookies and authorization) as "Never Indexed" to mirror Chromium's security and fingerprinting behavior.

## Documentation

Comprehensive technical documentation, including safety contracts and architecture deep-dives, is available at:
**[https://polymit.github.io/phantom-engine/quik/index.html](https://polymit.github.io/phantom-engine/quik/index.html)**

## Usage

`quik` is designed to be used as part of the Phantom Engine ecosystem. It provides a stateful `Client` that handles connection pooling, cookie management, and automated redirects.

```rust
use quik::{Client, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::new();
    let response = client.get("https://example.com").await?;
    
    println!("Status: {}", response.status());
    let body = response.body().await?;
    Ok(())
}
```

## Part of Phantom Engine

This crate is a core component of the **Phantom Engine** project. It works in conjunction with `phantom-net` and `phantom-session` to provide a complete, stealth-optimized browsing environment.

## Contributing

We welcome contributions that improve the fidelity of the transport layer or add support for newer Chrome versions. Please refer to the [CONTRIBUTING.md](https://github.com/polymit/phantom-engine/blob/main/CONTRIBUTING.md) at the repository root for our contribution guidelines and code of conduct.

## License

Copyright © 2026 Polymit.

Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0).
