use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use url::Url;

use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Proxy {
    Http(String),
    Socks5(String),
}

impl Proxy {
    pub fn parse(url: &str) -> Result<Self> {
        let parsed = Url::parse(url).map_err(|e| Error::InvalidUrl(e.to_string()))?;
        let addr = format!(
            "{}:{}",
            parsed.host_str().unwrap_or(""),
            parsed.port().unwrap_or(1080)
        );

        match parsed.scheme() {
            "http" => Ok(Proxy::Http(addr)),
            "socks5" => Ok(Proxy::Socks5(addr)),
            _ => Err(Error::InvalidUrl("Unsupported proxy scheme".to_string())),
        }
    }
}

pub async fn dial_proxy(proxy: &Proxy, target_host: &str, target_port: u16) -> Result<TcpStream> {
    match proxy {
        Proxy::Http(addr) => dial_http_proxy(addr, target_host, target_port).await,
        Proxy::Socks5(addr) => dial_socks5_proxy(addr, target_host, target_port).await,
    }
}

async fn dial_http_proxy(
    proxy_addr: &str,
    target_host: &str,
    target_port: u16,
) -> Result<TcpStream> {
    let mut stream = TcpStream::connect(proxy_addr).await?;

    let connect_req = format!(
        "CONNECT {}:{} HTTP/1.1\r\nHost: {}:{}\r\n\r\n",
        target_host, target_port, target_host, target_port
    );

    stream.write_all(connect_req.as_bytes()).await?;

    let mut buf = [0; 4096];
    let mut read_bytes = 0;

    loop {
        let n = stream.read(&mut buf[read_bytes..]).await?;
        if n == 0 {
            return Err(Error::Connect(std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                "Proxy closed connection",
            )));
        }
        read_bytes += n;

        let response = String::from_utf8_lossy(&buf[..read_bytes]);
        if response.contains("\r\n\r\n") {
            if response.starts_with("HTTP/1.1 200") || response.starts_with("HTTP/1.0 200") {
                return Ok(stream);
            } else {
                return Err(Error::Connect(std::io::Error::other(format!(
                    "HTTP proxy error: {}",
                    response
                ))));
            }
        }
    }
}

async fn dial_socks5_proxy(
    proxy_addr: &str,
    target_host: &str,
    target_port: u16,
) -> Result<TcpStream> {
    let mut stream = TcpStream::connect(proxy_addr).await?;

    // 1. Initial Handshake (No Auth for now)
    stream.write_all(&[0x05, 0x01, 0x00]).await?;

    let mut response = [0; 2];
    stream.read_exact(&mut response).await?;

    if response[0] != 0x05 || response[1] != 0x00 {
        return Err(Error::Connect(std::io::Error::other(
            "SOCKS5 handshake failed",
        )));
    }

    // 2. Connection Request
    let host_bytes = target_host.as_bytes();
    let mut req = vec![0x05, 0x01, 0x00, 0x03, host_bytes.len() as u8];
    req.extend_from_slice(host_bytes);
    req.extend_from_slice(&target_port.to_be_bytes());

    stream.write_all(&req).await?;

    let mut resp_header = [0; 4];
    stream.read_exact(&mut resp_header).await?;

    if resp_header[0] != 0x05 || resp_header[1] != 0x00 {
        return Err(Error::Connect(std::io::Error::other(
            "SOCKS5 connect failed",
        )));
    }

    let addr_type = resp_header[3];
    match addr_type {
        0x01 => {
            // IPv4
            let mut addr = [0; 4];
            stream.read_exact(&mut addr).await?;
        }
        0x03 => {
            // Domain
            let mut len = [0; 1];
            stream.read_exact(&mut len).await?;
            let mut domain = vec![0; len[0] as usize];
            stream.read_exact(&mut domain).await?;
        }
        0x04 => {
            // IPv6
            let mut addr = [0; 16];
            stream.read_exact(&mut addr).await?;
        }
        _ => {
            return Err(Error::Connect(std::io::Error::other(
                "Invalid SOCKS5 address type",
            )))
        }
    }

    let mut port = [0; 2];
    stream.read_exact(&mut port).await?;

    Ok(stream)
}
