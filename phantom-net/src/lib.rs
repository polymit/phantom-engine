use std::collections::HashMap;

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
}

/// Minimal network transport policy surface for Phase 3 wiring.
///
/// The real client will hold h2/h3 implementations; this type currently
/// tracks Alt-Svc state and chooses which transport to use per authority.
#[derive(Debug, Default)]
pub struct SmartNetworkClient {
    persona_id: String,
    alt_svc_cache: HashMap<String, AltSvcInfo>,
}

impl SmartNetworkClient {
    pub fn new(persona_id: impl Into<String>) -> Self {
        Self {
            persona_id: persona_id.into(),
            alt_svc_cache: HashMap::new(),
        }
    }

    pub fn persona_id(&self) -> &str {
        &self.persona_id
    }

    pub fn set_persona_id(&mut self, persona_id: impl Into<String>) {
        self.persona_id = persona_id.into();
    }

    pub fn record_alt_svc(
        &mut self,
        authority: impl Into<String>,
        info: AltSvcInfo,
    ) -> Result<(), PhantomNetError> {
        let key = normalize_authority(&authority.into())?;
        self.alt_svc_cache.insert(key, info);
        Ok(())
    }

    pub fn clear_alt_svc(&mut self, authority: &str) -> Result<bool, PhantomNetError> {
        let key = normalize_authority(authority)?;
        Ok(self.alt_svc_cache.remove(&key).is_some())
    }

    pub fn select_transport(&self, authority: &str) -> Result<Transport, PhantomNetError> {
        let key = normalize_authority(authority)?;
        let t = self
            .alt_svc_cache
            .get(&key)
            .map(|info| {
                if info.h3 {
                    Transport::Http3
                } else {
                    Transport::Http2
                }
            })
            .unwrap_or(Transport::Http2);
        Ok(t)
    }

    pub fn alt_svc_entries(&self) -> usize {
        self.alt_svc_cache.len()
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
    use super::{AltSvcInfo, SmartNetworkClient, Transport};

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
        let mut client = SmartNetworkClient::new("persona_a");
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
}
