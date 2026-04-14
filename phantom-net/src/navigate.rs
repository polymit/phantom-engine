use crate::{FetchResponse, PhantomNetError, SmartNetworkClient};
use phantom_core::pipeline::CoreError;
use phantom_core::process_html;
use phantom_serializer::{HeadlessSerializer, SerialiserConfig, SerialiserMode};
use std::collections::HashMap;
use tracing::{info, warn};
use url::Url;

#[derive(Debug, Clone)]
pub struct NavigationConfig {
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub max_retries: u8,
    pub task_hint: Option<String>,
}

impl Default for NavigationConfig {
    fn default() -> Self {
        Self {
            viewport_width: 1280.0,
            viewport_height: 720.0,
            max_retries: 2,
            task_hint: None,
        }
    }
}

pub struct NavigationResult {
    pub url: String,
    pub status: u16,
    pub cct: String,
    pub node_count: usize,
    pub tree: phantom_core::DomTree,
}

impl std::fmt::Debug for NavigationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NavigationResult")
            .field("url", &self.url)
            .field("status", &self.status)
            .field("cct_len", &self.cct.len())
            .field("node_count", &self.node_count)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NavigationError {
    #[error("network error fetching {url}: {source}")]
    Network {
        url: String,
        source: PhantomNetError,
    },

    #[error("HTTP error {status} fetching {url}")]
    HttpError { status: u16, url: String },

    #[error("HTML is not valid UTF-8 at {url}")]
    Encoding { url: String },

    #[error("core pipeline failed for {url}: {source}")]
    Pipeline { url: String, source: CoreError },

    #[error("unexpected redirect response {status} fetching {url}")]
    RedirectResponse {
        status: u16,
        url: String,
        location: Option<String>,
    },

    #[error("all {attempts} attempts failed for {url}")]
    AllAttemptsFailed { url: String, attempts: u8 },
}

