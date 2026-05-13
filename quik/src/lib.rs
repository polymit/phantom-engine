//! # Quik: High-Fidelity Transport Layer
//!
//! `quik` is a specialized HTTP transport engine designed for absolute network identity parity
//! with Google Chrome. Unlike general-purpose HTTP clients, `quik` provides low-level control
//! over the entire protocol stack—from TLS handshakes to HTTP/2 frame signaling—to ensure
//! that every network interaction is indistinguishable from a real browser.
//!
//! ## Core Capabilities
//! - **TLS Identity**: Uses BoringSSL to replicate Chrome's ClientHello, including GREASE,
//!   extension permutation, and post-quantum key shares.
//! - **H2 Parity**: Enforces specific SETTINGS order, pseudo-header sequences, and connection
//!   window deltas to match Chromium's behavioral fingerprints.
//! - **Stealth Navigation**: Manages request context metadata (sec-fetch headers) and HPACK
//!   sensitivity to bypass advanced anti-automation heuristics.

pub mod client;
pub mod error;

/// Low-level HTTP/2 frame and builder configuration.
pub(crate) mod http2;

pub mod profile;

/// TLS connector construction and FFI bindings for BoringSSL.
pub(crate) mod tls;

pub use crate::client::{connect, Client, Response};
pub use crate::error::{Error, Result};
pub use crate::profile::chrome_134::AKAMAI_FINGERPRINT;
pub use crate::profile::chrome_134::JA3_HASH;
pub use crate::profile::{ChromeProfile, Platform};
