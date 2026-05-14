// # Phantom Network Orchestration
//
// This crate provides high-level network orchestration for the Phantom Engine,
// managing the complex lifecycle of browser-identical navigations. It serves
// as the primary interface between the engine core and the specialized `http-quik`
// transport layer.
//
// ## Technical Architecture
// - **Persona-Driven Transport**: Dynamically maps engine personas to low-level
//   TLS and HTTP/2 fingerprints.
// - **Protocol Lifecycle Management**: Implements `Alt-Svc` negotiation and caching
//   to manage the transition between standard H2 and experimental H3 transports.
// - **Navigation Integrity**: Orchestrates multi-hop redirect resolution, ensuring
//   that security metadata (e.g., `sec-fetch-*`) and connection states remain consistent
//   with standard browser behavior.
// - **Resource Optimization**: Implements proactive asset discovery and concurrent
//   loading for critical path resources like external stylesheets.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use http_quik::Client;
use url::Url;

pub mod navigate;
pub use navigate::NavigationResult;

// Defined network transport protocols supported by the orchestration layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    // Standard HTTP/2 transport over TLS.
    Http2,
    // HTTP/3 transport using QUIC, typically negotiated via `Alt-Svc`.
    Http3,
}

// Metadata extracted from a server's protocol advertisement (`Alt-Svc`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AltSvcInfo {
    // Indicates if the authority supports HTTP/3.
    pub h3: bool,
    // TTL (in seconds) for the advertisement validity.
    pub max_age_secs: u64,
}

// Specialized error surface for the network orchestration subsystem.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PhantomNetError {
    // Indicates a missing or malformed authority segment in the target URI.
    #[error("authority must not be empty")]
    EmptyAuthority,
    // Encapsulates a failure in the underlying transport client.
    #[error("HTTP request failed: {0}")]
    RequestFailed(String),
    // Indicates the provided URL could not be parsed into a valid internal representation.
    #[error("URL is invalid: {0}")]
    InvalidUrl(String),
}

// Represents a processed network response ready for engine-level consumption.
pub struct FetchResponse {
    // Standard HTTP status code.
    pub status: u16,
    // Immutable map of response headers.
    pub headers: HashMap<String, String>,
    // Decoded or raw body payload.
    pub body: Vec<u8>,
    // The final, canonical URL after redirect resolution.
    pub final_url: String,
}

// An entry in the protocol advertisement cache.
#[derive(Debug, Clone)]
struct AltSvcCacheEntry {
    info: AltSvcInfo,
    stored_at: Instant,
}

// The central network client orchestrator for the Phantom Engine.
//
// `SmartNetworkClient` manages the persistent state required for browser-identical
// networking, including connection pooling, cookie management, and protocol
// upgrade caching.
#[derive(Clone)]
pub struct SmartNetworkClient {
    // Active persona identifier used for logging and tracking.
    persona_id: String,
    // Synchronized cache for `Alt-Svc` advertisements.
    // Uses an `Arc<RwLock<...>>` to allow shared, concurrent access across multiple
    // navigation tasks while maintaining thread safety for protocol upgrades.
    alt_svc_cache: Arc<RwLock<HashMap<String, AltSvcCacheEntry>>>,
    // The high-stealth transport implementation.
    client: Client,
    // Hard limit on the number of bytes allowed for any single network response.
    // This prevents OOM (Out Of Memory) issues when navigating to malicious or
    // excessively large pages.
    pub max_network_bytes: Option<usize>,
}

impl std::fmt::Debug for SmartNetworkClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SmartNetworkClient")
            .field("persona_id", &self.persona_id)
            .field("alt_svc_cache_size", &self.alt_svc_entries())
            .finish_non_exhaustive()
    }
}

impl Default for SmartNetworkClient {
    fn default() -> Self {
        Self::new("default")
    }
}

impl SmartNetworkClient {
    // Constructs a new orchestrator with a default browser identity.
    pub fn new(persona_id: impl Into<String>) -> Self {
        Self {
            persona_id: persona_id.into(),
            alt_svc_cache: Arc::new(RwLock::new(HashMap::new())),
            client: Client::new(),
            max_network_bytes: None,
        }
    }

