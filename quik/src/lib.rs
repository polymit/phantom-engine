//! # http-quik: High-Fidelity stealth transport engine
//!
//! `http-quik` is a specialized HTTP transport library designed for absolute network identity parity
//! with Google Chrome. It provides low-level control over the entire protocol stack—from TLS
//! handshakes to HTTP/2 frame signaling—to ensure that every network interaction is
//! indistinguishable from a real browser.
//!
//! This crate is a core component of the [Phantom Engine](https://github.com/polymit/phantom-engine) 
//! ecosystem and provides the high-stealth transport layer required for modern agentic navigation.
//!
//! ## Why http-quik?
//! Modern Anti-Bot systems (like Cloudflare, Akamai, and DataDome) use "Passive Fingerprinting"
//! to identify automated traffic. They inspect:
//! 1. **TLS Fingerprint (JA3/JA4)**: The order of cipher suites, extensions, and elliptic curves.
//! 2. **HTTP/2 Fingerprint (Akamai)**: The SETTINGS frame values, the order of pseudo-headers, and stream priority.
//!
//! `http-quik` solves this by using a custom BoringSSL stack and a specialized HTTP/2 builder to replicate
//! these fingerprints with bit-perfect accuracy.
//!
//! ## Core Features
//! - **BoringSSL Integration**: Full control over ClientHello, including GREASE and extension permutation.
//! - **Chrome 134 Identity**: Pre-configured profiles for the latest Chrome stable releases.
//! - **Connection Pooling**: Managed H2 session reuse to maintain consistent behavioral fingerprints.
//! - **Stealth Redirects**: A redirect state machine that handles `sec-fetch-*` headers and method rotation identical to Chromium.
//!
//! ## Getting Started
//!
//! ```rust
//! use http_quik::{Client, ChromeProfile, Platform};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), http_quik::Error> {
//!     // Create a client with a macOS Chrome 134 identity
//!     let client = Client::builder()
//!         .profile(http_quik::profile::chrome_134::profile(Platform::MacOsArm))
//!         .build()?;
//!
//!     // Execute a stealth request
//!     let response = client.get("https://example.com").await?;
//!     println!("Status: {}", response.status());
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Safety & FFI
//! This crate uses `boring` and `boring-sys` for low-level TLS control. All unsafe blocks are
//! localized to the `tls` and `client` modules and are documented with safety rationales.

pub mod client;
pub mod error;

/// Low-level HTTP/2 frame and builder configuration.
///
/// This module provides internal utilities for overriding the default H2 handshake
/// to match Chromium's behavioral markers.
pub(crate) mod http2;

pub mod profile;

/// TLS connector construction and FFI bindings for BoringSSL.
///
/// Handles the bit-perfect replication of Chrome's TLS handshake, including
/// post-quantum key shares and extension permutation.
pub(crate) mod tls;

pub use crate::client::{connect, Client, ClientBuilder, Response};
pub use crate::error::{Error, Result};
pub use crate::profile::chrome_134::AKAMAI_FINGERPRINT;
pub use crate::profile::chrome_134::JA3_HASH;
pub use crate::profile::{ChromeProfile, Platform};
