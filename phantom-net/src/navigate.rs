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
    pub max_network_bytes: Option<usize>,
}

impl Default for NavigationConfig {
    fn default() -> Self {
        Self {
            viewport_width: 1280.0,
            viewport_height: 720.0,
            max_retries: 2,
            task_hint: None,
            max_network_bytes: Some(64 * 1024 * 1024), // 64MB default
        }
    }
}

pub struct NavigationResult {
    pub url: String,
    pub status: u16,
    pub cct: String,
    pub node_count: usize,
    pub tree: phantom_core::DomTree,
    /// CPU-only pipeline time in milliseconds (parse + layout + serialise).
    /// Excludes network I/O wait time.
    pub pipeline_ms: Option<u64>,
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

        let response = match client.fetch(&current_url, config.max_network_bytes).await {
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

        // 1. Initial parse to find external stylesheets
        let tree = phantom_core::parser::parse_html(&html);
        let mut external_css_urls = Vec::new();
        if let Some(root) = tree.document_root {
            collect_external_css_links(&tree, root, &final_url, &mut external_css_urls);
        }

        // 2. Fetch external CSS concurrently
        let mut external_css = Vec::new();
        if !external_css_urls.is_empty() {
            let mut fetch_tasks = Vec::new();
            for css_url in external_css_urls {
                let client = client.clone();
                let max_bytes = config.max_network_bytes;
                fetch_tasks.push(tokio::spawn(async move {
                    client.fetch(&css_url, max_bytes).await
                }));
            }

            for task in fetch_tasks {
                if let Ok(Ok(resp)) = task.await {
                    if resp.status == 200 {
                        if let Ok(css_text) = decode_body(&resp) {
                            external_css.push(css_text);
                        }
                    }
                }
            }
        }

        let (tree, cct, node_count, pipeline_ms) = tokio::task::spawn_blocking(move || {
            let pipeline_start = std::time::Instant::now();

            let page = process_html(
                &html,
                &final_url_clone,
                viewport_width,
                viewport_height,
                external_css,
            )
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
            let pipeline_elapsed = pipeline_start.elapsed().as_millis() as u64;

            Ok::<_, NavigationError>((page.tree, cct, node_count, pipeline_elapsed))
        })
        .await
        .map_err(|e| NavigationError::Pipeline {
            url: final_url.clone(),
            source: CoreError::Parse(format!("blocking task panicked: {e}")),
        })??;

        info!(
            "Navigation successful: status {}, nodes {}, pipeline {}ms for url {}",
            response.status, node_count, pipeline_ms, final_url
        );

        return Ok(NavigationResult {
            url: final_url,
            status: response.status,
            cct,
            node_count,
            tree,
            pipeline_ms: Some(pipeline_ms),
        });
    }
}

fn collect_external_css_links(
    tree: &phantom_core::DomTree,
    node_id: phantom_core::NodeId,
    base_url: &str,
    urls: &mut Vec<String>,
) {
    if let Some(node) = tree.get(node_id) {
        if let phantom_core::dom::NodeData::Element {
            tag_name,
            attributes,
        } = &node.data
        {
            if tag_name.eq_ignore_ascii_case("link")
                && attributes
                    .get("rel")
                    .is_some_and(|r| r.eq_ignore_ascii_case("stylesheet"))
            {
                if let Some(href) = attributes.get("href") {
                    if let Ok(base) = Url::parse(base_url) {
                        if let Ok(resolved) = base.join(href) {
                            urls.push(resolved.to_string());
                        }
                    }
                }
            }
        }
    }

    for child in node_id.children(&tree.arena) {
        collect_external_css_links(tree, child, base_url, urls);
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

    let encoding =
        encoding_rs::Encoding::for_label(charset.as_bytes()).unwrap_or(encoding_rs::UTF_8);

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