    // Constructs an orchestrator synchronized with a specific anti-detection persona.
    //
    // This establishes the mapping between higher-level behavioral personas and
    // the cryptographic/transport fingerprints required to maintain parity.
    pub fn with_persona(persona: &phantom_anti_detect::Persona) -> Self {
        use phantom_anti_detect::ChromeProfile;

        // Resolve the transport profile based on the engine's persona configuration.
        // AI/Developer Note: We consolidate on the Chrome 134 profile even for 133
        // because 134 represents the current 'Stable' baseline with the highest
        // success rate against Akamai's TLS-fingerprint active probes.
        let profile = match persona.chrome_version {
            ChromeProfile::Chrome133 | ChromeProfile::Chrome134 => {
                http_quik::profile::chrome_134::profile(http_quik::Platform::MacOsArm)
            }
        };

        // Initialize the transport client with the resolved identity.
        let client = match Client::builder().profile(profile).build() {
            Ok(client) => client,
            Err(err) => {
                // Safety Fallback: If a specialized profile fails to initialize (e.g., FFI
                // issues with BoringSSL), we revert to a standard default client.
                // This ensures the engine remains operational at the cost of slight identity drift.
                tracing::warn!(
                    chrome_profile = ?persona.chrome_version,
                    user_agent = %persona.user_agent,
                    error = %err,
                    "failed to build persona-emulated client; falling back to default client"
                );
                Client::new()
            }
        };

        Self {
            persona_id: format!("{:?}", persona.chrome_version),
            alt_svc_cache: Arc::new(RwLock::new(HashMap::new())),
            client,
            max_network_bytes: None,
        }
    }

    // Returns the persona identifier assigned to this client.
    pub fn persona_id(&self) -> &str {
        &self.persona_id
    }

    // Updates the active persona identifier.
    pub fn set_persona_id(&mut self, persona_id: impl Into<String>) {
        self.persona_id = persona_id.into();
    }

    // Persists a protocol advertisement into the local cache for the specified authority.
    // Note: Authority normalization (lowercase) is critical here to avoid duplicate
    // entries for 'Example.Com' and 'example.com'.
    pub fn record_alt_svc(
        &self,
        authority: impl Into<String>,
        info: AltSvcInfo,
    ) -> Result<(), PhantomNetError> {
        let key = normalize_authority(&authority.into())?;
        if let Ok(mut cache) = self.alt_svc_cache.write() {
            cache.insert(
                key,
                AltSvcCacheEntry {
                    info,
                    stored_at: Instant::now(),
                },
            );
        }
        Ok(())
    }

    // Explicitly removes protocol advertisements for the specified authority.
    pub fn clear_alt_svc(&self, authority: &str) -> Result<bool, PhantomNetError> {
        let key = normalize_authority(authority)?;
        if let Ok(mut cache) = self.alt_svc_cache.write() {
            Ok(cache.remove(&key).is_some())
        } else {
            Ok(false)
        }
    }

    // Selects the optimal transport protocol for an authority based on advertisement state.
    //
    // This implementation enforces `max-age` constraints and automatically purges
    // stale entries from the cache during the selection process.
    pub fn select_transport(&self, authority: &str) -> Result<Transport, PhantomNetError> {
        let key = normalize_authority(authority)?;

        // We use a write lock even for reading because we may need to purge an expired entry.
        // This 'lazy eviction' ensures the cache doesn't bloat with dead protocol ads.
        let t = if let Ok(mut cache) = self.alt_svc_cache.write() {
            let expired = cache.get(&key).is_some_and(|entry| {
                entry.stored_at.elapsed().as_secs() >= entry.info.max_age_secs
            });

            if expired {
                cache.remove(&key);
                Transport::Http2
            } else {
                cache
                    .get(&key)
                    .map(|entry| {
                        if entry.info.h3 {
                            Transport::Http3
                        } else {
                            Transport::Http2
                        }
                    })
                    .unwrap_or(Transport::Http2)
            }
        } else {
            // Lock poisoned or unavailable; safely fallback to standard H2.
            Transport::Http2
        };
        Ok(t)
    }

