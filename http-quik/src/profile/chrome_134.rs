//! This module contains the exact byte sequences, hash values, and protocol
//! parameters observed in the wild for Chrome 134. These values are used
//! by the TLS and HTTP/2 builders to construct an indistinguishable identity.

use boring::ssl::SslVersion;

use crate::profile::{
    ChromeProfile, HeaderProfile, HeadersPriority, Http2Profile, Platform, PseudoOrder,
    SettingsFrame, TlsProfile,
};

/// Reference JA3 fingerprint hash for Chrome 134 on macOS.
pub const JA3_HASH: &str = "845db3b4e398789bdeb5b15594360a29";
/// Reference normalized JA3 (JA3N) hash.
pub const JA3N_HASH: &str = "8e19337e7524d2573be54efb2b0784c9";
/// Reference JA4 fingerprint for modern Chromium.
pub const JA4: &str = "t13d1516h2_8daaf6152771_d8a2da3f94cd";
/// Reference Akamai HTTP/2 fingerprint string.
pub const AKAMAI_FINGERPRINT: &str = "1:65536;2:0;4:6291456;6:262144|15663105|0|m,a,s,p";

/// Exact cipher suite list for Chrome 134.
///
/// Includes TLS 1.3 suites followed by ECDHE and RSA legacy suites in the
/// precise order emitted by the Chromium BoringSSL configuration.
const CIPHER_LIST: &str = concat!(
    "TLS_AES_128_GCM_SHA256:",
    "TLS_AES_256_GCM_SHA384:",
    "TLS_CHACHA20_POLY1305_SHA256:",
    "ECDHE-ECDSA-AES128-GCM-SHA256:",
    "ECDHE-RSA-AES128-GCM-SHA256:",
    "ECDHE-ECDSA-AES256-GCM-SHA384:",
    "ECDHE-RSA-AES256-GCM-SHA384:",
    "ECDHE-ECDSA-CHACHA20-POLY1305:",
    "ECDHE-RSA-CHACHA20-POLY1305:",
    "ECDHE-RSA-AES128-SHA:",
    "ECDHE-RSA-AES256-SHA:",
    "AES128-GCM-SHA256:",
    "AES256-GCM-SHA384:",
    "AES128-SHA:",
    "AES256-SHA"
);

/// Supported elliptic curve groups.
///
/// Group 4588 corresponds to X25519MLKEM768, Chrome's post-quantum hybrid group.
/// If this is missing from the ClientHello, the client is immediately flagged
/// as non-Chrome 131+.
const CURVES: &[u16] = &[4588, 29, 23, 24];

const ALPN_H2: &[u8] = b"h2";
const ALPN_HTTP_11: &[u8] = b"http/1.1";
const ALPN_PROTOCOLS: &[&[u8]] = &[ALPN_H2, ALPN_HTTP_11];

/// Signature algorithms in JA4_r order.
const SIGALGS: &[u16] = &[
    0x0403, // ecdsa_secp256r1_sha256
    0x0804, // rsa_pss_rsae_sha256
    0x0401, // rsa_pkcs1_sha256
    0x0503, // ecdsa_secp384r1_sha384
    0x0805, // rsa_pss_rsae_sha384
    0x0501, // rsa_pkcs1_sha384
    0x0806, // rsa_pss_rsae_sha512
    0x0601, // rsa_pkcs1_sha512
];

/// HTTP/2 pseudo-header ordering (m,a,s,p).
///
/// Moving `:authority` to the second position is a key Chrome-specific
/// marker that differs from standard HTTP/2 library defaults.
const PSEUDO_ORDER: [PseudoOrder; 4] = [
    PseudoOrder::Method,
    PseudoOrder::Authority,
    PseudoOrder::Scheme,
    PseudoOrder::Path,
];

/// Constructs a profile for Chrome 134 on Apple Silicon macOS.
pub fn chrome_134_macos_arm() -> ChromeProfile {
    ChromeProfile {
        version: 134,
        platform: Platform::MacOsArm,
        tls: TlsProfile {
            min_version: SslVersion::TLS1_2,
            max_version: SslVersion::TLS1_3,
            cipher_list: CIPHER_LIST,
            curves: CURVES,
            grease_enabled: true,
            permute_extensions: true,
            enable_ech_grease: true,
            alps_enabled: true,
            alps_use_new_codepoint: true,
            compress_certificate: true,
            session_ticket_enabled: true,
            alpn_protocols: ALPN_PROTOCOLS,
            sigalgs: SIGALGS,
        },
        h2: Http2Profile {
            settings: SettingsFrame {
                header_table_size: 65_536,
                enable_push: false,
                initial_window_size: 6_291_456,
                max_header_list_size: 262_144,
            },
            // The connection window determines the initial WINDOW_UPDATE delta.
            // Chrome uses 15663105, which results in a total connection window
            // of 15728640 (65535 + 15663105).
            initial_connection_window_size: 15_728_640,
            pseudo_order: PSEUDO_ORDER,
            headers_priority: HeadersPriority {
                dep: 0,
                weight: 255,
                exclusive: true,
            },
        },
        headers: HeaderProfile {
            user_agent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36".to_owned(),
            sec_ch_ua:
                "\"Chromium\";v=\"134\", \"Not:A-Brand\";v=\"24\", \"Google Chrome\";v=\"134\""
                    .to_owned(),
            sec_ch_ua_platform: "\"macOS\"".to_owned(),
            include_priority_header: true,
            zstd_encoding: true,
        },
    }
}

/// Generic accessor for a Chrome 134 profile on a specific platform.
pub fn profile(_platform: Platform) -> ChromeProfile {
    chrome_134_macos_arm()
}
