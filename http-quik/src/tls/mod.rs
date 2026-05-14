//! TLS layer — BoringSSL ClientHello construction for Chrome 134 identity.
//! Owns: connector (SslConnector builder), session_store (TLS ticket cache),
//! profile (TlsProfile → boring API translation).

pub(crate) mod connector;

pub use connector::build_connector;
