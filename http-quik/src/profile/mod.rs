//! Data-only definitions for Chrome transport identities.
//!
//! This module defines the schemas used to configure the TLS and HTTP/2
//! layers. No protocol logic resides here; instead, these structures act as
//! the configuration contract that the `tls` and `http2` modules translate
//! into specific BoringSSL and H2 builder calls.

use boring::ssl::SslVersion;

pub mod chrome_134;

/// Alias for BoringSSL's internal version type.
pub type TlsVersion = SslVersion;

/// Supported execution environments for profile targeting.
///
/// Hardware and OS markers are embedded in several layers, including the
/// TLS ClientHello (via GREASE and curves) and the HTTP User-Agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// Apple Silicon (M1/M2/M3) - Targeted with specific X25519MLKEM768 support.
    MacOsArm,
    /// Intel-based macOS.
    MacOsX86,
    /// 64-bit Windows.
    WindowsX64,
    /// 64-bit Linux (Generic).
    LinuxX64,
}

/// Configuration for the TLS 1.2/1.3 handshake layer.
///
/// This structure defines the Layer 4 identity of the client. Small changes
/// here (such as the order of cipher suites) will change the JA3/JA4 
/// fingerprint and can lead to immediate detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TlsProfile {
    /// Minimum allowed TLS version (typically TLS 1.2).
    pub min_version: TlsVersion,
    /// Maximum allowed TLS version (typically TLS 1.3).
    pub max_version: TlsVersion,
    /// Colon-separated list of cipher suites in OpenSSL format.
    ///
    /// Precision in the order of this list is critical as it directly
    /// impacts the JA3/JA4 fingerprint.
    pub cipher_list: &'static str,
    /// Numeric IDs for supported elliptic curve groups.
    pub curves: &'static [u16],
    /// Whether to enable TLS GREASE (RFC 8701) to simulate randomized extensions.
    pub grease_enabled: bool,
    /// Whether to permute (shuffle) TLS extensions per connection.
    pub permute_extensions: bool,
    /// Whether to send a dummy ECH (Encrypted Client Hello) extension for GREASE.
    pub enable_ech_grease: bool,
    /// Whether to enable ALPS (Application-Layer Protocol Settings).
    pub alps_enabled: bool,
    /// Whether to use the draft-01 or final ALPS codepoint.
    pub alps_use_new_codepoint: bool,
    /// Whether to support RFC 8879 certificate compression (Brotli).
    pub compress_certificate: bool,
    /// Whether to enable stateless session tickets for fast reconnection.
    pub session_ticket_enabled: bool,
    /// Ordered list of ALPN protocol identifiers.
    pub alpn_protocols: &'static [&'static [u8]],
    /// Ordered list of signature algorithm IDs (used for JA4_r).
    pub sigalgs: &'static [u16],
}

/// Initial HTTP/2 SETTINGS frame parameters.
///
/// The values and the *order* in which they are sent are used by Akamai
/// and other WAFs to identify the client implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingsFrame {
    /// SETTINGS_HEADER_TABLE_SIZE (ID 0x1).
    pub header_table_size: u32,
    /// SETTINGS_ENABLE_PUSH (ID 0x2).
    pub enable_push: bool,
    /// SETTINGS_INITIAL_WINDOW_SIZE (ID 0x4).
    pub initial_window_size: u32,
    /// SETTINGS_MAX_HEADER_LIST_SIZE (ID 0x6).
    pub max_header_list_size: u32,
}

/// Configuration for the HTTP/2 protocol layer.
///
/// Defines the Layer 5 identity, focusing on behavioral markers like
/// pseudo-header ordering and stream priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Http2Profile {
    /// Initial SETTINGS frame values and order.
    pub settings: SettingsFrame,
    /// Total connection-level window size (default + delta).
    ///
    /// This value determines the initial `WINDOW_UPDATE` frame increment
    /// sent immediately after the handshake. Chrome uses a specific non-standard
    /// increment that acts as a strong identity signal.
    pub initial_connection_window_size: u32,
    /// Ordering of pseudo-headers (e.g., :method, :authority, :scheme, :path).
    pub pseudo_order: [PseudoOrder; 4],
    /// Priority parameters for the initial HEADERS frame.
    pub headers_priority: HeadersPriority,
}

/// Stream priority parameters embedded in the HEADERS frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeadersPriority {
    /// Stream ID that this request depends on (typically 0).
    pub dep: u32,
    /// Priority weight (0-255).
    pub weight: u8,
    /// Whether this dependency is exclusive.
    pub exclusive: bool,
}

/// Canonical HTTP/2 pseudo-header identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PseudoOrder {
    /// `:method`
    Method,
    /// `:authority`
    Authority,
    /// `:scheme`
    Scheme,
    /// `:path`
    Path,
}

/// Chrome-specific HTTP header values and behaviors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderProfile {
    /// Full User-Agent string.
    pub user_agent: String,
    /// `sec-ch-ua` Client Hint string.
    pub sec_ch_ua: String,
    /// `sec-ch-ua-platform` Client Hint string.
    pub sec_ch_ua_platform: String,
    /// Whether to include the `priority` header in the request.
    pub include_priority_header: bool,
    /// Whether to include `zstd` in `accept-encoding`.
    pub zstd_encoding: bool,
}

/// A complete, multi-layer identity profile for a Chrome instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeProfile {
    /// Major Chrome version (e.g., 134).
    pub version: u32,
    /// Target operating system and architecture.
    pub platform: Platform,
    /// Layer 4: TLS configuration.
    pub tls: TlsProfile,
    /// Layer 5: HTTP/2 configuration.
    pub h2: Http2Profile,
    /// Layer 6: HTTP header configuration.
    pub headers: HeaderProfile,
}
