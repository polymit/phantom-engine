use bytes::Bytes;
use foreign_types::ForeignTypeRef;
use http2::client::SendRequest;
use std::net::SocketAddr;
use tokio::net::TcpStream;

use crate::client::proxy::{dial_proxy, Proxy};
use crate::client::response::Response;
use crate::error::Result;
use crate::http2::configure_builder;
use crate::profile::ChromeProfile;
use crate::tls::build_connector;

/// Represents an established HTTP/2 connection with a fixed Chrome identity.
///
/// This structure holds the active H2 request handle and the profile used
/// to establish the connection. Reusing this connection ensures that all 
/// subsequent requests adhere to the same behavioral constraints (e.g., 
/// same SETTINGS, same window increments).
pub struct QuikConnection {
    /// The handle used to initiate new H2 streams.
    pub h2: SendRequest<Bytes>,
    /// The profile used for TLS and H2 handshake parity.
    pub profile: ChromeProfile,
}

/// Establishes a new network connection following the Chrome 134 transport pipeline.
///
/// This function orchestrates a multi-stage handshake to ensure the resulting
/// connection is indistinguishable from a real browser:
///
/// 1. **Proxy/TCP**: Dials the target host (optionally via a SOCKS5/HTTP tunnel).
/// 2. **TLS Handshake**: Performs a BoringSSL handshake with ClientHello permutation,
///    GREASE, and extension shuffling.
/// 3. **ALPS/ECH**: Injects per-connection application settings (ALPS) and ECH GREASE
///    via raw BoringSSL FFI calls.
/// 4. **H2 Handshake**: Negotiates the HTTP/2 session using a specialized builder that
///    replicates Chromium's SETTINGS frame order and connection window increments.
pub async fn connect(
    host: &str,
    port: u16,
    addr: SocketAddr,
    profile: &ChromeProfile,
    proxy: Option<&Proxy>,
) -> Result<QuikConnection> {
    // Stage 1: Establish raw TCP transport.
    let tcp = if let Some(p) = proxy {
        dial_proxy(p, host, port).await?
    } else {
        TcpStream::connect(addr).await?
    };

    // Stage 2: Configure the TLS connector.
    let connector = build_connector(&profile.tls)?;
    let mut config = connector.configure()?;

    // Request OCSP stapling to match Chrome's certificate verification behavior.
    config.set_status_type(boring::ssl::StatusType::OCSP)?;

    // Stage 3: Per-connection FFI for advanced Chrome features.
    let ssl_ptr = config.as_ptr();

    // SAFETY: The `ssl_ptr` is valid for the duration of the configuration phase.
    // We pass valid pointers for the ALPN protocol "h2" and the static ALPS buffer.
    // These calls are required because high-level Rust wrappers often do not yet 
    // expose the latest Chromium-specific BoringSSL features.
    unsafe {
        if profile.tls.enable_ech_grease {
            boring_sys::SSL_set_enable_ech_grease(ssl_ptr, 1);
        }
        if profile.tls.alps_enabled {
            // Chrome 134 ALPS H2 settings payload:
            // ID 1: 65536, ID 2: 0, ID 4: 6291456, ID 6: 262144
            let alps_data: [u8; 24] = [
                0, 1, 0, 1, 0, 0, 0, 2, 0, 0, 0, 0, 0, 4, 0, 96, 0, 0, 0, 6, 0, 4, 0, 0,
            ];

            boring_sys::SSL_add_application_settings(
                ssl_ptr,
                b"h2".as_ptr(),
                2,
                alps_data.as_ptr(),
                alps_data.len(),
            );
        }
    }

    // Stage 4: TLS handshake.
    let tls_stream = tokio_boring::connect(config, host, tcp)
        .await
        .map_err(|e| {
            tracing::error!("TLS handshake failed: {:?}", e);
            e
        })?;

    // Stage 5: HTTP/2 handshake.
    let mut h2_builder = http2::client::Builder::new();
    configure_builder(&mut h2_builder, &profile.h2);

    let (h2, connection) = h2_builder.handshake(tls_stream).await?;

    // Drive the connection in the background. If this task terminates,
    // the H2 session is considered dead.
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("HTTP/2 connection driver failed: {:?}", e);
        }
    });

    Ok(QuikConnection {
        h2,
        profile: profile.clone(),
    })
}

impl QuikConnection {
    /// Dispatches an HTTP request over the established H2 session.
    pub async fn send(
        &mut self,
        request: http::Request<()>,
        body: Option<Bytes>,
    ) -> Result<Response> {
        let url_str = request.uri().to_string();
        if let Some(data) = body {
            let (response_future, mut send_stream) = self.h2.send_request(request, false)?;
            send_stream.send_data(data, true)?;
            let response = response_future.await?;
            Ok(Response::new(response, url_str))
        } else {
            let (response_future, _) = self.h2.send_request(request, true)?;
            let response = response_future.await?;
            Ok(Response::new(response, url_str))
        }
    }
}