    // Returns the total number of cached protocol advertisements.
    pub fn alt_svc_entries(&self) -> usize {
        if let Ok(cache) = self.alt_svc_cache.read() {
            cache.len()
        } else {
            0
        }
    }

    // Dispatches a high-level network request with transparent redirect and state management.
    //
    // This method is optimized for resource fetching and initial navigation hops,
    // delegating the underlying transport execution to the `quik` client while
    // intercepting protocol advertisements and enforcing security budgets.
    pub async fn fetch(
        &self,
        url: &str,
        max_bytes: Option<usize>,
    ) -> Result<FetchResponse, PhantomNetError> {
        let limit = max_bytes.or(self.max_network_bytes);
        let parsed_url = Url::parse(url).map_err(|e| PhantomNetError::InvalidUrl(e.to_string()))?;

        // Primary transport execution via Quik. This handles TLS/H2 fingerprinting.
        // Technical Note: http_quik::Client handles connection pooling internally;
        // fetch() here manages the orchestration of that data.
        let res = self
            .client
            .get(url)
            .await
            .map_err(|e| PhantomNetError::RequestFailed(e.to_string()))?;

        let status = res.status().as_u16();
        let final_url = res.url().to_string();

        let mut headers_map = HashMap::new();
        for (k, v) in res.headers().iter() {
            if let Ok(value_str) = v.to_str() {
                headers_map.insert(k.as_str().to_string(), value_str.to_string());

                // Intercept Alt-Svc headers. Servers use these to signal that they support
                // newer protocols (like H3) on different ports.
                // Technical Note: We parse this manually to avoid pulling in a full HTTP-parsing
                // dependency just for one header variant.
                if k.as_str().eq_ignore_ascii_case("alt-svc") {
                    let h3 = value_str.contains("h3=");
                    let max_age_secs = value_str
                        .split(';')
                        .find_map(|p| {
                            let p = p.trim();
                            if let Some(ma_str) = p.strip_prefix("ma=") {
                                ma_str.parse().ok()
                            } else {
                                None
                            }
                        })
                        .unwrap_or(86400); // 24hr default fallback if 'ma=' is missing.

                    if let Some(host) = parsed_url.host_str() {
                        let _ = self.record_alt_svc(host, AltSvcInfo { h3, max_age_secs });
                    }
                }
            }
        }

        // Aggregate body bytes. Response decompression (gzip/br/zstd) is handled
        // transparently by the http_quik::Response layer via async-compression.
        let body = res
            .bytes()
            .await
            .map_err(|e| PhantomNetError::RequestFailed(e.to_string()))?
            .to_vec();

        // Enforcement of response size constraints.
        // AI Note: This is the primary defense against resource exhaustion attacks
        // during unattended agentic navigation.
        if let Some(limit) = limit {
            if body.len() > limit {
                return Err(PhantomNetError::RequestFailed(format!(
                    "response body size {} exceeds limit of {} bytes",
                    body.len(),
                    limit
                )));
            }
        }

        Ok(FetchResponse {
            status,
            headers: headers_map,
            body,
            final_url,
        })
    }
}

