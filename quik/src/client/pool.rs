use std::collections::HashMap;
use std::net::ToSocketAddrs;
use std::sync::{Arc, Mutex};
use url::Url;

use crate::client::connector::{connect, QuikConnection};
use crate::client::proxy::Proxy;
use crate::client::request::inject_chrome_headers;
use crate::client::response::Response;
use crate::error::{Error, Result};
use crate::profile::{ChromeProfile, Platform};

use bytes::Bytes;
use cookie_store::CookieStore;
use std::sync::RwLock;

/// A stateful, pooling HTTP client that enforces Chrome 134 identity.
///
/// The `Client` is the primary entry point for the `quik` library. It manages:
/// 1. **Connection Pooling**: Reuses established HTTP/2 sessions to maintain persistent fingerprints.
/// 2. **Cookie Persistence**: A synchronized cookie jar shared across all requests.
/// 3. **Stealth Redirects**: Automatically follows redirects while mutating headers and methods
///    to match Chromium's behavioral markers.
///
/// # Example
/// ```rust
/// use quik::Client;
/// 
/// let client = Client::new();
/// ```
#[derive(Clone)]
pub struct Client {
    /// A synchronized pool of active H2 connections keyed by their origin and proxy.
    pool: Arc<Mutex<HashMap<String, QuikConnection>>>,
    /// The canonical identity profile used for all transport-layer operations.
    profile: ChromeProfile,
    /// An optional proxy used for all outbound connections.
    proxy: Option<Proxy>,
    /// A synchronized cookie jar shared across all requests.
    ///
    /// This store is thread-safe and is automatically updated during redirect
    /// chains and standard request execution.
    pub cookie_store: Arc<RwLock<CookieStore>>,
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    /// Creates a new `Client` with a default Chrome 134 macOS profile.
    ///
    /// This is a convenience method that builds a client with Apple Silicon
    /// hardware signatures. For custom profiles or proxies, use [`Client::builder`].
    pub fn new() -> Self {
        Self::builder().build().unwrap_or_else(|_| Client {
            pool: Arc::new(Mutex::new(HashMap::new())),
            profile: crate::profile::chrome_134::profile(Platform::MacOsArm),
            proxy: None,
            cookie_store: Arc::new(RwLock::new(CookieStore::default())),
        })
    }

    /// Returns a [`ClientBuilder`] to configure a specialized `Client` instance.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// Executes a GET request and follows redirects stealthily.
    pub async fn get(&self, url: &str) -> Result<Response> {
        self.execute_with_redirects("GET", url, None).await
    }

    /// Executes a POST request and follows redirects stealthily.
    pub async fn post(&self, url: &str, body: Bytes) -> Result<Response> {
        self.execute_with_redirects("POST", url, Some(body)).await
    }

