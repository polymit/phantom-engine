//! Unified error type for the entire crate.
//!
//! Every transport layer in `quik` reports through this one boundary: TLS
//! setup, HTTP/2 handshake, connection I/O, and fingerprint verification.
//! The session layer above this crate needs one stable error surface so it
//! can pool connections, rotate identities, and report drift without
//! guessing which subsystem produced the failure.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    // `boring::error::ErrorStack` wraps BoringSSL's internal error queue and
    // is the canonical error returned from any `SSL_CTX_*` call.
    #[error("failed to build TLS connector: {0}")]
    TlsBuild(#[from] boring::error::ErrorStack),
    // The `tokio-boring` handshake error can be complex; we wrap it here.
    #[error("TLS handshake failed: {0}")]
    TlsHandshake(#[from] tokio_boring::HandshakeError<tokio::net::TcpStream>),
    // The `http2` fork may return non-standard error codes for SETTINGS frame
    // mismatches; always log the raw string.
    #[error("http/2 handshake failed: {0}")]
    Http2(#[from] http2::Error),
    #[error("connection failed: {0}")]
    Connect(#[from] std::io::Error),
    #[error("fingerprint verification failed: {0}")]
    Verify(String),
    #[error("invalid url: {0}")]
    InvalidUrl(String),
}

pub type Result<T> = std::result::Result<T, Error>;