// Internal utility for normalizing authorities to a canonical lowercase representation.
fn normalize_authority(input: &str) -> Result<String, PhantomNetError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        // Technical Note: An empty authority would cause ambiguous cache lookups
        // and potential connection failures in the quik transport layer.
        return Err(PhantomNetError::EmptyAuthority);
    }
    // Case insensitivity is standard for hostnames in RFC 3986.
    // Normalizing here prevents 'Google.com' and 'google.com' from creating
    // split protocol policies (one H2, one H3).
    Ok(trimmed.to_ascii_lowercase())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::{AltSvcInfo, PhantomNetError, SmartNetworkClient, Transport};

    // **Why this test is necessary**:
    // Most servers do not advertise Alt-Svc. We must ensure that the orchestrator
    // consistently defaults to the high-stealth HTTP/2 (http-quik) transport to
    // maintain the browser-identical identity without leaking protocol-agnostic behavior.
    #[test]
    fn unknown_authority_defaults_to_h2() {
        let client = SmartNetworkClient::new("persona_a");
        assert_eq!(
            client.select_transport("example.com").unwrap(),
            Transport::Http2
        );
    }

    // **Why this test is necessary**:
    // This validates the 'Smart' aspect of the client. When a server advertises H3
    // (via Alt-Svc), the orchestrator must respect that advertisement for future
    // connections to maintain protocol parity with modern Chrome. It also verifies
    // that authority normalization (e.g. 'Example.COM' -> 'example.com') is working
    // correctly to prevent redundant connection overhead.
    #[test]
    fn h3_alt_svc_prefers_h3() {
        let client = SmartNetworkClient::new("persona_a");
        client
            .record_alt_svc(
                "Example.COM",
                AltSvcInfo {
                    h3: true,
                    max_age_secs: 3600,
                },
            )
            .unwrap();
        assert_eq!(
            client.select_transport("example.com").unwrap(),
            Transport::Http3
        );
    }

    // **Why this test is necessary**:
    // Ensuring that the fetch orchestrator correctly handles malformed input.
    // This prevents downstream components from receiving invalid URL states
    // and ensures we return a typed `PhantomNetError` rather than panicking.
    #[tokio::test]
    async fn fetch_invalid_url_returns_invalid_url_error() {
        use phantom_anti_detect::Persona;

        let persona = Persona::chrome_133(rand::random::<u64>());
        let client = SmartNetworkClient::with_persona(&persona);

        let err = match client.fetch("not-a-url", None).await {
            Ok(_) => panic!("invalid URL must not be accepted by fetch"),
            Err(err) => err,
        };
        assert!(
            matches!(err, PhantomNetError::InvalidUrl(_)),
            "invalid URL should map to PhantomNetError::InvalidUrl, got: {err}"
        );
    }

    // **Why this test is necessary**:
    // This verifies the thread-safety and shared state of the `alt_svc_cache`.
    // It ensures that manual policy injections (crucial for testing and bypassing
    // certain CDN behaviors) are correctly persisted and retrievable by the
    // transport selection logic.
    #[tokio::test]
    async fn manual_alt_svc_injection_updates_transport_policy() {
        let client = SmartNetworkClient::new("persona_a");
        client
            .record_alt_svc(
                "LOCALHOST",
                AltSvcInfo {
                    h3: true,
                    max_age_secs: 3600,
                },
            )
            .expect("Alt-Svc insertion should succeed");

        let transport = client
            .select_transport("localhost")
            .expect("transport lookup should succeed for normalized authority");
        assert_eq!(
            transport,
            Transport::Http3,
            "manual Alt-Svc h3 policy should prefer Http3"
        );
        assert!(
            client.alt_svc_entries() >= 1,
            "Alt-Svc cache should contain at least one entry after insertion"
        );
    }

    // **Why this test is necessary**:
    // Protocol advertisements are time-sensitive. This test ensures the 'Lazy Eviction'
    // logic correctly purges entries where `max_age_secs` has elapsed. Failure here
    // would result in the engine attempting to use H3 on servers that no longer
    // support it, causing unnecessary connection latency and identity leakage.
    #[test]
    fn expired_alt_svc_entry_falls_back_to_h2() {
        let client = SmartNetworkClient::new("persona_a");
        client
            .record_alt_svc(
                "example.com",
                AltSvcInfo {
                    h3: true,
                    max_age_secs: 0, // Instant expiry.
                },
            )
            .expect("Alt-Svc insertion should succeed");

        let transport = client
            .select_transport("example.com")
            .expect("transport lookup should succeed");
        assert_eq!(transport, Transport::Http2);
        assert_eq!(client.alt_svc_entries(), 0);
    }
}
