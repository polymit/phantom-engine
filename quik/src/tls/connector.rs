use boring::ssl::{
    CertificateCompressionAlgorithm, CertificateCompressor, SslConnector, SslMethod,
};
use std::io::{Read, Write};

use crate::error::Result;
use crate::profile::TlsProfile;

/// Chrome 134 supported certificate compression algorithms.
/// Algorithm ID 2 is Brotli (RFC 8879).
pub struct BrotliCompressor;

impl CertificateCompressor for BrotliCompressor {
    const ALGORITHM: CertificateCompressionAlgorithm = CertificateCompressionAlgorithm::BROTLI;
    const CAN_COMPRESS: bool = false;
    const CAN_DECOMPRESS: bool = true;

    fn decompress<W>(&self, input: &[u8], output: &mut W) -> std::io::Result<()>
    where
        W: Write,
    {
        let mut reader = brotli::Decompressor::new(input, 4096);
        let mut buf = [0u8; 4096];
        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
            output.write_all(&buf[..n])?;
        }
        Ok(())
    }
}

/// Builds a BoringSSL connector that produces a Chrome 134-identical ClientHello.
///
/// This includes:
/// - Exact cipher suite order (15 suites)
/// - GREASE (ciphers and extensions)
/// - Extension permutation (randomized order per connection)
/// - Post-quantum key shares (X25519MLKEM768)
///
/// Note: ECH GREASE and ALPS must be applied to the Ssl object per-connection
/// in this version of boring.
pub fn build_connector(profile: &TlsProfile) -> Result<SslConnector> {
    tracing::debug!("Building TLS connector...");
    let mut builder = SslConnector::builder(SslMethod::tls_client())?;

    // TLS version bounds
    builder.set_min_proto_version(Some(profile.min_version))?;
    builder.set_max_proto_version(Some(profile.max_version))?;

    // Cipher list
    tracing::debug!("Setting cipher list: {}", profile.cipher_list);
    builder.set_cipher_list(profile.cipher_list)?;

    // Curves
    let mut curves_str = String::new();
    for (i, &group) in profile.curves.iter().enumerate() {
        if i > 0 {
            curves_str.push(':');
        }
        match group {
            4588 => curves_str.push_str("X25519Kyber768Draft00"),
            29 => curves_str.push_str("X25519"),
            23 => curves_str.push_str("P-256"),
            24 => curves_str.push_str("P-384"),
            _ => curves_str.push_str(&group.to_string()),
        }
    }
    tracing::debug!("Setting curves list: {}", curves_str);
    builder.set_curves_list(&curves_str)?;

    // GREASE and Extension Permutation
    if profile.grease_enabled {
        builder.set_grease_enabled(true);
    }
    if profile.permute_extensions {
        builder.set_permute_extensions(true);
    }

    // ALPN
    let mut alpn = Vec::new();
    for proto in profile.alpn_protocols {
        alpn.push(proto.len() as u8);
        alpn.extend_from_slice(proto);
    }
    builder.set_alpn_protos(&alpn)?;

    // SCT (Signed Certificate Timestamps)
    builder.enable_signed_cert_timestamps();

    // FFI for advanced Chrome features
    let ctx_ptr = builder.as_ptr();

    unsafe {
        tracing::debug!("Setting sigalgs via FFI");
        let sigalgs_i32: Vec<i32> = profile.sigalgs.iter().map(|&s| s as i32).collect();
        boring_sys::SSL_CTX_set1_sigalgs(ctx_ptr, sigalgs_i32.as_ptr(), sigalgs_i32.len());
    }

    // Certificate compression
    if profile.compress_certificate {
        tracing::debug!("Adding Brotli certificate compressor");
        builder.add_certificate_compression_algorithm(BrotliCompressor)?;
    }

    tracing::debug!("TLS connector built successfully");
    Ok(builder.build())
}