pub async fn navigate(
    client: &SmartNetworkClient,
    url: &str,
    config: &NavigationConfig,
) -> Result<NavigationResult, NavigationError> {
    let max_attempts = config.max_retries + 1;
    let mut current_attempt = 1;
    let mut current_url = url.to_string();
    let mut redirect_count = 0;
    const MAX_REDIRECTS: u8 = 10;

    loop {
        if current_attempt > 1 {
            info!(
                "Retrying navigation to {} (attempt {}/{})",
                current_url, current_attempt, max_attempts
            );
        }

        let response = match client.fetch(&current_url).await {
            Ok(res) => res,
            Err(e) => {
                if let PhantomNetError::RequestFailed(ref _msg) = e {
                    warn!("Network request failed: {} for url {}", e, url);
                    if current_attempt < max_attempts {
                        current_attempt += 1;
                        continue;
                    } else {
                        return Err(NavigationError::AllAttemptsFailed {
                            url: current_url,
                            attempts: max_attempts,
                        });
                    }
                } else {
                    return Err(NavigationError::Network {
                        url: current_url,
                        source: e,
                    });
                }
            }
        };

        if response.status >= 300 && response.status < 400 {
            if let Some(location) = redirect_location(&response.headers) {
                if redirect_count >= MAX_REDIRECTS {
                    return Err(NavigationError::RedirectResponse {
                        status: response.status,
                        url: current_url,
                        location: Some(location),
                    });
                }

                // Resolve relative redirect against current URL
                let base = Url::parse(&current_url).map_err(|e| NavigationError::Network {
                    url: current_url.clone(),
                    source: PhantomNetError::InvalidUrl(e.to_string()),
                })?;
                let resolved = base.join(&location).map_err(|e| NavigationError::Network {
                    url: current_url.clone(),
                    source: PhantomNetError::InvalidUrl(format!(
                        "failed to resolve redirect '{}': {}",
                        location, e
                    )),
                })?;

                let next_url = resolved.to_string();
                info!("Following redirect to {}", next_url);
                current_url = next_url;
                redirect_count += 1;
                current_attempt = 1; // Reset retries on successful redirect hop
                continue;
            } else {
                return Err(NavigationError::RedirectResponse {
                    status: response.status,
                    url: current_url,
                    location: None,
                });
            }
        }

        if response.status >= 400 && response.status < 500 {
            return Err(NavigationError::HttpError {
                status: response.status,
                url: current_url,
            });
        }

        if response.status >= 500 && response.status < 600 {
            warn!("HTTP {} for url {}", response.status, url);
            if current_attempt < max_attempts {
                current_attempt += 1;
                continue;
            } else {
                return Err(NavigationError::AllAttemptsFailed {
                    url: current_url,
                    attempts: max_attempts,
                });
            }
        }

        // Now handling 200..=299
        let html = decode_body(&response).map_err(|_| NavigationError::Encoding {
            url: current_url.clone(),
        })?;

        let final_url = response.final_url;

        let final_url_clone = final_url.clone();
        let viewport_width = config.viewport_width;
        let viewport_height = config.viewport_height;
        let task_hint = config.task_hint.clone();

        let (tree, cct, node_count) = tokio::task::spawn_blocking(move || {
            let page = process_html(&html, &final_url_clone, viewport_width, viewport_height)
                .map_err(|e| NavigationError::Pipeline {
                    url: final_url_clone.clone(),
                    source: e,
                })?;

            let serialiser_mode = if task_hint.is_some() {
                SerialiserMode::Selective
            } else {
                SerialiserMode::Full
            };

            let serialiser_config = SerialiserConfig {
                url: final_url_clone,
                viewport_width,
                viewport_height,
                mode: serialiser_mode,
                task_hint,
                scroll_x: 0.0,
                scroll_y: 0.0,
                total_height: viewport_height,
            };

            let cct = HeadlessSerializer::serialise(&page, &serialiser_config);
            let node_count = cct.lines().filter(|line| !line.starts_with("##")).count();

            Ok::<_, NavigationError>((page.tree, cct, node_count))
        })
        .await
        .map_err(|e| NavigationError::Pipeline {
            url: final_url.clone(),
            source: CoreError::Parse(format!("blocking task panicked: {e}")),
        })??;

        info!(
            "Navigation successful: status {}, nodes {} for url {}",
            response.status, node_count, final_url
        );

        return Ok(NavigationResult {
            url: final_url,
            status: response.status,
            cct,
            node_count,
            tree,
        });
    }
}

fn redirect_location(headers: &HashMap<String, String>) -> Option<String> {
    headers.iter().find_map(|(k, v)| {
        if k.eq_ignore_ascii_case("location") {
            Some(v.clone())
        } else {
            None
        }
    })
}

fn decode_body(res: &FetchResponse) -> Result<String, String> {
    let content_type = res
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .map(|(_, v)| v.as_str())
        .unwrap_or("");

    let charset = content_type
        .split(';')
        .find_map(|p| {
            let p = p.trim();
            if p.to_lowercase().starts_with("charset=") {
                Some(&p[8..])
            } else {
                None
            }
        })
        .unwrap_or("utf-8");

    let encoding = encoding_rs::Encoding::for_label(charset.as_bytes()).unwrap_or(encoding_rs::UTF_8);

    let (decoded, _, malformed) = encoding.decode(&res.body);
    if malformed && charset.to_lowercase() == "utf-8" {
        return Err("malformed utf-8 body".to_string());
    }

    Ok(decoded.into_owned())
}

#[cfg(test)]
mod tests {
    use super::redirect_location;
    use std::collections::HashMap;

    #[test]
    fn redirect_location_is_case_insensitive() {
        let mut headers = HashMap::new();
        headers.insert(
            "Location".to_string(),
            "https://example.com/next".to_string(),
        );
        assert_eq!(
            redirect_location(&headers).as_deref(),
            Some("https://example.com/next")
        );
    }

    #[test]
    fn redirect_location_returns_none_when_missing() {
        let headers = HashMap::new();
        assert!(redirect_location(&headers).is_none());
    }
}
