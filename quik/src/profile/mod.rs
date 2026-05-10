//! Data-only carrier types for Chrome fingerprint configuration.
//!
//! No protocol logic lives here. The TLS and HTTP/2 layers read these
//! structs and translate them into BoringSSL and `http2` builder calls.

use boring::ssl::SslVersion;

pub mod chrome_134;

pub type TlsVersion = SslVersion;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    MacOsArm,
    MacOsX86,
    WindowsX64,
    LinuxX64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TlsProfile {
    pub min_version: TlsVersion,
    pub max_version: TlsVersion,
    // OpenSSL expects a colon-separated cipher name string here, not JA3
    // decimal IDs. The wrong format can make `set_cipher_list()` silently
    // accept only a partial list.
    pub cipher_list: &'static str,
    pub curves: &'static [u16],
    pub grease_enabled: bool,
    pub permute_extensions: bool,
    pub enable_ech_grease: bool,
    pub alps_enabled: bool,
    pub alps_use_new_codepoint: bool,
    pub compress_certificate: bool,
    pub session_ticket_enabled: bool,
    pub alpn_protocols: &'static [&'static [u8]],
    pub sigalgs: &'static [u16],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingsFrame {
    pub header_table_size: u32,
    pub enable_push: bool,
    pub initial_window_size: u32,
    pub max_header_list_size: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Http2Profile {
    pub settings: SettingsFrame,
    // This is the full connection window: 65535 (default) + 15663105
    // (`WINDOW_UPDATE` delta) = 15728640. It is distinct from
    // `settings.initial_window_size`, which is per-stream. These are
    // different frames.
    pub initial_connection_window_size: u32,
    pub pseudo_order: [PseudoOrder; 4],
    pub headers_priority: HeadersPriority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// A stored value of `255` is the HTTP/2 wire value `255`, which represents a
// semantic priority weight of `256` because the encoded octet is `weight - 1`.
// This is the PRIORITY block embedded in the HEADERS frame when `flags = 0x25`,
// not a separate PRIORITY frame (`type = 0x02`).
pub struct HeadersPriority {
    pub dep: u32,
    pub weight: u8,
    pub exclusive: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PseudoOrder {
    Method,
    Authority,
    Scheme,
    Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderProfile {
    pub user_agent: String,
    pub sec_ch_ua: String,
    pub sec_ch_ua_platform: String,
    pub include_priority_header: bool,
    pub zstd_encoding: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeProfile {
    pub version: u32,
    pub platform: Platform,
    pub tls: TlsProfile,
    pub h2: Http2Profile,
    pub headers: HeaderProfile,
}
