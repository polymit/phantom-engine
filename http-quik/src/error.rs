//! Unified error surface for the `quik` transport stack.
//!
//! Every layer in the engine—TLS negotiation, HTTP/2 signaling, and proxy
//! handshakes—reports through this boundary. A stable error surface is essential
//! for higher-level session management to perform connection pooling, identity
//! rotation, and retry logic without inspecting subsystem-specific internals.

use thiserror::Error;

/// Errors that can occur during high-fidelity transport operations.
///
/// This enum categorizes failures across the entire protocol stack, from low-level
/// TCP dialing to high-level HTTP/2 frame signaling.
#[derive(Debug, Error)]
pub enum Error {
    /// Failure during the construction of the BoringSSL context.
    ///
    /// This usually indicates an invalid cipher list, unsupported curve
    /// configuration, or a missing FFI symbol in the linked BoringSSL binary.
    #[error("failed to build TLS connector: {0}")]
    TlsBuild(#[from] boring::error::ErrorStack),

    /// Failure during the TLS handshake with the remote peer.
    ///
    /// These errors often stem from peer-side fingerprint validation, protocol
    /// version mismatches, or failures in the ALPN/ALPS negotiation phase.
    #[error("TLS handshake failed: {0}")]
    TlsHandshake(#[from] tokio_boring::HandshakeError<tokio::net::TcpStream>),

    /// Failure during the HTTP/2 handshake or frame signaling.
    ///
    /// This error is returned when the remote peer violates the H2 protocol or
    /// when the internal state machine fails to replicate the required Chrome 
    /// behavior (e.g., SETTINGS frame ordering).
    #[error("http/2 handshake failed: {0}")]
    Http2(#[from] http2::Error),

    /// Standard I/O failure during connection establishment or data transfer.
    ///
    /// This covers TCP timeout, connection reset, and other OS-level network errors.
    #[error("connection failed: {0}")]
    Connect(#[from] std::io::Error),

    /// Fingerprint verification failed against a reference validator.
    ///
    /// This is an orchestration error that occurs when the actual wire behavior
    /// (JA3/JA4/Akamai) deviates from the constants defined in the identity profile.
    #[error("fingerprint verification failed: {0}")]
    Verify(String),

    /// The provided URL is malformed or uses an unsupported scheme.
    #[error("invalid url: {0}")]
    InvalidUrl(String),
}

pub type Result<T> = std::result::Result<T, Error>;
