use std::collections::HashMap;
use std::sync::RwLock;

use url::Url;
use wreq::Client;

pub mod navigate;
pub use navigate::NavigationResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    Http2,
    Http3,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AltSvcInfo {
    pub h3: bool,
    pub max_age_secs: u64,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PhantomNetError {
    #[error("authority must not be empty")]
    EmptyAuthority,
    #[error("HTTP request failed: {0}")]
    RequestFailed(String),
    #[error("URL is invalid: {0}")]
    InvalidUrl(String),
}

pub struct FetchResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub final_url: String,
}

/// Minimal network transport policy surface for Phase 3 wiring.
///
/// The real client will hold h2/h3 implementations; this type currently
/// tracks Alt-Svc state and chooses which transport to use per authority.
pub struct SmartNetworkClient {
    persona_id: String,
    alt_svc_cache: RwLock<HashMap<String, AltSvcInfo>>,
    client: Client,
}

impl std::fmt::Debug for SmartNetworkClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SmartNetworkClient")
            .field("persona_id", &self.persona_id)
            .field("alt_svc_cache", &self.alt_svc_cache)
            .finish_non_exhaustive()
    }
}

impl Default for SmartNetworkClient {
    fn default() -> Self {
        Self::new("default")
    }
}

impl SmartNetworkClient {
    pub fn new(persona_id: impl Into<String>) -> Self {
        Self {
            persona_id: persona_id.into(),
            alt_svc_cache: RwLock::new(HashMap::new()),
            client: Client::new(),
        }
    }

    pub fn with_persona(persona: &phantom_anti_detect::Persona) -> Self {
        use phantom_anti_detect::ChromeProfile;

        let emulation = match persona.chrome_version {
            ChromeProfile::Chrome133 => wreq_util::Emulation::Chrome133,
            ChromeProfile::Chrome134 => wreq_util::Emulation::Chrome134,
        };

        let client = Client::builder()
            .emulation(emulation)
            .user_agent(&persona.user_agent)
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            persona_id: format!("{:?}", persona.chrome_version),
            alt_svc_cache: RwLock::new(HashMap::new()),
            client,
        }
    }

    pub fn persona_id(&self) -> &str {
        &self.persona_id
    }

    pub fn set_persona_id(&mut self, persona_id: impl Into<String>) {
        self.persona_id = persona_id.into();
    }

    pub fn record_alt_svc(
        &self, // changed to &self for RwLock usage
        authority: impl Into<String>,
        info: AltSvcInfo,
    ) -> Result<(), PhantomNetError> {
        let key = normalize_authority(&authority.into())?;
        if let Ok(mut cache) = self.alt_svc_cache.write() {
            cache.insert(key, info);
        }
        Ok(())
    }

    pub fn clear_alt_svc(&self, authority: &str) -> Result<bool, PhantomNetError> {
        let key = normalize_authority(authority)?;
        if let Ok(mut cache) = self.alt_svc_cache.write() {
            Ok(cache.remove(&key).is_some())
        } else {
            Ok(false)
        }
    }

    pub fn select_transport(&self, authority: &str) -> Result<Transport, PhantomNetError> {
        let key = normalize_authority(authority)?;
        let t = if let Ok(cache) = self.alt_svc_cache.read() {
            cache
                .get(&key)
                .map(|info| {
                    if info.h3 {
                        Transport::Http3
                    } else {
                        Transport::Http2
                    }
                })
                .unwrap_or(Transport::Http2)
        } else {
            Transport::Http2
        };
        Ok(t)
    }

    pub fn alt_svc_entries(&self) -> usize {
        if let Ok(cache) = self.alt_svc_cache.read() {
            cache.len()
        } else {
            0
        }
    }

    pub async fn fetch(&self, url: &str) -> Result<FetchResponse, PhantomNetError> {
        let parsed_url = Url::parse(url).map_err(|e| PhantomNetError::InvalidUrl(e.to_string()))?;

        let res = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| PhantomNetError::RequestFailed(e.to_string()))?;

        let status = res.status().as_u16();
        let final_url = res.uri().to_string();

        let mut headers_map = HashMap::new();
        for (k, v) in res.headers().iter() {
            if let Ok(value_str) = v.to_str() {
                headers_map.insert(k.as_str().to_string(), value_str.to_string());

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
                        .unwrap_or(86400);

                    if let Some(host) = parsed_url.host_str() {
                        let _ = self.record_alt_svc(host, AltSvcInfo { h3, max_age_secs });
                    }
                }
            }
        }

        let body = res
            .bytes()
            .await
            .map_err(|e| PhantomNetError::RequestFailed(e.to_string()))?
            .to_vec();

        Ok(FetchResponse {
            status,
            headers: headers_map,
            body,
            final_url,
        })
    }
}

fn normalize_authority(input: &str) -> Result<String, PhantomNetError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(PhantomNetError::EmptyAuthority);
    }
    Ok(trimmed.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::{AltSvcInfo, PhantomNetError, SmartNetworkClient, Transport};

    #[test]
    fn unknown_authority_defaults_to_h2() {
        let client = SmartNetworkClient::new("persona_a");
        assert_eq!(
            client.select_transport("example.com").unwrap(),
            Transport::Http2
        );
    }

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

    #[tokio::test]
    async fn fetch_invalid_url_returns_invalid_url_error() {
        use phantom_anti_detect::Persona;
        use rand::rngs::OsRng;
        use rand::RngCore;

        let persona = Persona::chrome_133(OsRng.next_u64());
        let client = SmartNetworkClient::with_persona(&persona);

        let err = match client.fetch("not-a-url").await {
            Ok(_) => panic!("invalid URL must not be accepted by fetch"),
            Err(err) => err,
        };
        assert!(
            matches!(err, PhantomNetError::InvalidUrl(_)),
            "invalid URL should map to PhantomNetError::InvalidUrl, got: {err}"
        );
    }

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
}
