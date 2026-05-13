//! Unified error surface for the `quik` transport stack.
//!
//! Every layer in the engine—TLS negotiation, HTTP/2 signaling, and proxy
//! handshakes—reports through this boundary. A stable error surface is essential
//! for higher-level session management to perform connection pooling, identity
//! rotation, and retry logic without inspecting subsystem-specific internals.

use thiserror::Error;

/// Errors that can occur during high-fidelity transport operations.
#[derive(Debug, Error)]
pub enum Error {
    /// Failure during the construction of the BoringSSL context.
    ///
    /// This usually indicates an invalid cipher list or unsupported curve
    /// configuration in the profile.
    #[error("failed to build TLS connector: {0}")]
    TlsBuild(#[from] boring::error::ErrorStack),

    /// Failure during the TLS handshake with the remote peer.
    ///
    /// These errors often stem from peer-side fingerprint validation or
    /// mismatches in the ClientHello permutation.
    #[error("TLS handshake failed: {0}")]
    TlsHandshake(#[from] tokio_boring::HandshakeError<tokio::net::TcpStream>),

    /// Failure during the HTTP/2 handshake or frame signaling.
    ///
    /// The `http2` crate returns specific errors for SETTINGS violations
    /// or stream reset events that deviate from the expected profile.
    #[error("http/2 handshake failed: {0}")]
    Http2(#[from] http2::Error),

    /// Standard I/O failure during connection establishment or data transfer.
    #[error("connection failed: {0}")]
    Connect(#[from] std::io::Error),

    /// Fingerprint verification failed against a reference validator.
    ///
    /// This occurs when the actual wire behavior (JA3/JA4/Akamai) drifts
    /// from the constants defined in the identity profile.
    #[error("fingerprint verification failed: {0}")]
    Verify(String),

    /// The provided URL is malformed or uses an unsupported scheme.
    #[error("invalid url: {0}")]
    InvalidUrl(String),
}

pub type Result<T> = std::result::Result<T, Error>;
