use crate::{PhantomNetError, SmartNetworkClient};
use phantom_core::pipeline::CoreError;
use phantom_core::{process_html, ParsedPage};
use phantom_serializer::{HeadlessSerializer, SerialiserConfig, SerialiserMode};
use tracing::{info, warn};

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
    pub page: ParsedPage,
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

    #[error("redirect loop detected for {url}")]
    RedirectLoop { url: String },

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

    loop {
        if current_attempt > 1 {
            info!(
                "Retrying navigation to {} (attempt {}/{})",
                url, current_attempt, max_attempts
            );
        }

        let response = match client.fetch(url).await {
            Ok(res) => res,
            Err(e) => {
                if let PhantomNetError::RequestFailed(ref _msg) = e {
                    warn!("Network request failed: {} for url {}", e, url);
                    if current_attempt < max_attempts {
                        current_attempt += 1;
                        continue;
                    } else {
                        return Err(NavigationError::AllAttemptsFailed {
                            url: url.to_string(),
                            attempts: max_attempts,
                        });
                    }
                } else {
                    return Err(NavigationError::Network {
                        url: url.to_string(),
                        source: e,
                    });
                }
            }
        };

        if response.status >= 300 && response.status < 400 {
            return Err(NavigationError::RedirectLoop {
                url: url.to_string(),
            });
        }

        if response.status >= 400 && response.status < 500 {
            return Err(NavigationError::HttpError {
                status: response.status,
                url: url.to_string(),
            });
        }

        if response.status >= 500 && response.status < 600 {
            warn!("HTTP {} for url {}", response.status, url);
            if current_attempt < max_attempts {
                current_attempt += 1;
                continue;
            } else {
                return Err(NavigationError::AllAttemptsFailed {
                    url: url.to_string(),
                    attempts: max_attempts,
                });
            }
        }

        // Now handling 200..=299
        let html = String::from_utf8(response.body).map_err(|_| NavigationError::Encoding {
            url: url.to_string(),
        })?;

        let final_url = response.final_url;

        let page = process_html(
            &html,
            &final_url,
            config.viewport_width,
            config.viewport_height,
        )
        .map_err(|e| NavigationError::Pipeline {
            url: final_url.clone(),
            source: e,
        })?;

        let serialiser_mode = if config.task_hint.is_some() {
            SerialiserMode::Selective
        } else {
            SerialiserMode::Full
        };

        let serialiser_config = SerialiserConfig {
            url: final_url.clone(),
            viewport_width: config.viewport_width,
            viewport_height: config.viewport_height,
            mode: serialiser_mode,
            task_hint: config.task_hint.clone(),
            scroll_x: 0.0,
            scroll_y: 0.0,
            total_height: config.viewport_height,
        };

        let cct = HeadlessSerializer::serialise(&page, &serialiser_config);

        let node_count = cct.lines().filter(|line| !line.starts_with("##")).count();

        info!(
            "Navigation successful: status {}, nodes {} for url {}",
            response.status, node_count, final_url
        );

        return Ok(NavigationResult {
            url: final_url,
            status: response.status,
            cct,
            node_count,
            page,
        });
    }
}
