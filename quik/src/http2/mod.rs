//! HTTP/2 layer — Chrome SETTINGS frame, WINDOW_UPDATE, pseudo-header order,
//! and HEADERS frame PRIORITY block. Uses http2 = 0.5 fork exclusively.

pub(crate) mod handshake;

pub use handshake::configure_builder;
