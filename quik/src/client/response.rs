use bytes::Bytes;
use http2::RecvStream;
use std::io::Read;

use crate::error::{Error, Result};

/// A high-level wrapper around an HTTP/2 response.
pub struct Response {
    inner: http::Response<RecvStream>,
    final_url: String,
}

impl Response {
    pub fn new(inner: http::Response<RecvStream>, final_url: String) -> Self {
        Self { inner, final_url }
    }

    pub fn url(&self) -> &str {
        &self.final_url
    }

    pub fn set_url(&mut self, url: String) {
        self.final_url = url;
    }
    pub fn status(&self) -> http::StatusCode {
        self.inner.status()
    }

    pub fn headers(&self) -> &http::HeaderMap {
        self.inner.headers()
    }

    /// Collects the entire response body and returns it as Bytes.
    /// Handles automatic decompression (Brotli, Zstd, Gzip).
    pub async fn bytes(mut self) -> Result<Bytes> {
        let body = self.inner.body_mut();
        let mut data = Vec::new();

        while let Some(chunk) = body.data().await {
            let chunk = chunk?;
            data.extend_from_slice(chunk.as_ref());
        }

        // Decompression logic
        let encoding = self
            .headers()
            .get(http::header::CONTENT_ENCODING)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        match encoding {
            "br" => {
                let mut decoder = brotli_decompressor::Decompressor::new(&data[..], 4096);
                let mut decoded = Vec::new();
                decoder.read_to_end(&mut decoded)?;
                Ok(Bytes::from(decoded))
            }
            "zstd" => {
                let decoded = zstd::decode_all(&data[..])?;
                Ok(Bytes::from(decoded))
            }
            "gzip" | "deflate" => {
                // For simplicity, we could add flate2 later if needed.
                // Chrome 134 prefers br and zstd.
                Ok(Bytes::from(data))
            }
            _ => Ok(Bytes::from(data)),
        }
    }

    pub async fn text(self) -> Result<String> {
        let bytes = self.bytes().await?;
        String::from_utf8(bytes.to_vec()).map_err(|e| Error::InvalidUrl(e.to_string()))
    }

    pub async fn json<T: serde::de::DeserializeOwned>(self) -> Result<T> {
        let bytes = self.bytes().await?;
        serde_json::from_slice(&bytes).map_err(|e| Error::InvalidUrl(e.to_string()))
    }
}
