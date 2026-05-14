use bytes::Bytes;
use http::header::{HeaderMap, CONTENT_ENCODING};
use http::{Response as HttpResponse, StatusCode};
use http2::RecvStream;
use std::io::Read;

use crate::error::{Error, Result};

/// A high-level response wrapper providing transparent decompression and body management.
///
/// `Response` abstracts away the complexities of HTTP/2 stream management and
/// automatic decompression of browser-standard encodings.
pub struct Response {
    /// The underlying HTTP response containing status and headers.
    inner: HttpResponse<RecvStream>,
    /// The final, post-redirect URL that produced this response.
    url: String,
}

impl Response {
    /// Creates a new `Response` from a raw H2 response and origin URL.
    pub fn new(inner: HttpResponse<RecvStream>, url: String) -> Self {
        Self { inner, url }
    }

    /// Returns the HTTP status code.
    pub fn status(&self) -> StatusCode {
        self.inner.status()
    }

    /// Returns a reference to the header map.
    pub fn headers(&self) -> &HeaderMap {
        self.inner.headers()
    }

    /// Returns the final post-redirect URL.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Internal setter for the final URL (used by the redirect engine).
    pub(crate) fn set_url(&mut self, url: String) {
        self.url = url;
    }

    /// Collects the response body and returns the decompressed bytes.
    ///
    /// This method is async as it must wait for all HTTP/2 DATA frames to arrive.
    /// Supports `gzip`, `br`, and `zstd` encodings.
    pub async fn bytes(self) -> Result<Bytes> {
        let (parts, mut body_stream) = self.inner.into_parts();
        let mut data = Vec::new();

        while let Some(chunk) = body_stream.data().await {
            let chunk = chunk.map_err(Error::Http2)?;
            data.extend_from_slice(chunk.as_ref());
        }

        let encoding = parts
            .headers
            .get(CONTENT_ENCODING)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if encoding.contains("br") {
            let mut decoder = brotli_decompressor::Decompressor::new(&data[..], 4096);
            let mut decoded = Vec::new();
            decoder
                .read_to_end(&mut decoded)
                .map_err(|e| Error::Connect(std::io::Error::other(e.to_string())))?;
            Ok(Bytes::from(decoded))
        } else if encoding.contains("zstd") {
            let decoded = zstd::decode_all(&data[..])
                .map_err(|e| Error::Connect(std::io::Error::other(e.to_string())))?;
            Ok(Bytes::from(decoded))
        } else if encoding.contains("gzip") {
            let mut decoder = flate2::read::GzDecoder::new(&data[..]);
            let mut decoded = Vec::new();
            decoder
                .read_to_end(&mut decoded)
                .map_err(|e| Error::Connect(std::io::Error::other(e.to_string())))?;
            Ok(Bytes::from(decoded))
        } else {
            Ok(Bytes::from(data))
        }
    }

    /// Collects the body and decodes it as a UTF-8 string.
    pub async fn text(self) -> Result<String> {
        let bytes = self.bytes().await?;
        String::from_utf8(bytes.to_vec())
            .map_err(|e| Error::Connect(std::io::Error::other(e.to_string())))
    }

    /// Collects the body and decodes it as JSON.
    pub async fn json<T: serde::de::DeserializeOwned>(self) -> Result<T> {
        let bytes = self.bytes().await?;
        serde_json::from_slice(&bytes)
            .map_err(|e| Error::Connect(std::io::Error::other(e.to_string())))
    }
}
