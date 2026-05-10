//! Single source of truth for all Chrome 134 fingerprint constants.
//! Every layer (TLS, H2, headers) reads from here.
//! If Chrome rotates a value, update it here only.

use boring::ssl::SslVersion;

use crate::profile::{
    ChromeProfile, HeaderProfile, HeadersPriority, Http2Profile, Platform, PseudoOrder,
    SettingsFrame, TlsProfile,
};

pub const JA3_HASH: &str = "845db3b4e398789bdeb5b15594360a29";
pub const JA3N_HASH: &str = "8e19337e7524d2573be54efb2b0784c9";
pub const JA4: &str = "t13d1516h2_8daaf6152771_d8a2da3f94cd";
pub const AKAMAI_FINGERPRINT: &str = "1:65536;2:0;4:6291456;6:262144|15663105|0|m,a,s,p";

// 4865 TLS_AES_128_GCM_SHA256
// 4866 TLS_AES_256_GCM_SHA384
// 4867 TLS_CHACHA20_POLY1305_SHA256
// 49195 ECDHE-ECDSA-AES128-GCM-SHA256
// 49199 ECDHE-RSA-AES128-GCM-SHA256
// 49196 ECDHE-ECDSA-AES256-GCM-SHA384
// 49200 ECDHE-RSA-AES256-GCM-SHA384
// 52393 ECDHE-ECDSA-CHACHA20-POLY1305
// 52392 ECDHE-RSA-CHACHA20-POLY1305
// 49171 ECDHE-RSA-AES128-SHA
// 49172 ECDHE-RSA-AES256-SHA
// 156 AES128-GCM-SHA256
// 157 AES256-GCM-SHA384
// 47 AES128-SHA
// 53 AES256-SHA
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

// `4588` is X25519MLKEM768, Chrome's post-quantum hybrid group:
// ML-KEM-768 contributes a 1184-byte public key and X25519 contributes 32
// bytes. If it is missing from JA3's groups field, the client fingerprints
// as non-Chrome 131+ immediately.
const CURVES: &[u16] = &[4588u16, 29, 23, 24];

const ALPN_H2: &[u8] = b"h2";
const ALPN_HTTP_11: &[u8] = b"http/1.1";
const ALPN_PROTOCOLS: &[&[u8]] = &[ALPN_H2, ALPN_HTTP_11];

// JA4_r order:
// 0x0403 ecdsa_secp256r1_sha256
// 0x0804 rsa_pss_rsae_sha256
// 0x0401 rsa_pkcs1_sha256
// 0x0503 ecdsa_secp384r1_sha384
// 0x0805 rsa_pss_rsae_sha384
// 0x0501 rsa_pkcs1_sha384
// 0x0806 rsa_pss_rsae_sha512
// 0x0601 rsa_pkcs1_sha512
const SIGALGS: &[u16] = &[
    0x0403u16, 0x0804, 0x0401, 0x0503, 0x0805, 0x0501, 0x0806, 0x0601,
];

// Akamai encodes this as `m,a,s,p` in its fingerprint string. The standard
// `h2` crate emits `:method :scheme :path :authority`, which is wrong; moving
// `:authority` into slot 2 is the most visible H2 fingerprint signal.
const PSEUDO_ORDER: [PseudoOrder; 4] = [
    PseudoOrder::Method,
    PseudoOrder::Authority,
    PseudoOrder::Scheme,
    PseudoOrder::Path,
];

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
            // 65535 (the RFC 7540 default connection window) + 15663105
            // (Chrome's handshake `WINDOW_UPDATE` delta) = 15728640. Akamai
            // fingerprints the delta, while the builder stores the total.
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
pub fn profile(_platform: Platform) -> ChromeProfile {
    chrome_134_macos_arm()
}