    /// Core request execution engine with automated, stateful redirect handling.
    ///
    /// This method implements a high-fidelity Chromium redirect state machine:
    ///
    /// 1. **Sec-Fetch-Site Evolution**: Dynamically calculates origin relationships 
    ///    (same-origin, same-site, cross-site) across hops to maintain stealth.
    /// 2. **Header Mutation**: Automatically strips `sec-fetch-user` and 
    ///    `upgrade-insecure-requests` after the first hop, exactly like Chrome.
    /// 3. **Method Rotation**: Rotates POST requests to GET for 301, 302, and 303 
    ///    status codes to prevent out-of-spec behavioral markers.
    /// 4. **H2 Multiplexing**: Reuses existing connections from the pool to avoid 
    ///    redundant TLS handshakes that could trigger anti-bot alerts.
    async fn execute_with_redirects(
        &self,
        initial_method: &str,
        initial_url: &str,
        initial_body: Option<Bytes>,
    ) -> Result<Response> {
        let mut current_url_str = initial_url.to_string();
        let mut current_method = initial_method.to_string();
        let mut current_body = initial_body;

        let mut sec_fetch_site = "none".to_string();
        let mut is_cross_site = false;

        for hop in 0..10 {
            let parsed_url =
                Url::parse(&current_url_str).map_err(|e| Error::InvalidUrl(e.to_string()))?;
            let authority = parsed_url
                .host_str()
                .ok_or_else(|| Error::InvalidUrl("missing host".to_string()))?;
            let port = parsed_url.port().unwrap_or(443);

            // Build a unique pool key considering the proxy and target origin.
            let proxy_prefix = self
                .proxy
                .as_ref()
                .map(|p| match p {
                    Proxy::Http(a) => format!("http://{}@", a),
                    Proxy::Socks5(a) => format!("socks5://{}@", a),
                })
                .unwrap_or_default();

            let key = format!("{}{}:{}", proxy_prefix, authority, port);

            // Extract relevant cookies for the current target URL.
            let cookie_header = {
                let store = self
                    .cookie_store
                    .read()
                    .map_err(|_| Error::Connect(std::io::Error::other("cookie store poisoned")))?;
                let cookies: Vec<_> = store
                    .matches(&parsed_url)
                    .iter()
                    .map(|c| format!("{}={}", c.name(), c.value()))
                    .collect();
                if cookies.is_empty() {
                    None
                } else {
                    Some(cookies.join("; "))
                }
            };

            let mut request = http::Request::builder()
                .method(current_method.as_str())
                .uri(parsed_url.as_str())
                .body(())
                .map_err(|e| Error::InvalidUrl(e.to_string()))?;

            if let Some(c) = cookie_header.as_deref() {
                if let Ok(val) = http::header::HeaderValue::from_str(c) {
                    request.headers_mut().insert("cookie", val);
                }
            }

            // Injects Chrome-identical headers, handling dynamic Sec-Fetch and Priority states.
            let is_initial = hop == 0;
            inject_chrome_headers(
                request.headers_mut(),
                &self.profile,
                &sec_fetch_site,
                is_initial,
            );

            // Connection acquisition logic.
            let conn = {
                let mut pool = self.pool.lock().map_err(|_| {
                    Error::Connect(std::io::Error::other("connection pool poisoned"))
                })?;
                pool.remove(&key)
            };

            let mut h2_client = if let Some(mut c) = conn {
                // Verify if the pooled connection is still active and ready for a new stream.
                match c.h2.ready().await {
                    Ok(h2) => {
                        c.h2 = h2;
                        c
                    }
                    Err(_) => self.dial(authority, port, &self.profile).await?,
                }
            } else {
                self.dial(authority, port, &self.profile).await?
            };

            let mut response = h2_client.send(request, current_body.clone()).await?;

            // Return the connection to the pool for potential reuse.
            if let Ok(mut pool) = self.pool.lock() {
                pool.insert(key, h2_client);
            }

            self.store_cookies(&response, &parsed_url);

            let status = response.status();
            if status.is_redirection() {
                if let Some(location) = response.headers().get("location") {
                    let loc_str = location.to_str().unwrap_or("");
                    let next_url = parsed_url
                        .join(loc_str)
                        .map_err(|e| Error::InvalidUrl(e.to_string()))?;

                    // Redirect Mutation: Rotate POST to GET on standard redirects.
                    if status == http::StatusCode::MOVED_PERMANENTLY
                        || status == http::StatusCode::FOUND
                        || status == http::StatusCode::SEE_OTHER
                    {
                        current_method = "GET".to_string();
                        current_body = None;
                    }

                    // sec-fetch-site computation: Once cross-site, always cross-site.
                    if !is_cross_site {
                        if next_url.origin() == parsed_url.origin() {
                            sec_fetch_site = "same-origin".to_string();
                        } else if next_url.domain() == parsed_url.domain() {
                            sec_fetch_site = "same-site".to_string();
                        } else {
                            sec_fetch_site = "cross-site".to_string();
                            is_cross_site = true;
                        }
                    }

                    current_url_str = next_url.to_string();
                    continue;
                }
            }

            response.set_url(current_url_str);
            return Ok(response);
        }

        Err(Error::Connect(std::io::Error::other(
            "Redirect limit exceeded (max 10)",
        )))
    }

    /// Dials a new connection following the profile's transport constraints.
    async fn dial(
        &self,
        authority: &str,
        port: u16,
        profile: &ChromeProfile,
    ) -> Result<QuikConnection> {
        let addr_str = format!("{}:{}", authority, port);
        let addr = addr_str.to_socket_addrs()?.next().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "could not resolve host")
        })?;

        connect(authority, port, addr, profile, self.proxy.as_ref()).await
    }

    /// Persists `Set-Cookie` headers from a response into the synchronized cookie store.
    fn store_cookies(&self, resp: &Response, url: &Url) {
        if let Ok(mut store) = self.cookie_store.write() {
            for v in resp.headers().get_all("set-cookie").iter() {
                if let Ok(cookie_str) = v.to_str() {
                    let _ = store.parse(cookie_str, url);
                }
            }
        }
    }
}

/// A builder for constructing a `Client` with specific identity and transport settings.
#[derive(Default)]
pub struct ClientBuilder {
    profile: Option<ChromeProfile>,
    proxy: Option<Proxy>,
    cookie_store: Option<Arc<RwLock<CookieStore>>>,
}

impl ClientBuilder {
    /// Sets the Chrome identity profile.
    pub fn profile(mut self, profile: ChromeProfile) -> Self {
        self.profile = Some(profile);
        self
    }

    /// Configures an outbound proxy.
    pub fn proxy(mut self, proxy: Proxy) -> Self {
        self.proxy = Some(proxy);
        self
    }

    /// Provides a pre-existing synchronized cookie store.
    pub fn cookie_store(mut self, store: Arc<RwLock<CookieStore>>) -> Self {
        self.cookie_store = Some(store);
        self
    }

    /// Finalizes the configuration and constructs a `Client`.
    pub fn build(self) -> Result<Client> {
        let profile = self
            .profile
            .unwrap_or_else(|| crate::profile::chrome_134::profile(Platform::MacOsArm));

        Ok(Client {
            pool: Arc::new(Mutex::new(HashMap::new())),
            profile,
            proxy: self.proxy,
            cookie_store: self
                .cookie_store
                .unwrap_or_else(|| Arc::new(RwLock::new(CookieStore::default()))),
        })
    }
}
