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

/// A single Chrome 134-identical network connection.
pub struct QuikConnection {
    pub h2: SendRequest<Bytes>,
    pub profile: ChromeProfile,
}

/// Orchestrates the TCP → TLS → H2 connection pipeline.
pub async fn connect(
    host: &str,
    port: u16,
    addr: SocketAddr,
    profile: &ChromeProfile,
    proxy: Option<&Proxy>,
) -> Result<QuikConnection> {
    let tcp = if let Some(p) = proxy {
        dial_proxy(p, host, port).await?
    } else {
        TcpStream::connect(addr).await?
    };

    let connector = build_connector(&profile.tls)?;
    let mut config = connector.configure()?;

    // Request OCSP status (extension status_request)
    config.set_status_type(boring::ssl::StatusType::OCSP)?;

    let ssl_ptr = config.as_ptr();
    unsafe {
        if profile.tls.enable_ech_grease {
            boring_sys::SSL_set_enable_ech_grease(ssl_ptr, 1);
        }
        if profile.tls.alps_enabled {
            // Chrome 134 ALPS H2 settings: 1:65536, 2:0, 4:6291456, 6:262144
            let alps_data: [u8; 24] = [
                0, 1, 0, 1, 0, 0, // 1: 65536
                0, 2, 0, 0, 0, 0, // 2: 0
                0, 4, 0, 96, 0, 0, // 4: 6291456
                0, 6, 0, 4, 0, 0, // 6: 262144
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

    let tls_stream = tokio_boring::connect(config, host, tcp)
        .await
        .map_err(|e| {
            tracing::error!("TLS handshake failed: {:?}", e);
            e
        })?;

    let mut h2_builder = http2::client::Builder::new();
    configure_builder(&mut h2_builder, &profile.h2);

    let (h2, connection) = h2_builder.handshake(tls_stream).await?;

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
    /// Sends a pre-constructed HTTP request with the Chrome identity connection.
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
