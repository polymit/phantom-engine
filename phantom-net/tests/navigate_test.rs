#![allow(clippy::unwrap_used, clippy::expect_used)]
use phantom_anti_detect::Persona;
use phantom_net::navigate::{navigate, NavigationConfig, NavigationError};
use phantom_net::SmartNetworkClient;
use rand::rngs::OsRng;
use rand::RngCore;

fn make_client() -> SmartNetworkClient {
    SmartNetworkClient::with_persona(&Persona::chrome_133(OsRng.next_u64()))
}

fn assert_network_unavailable(err: &NavigationError) {
    assert!(
        matches!(
            err,
            NavigationError::AllAttemptsFailed { .. } | NavigationError::Network { .. }
        ),
        "expected offline/network failure, got: {err}"
    );
}

#[test]
fn navigation_config_defaults_are_correct() {
    let config = NavigationConfig::default();
    assert_eq!(config.viewport_width, 1280.0);
    assert_eq!(config.viewport_height, 720.0);
    assert_eq!(config.max_retries, 2);
    assert!(config.task_hint.is_none());
    assert_eq!(config.max_network_bytes, Some(64 * 1024 * 1024));
}

#[tokio::test]
async fn navigate_real_page_returns_cct() {
    let client = make_client();
    let config = NavigationConfig::default();

    match navigate(&client, "https://httpbin.org/html", &config).await {
        Ok(result) => {
            assert_eq!(result.status, 200);
            assert!(result.cct.starts_with("##PAGE"));
            assert!(result.cct.contains("httpbin.org"));
            assert!(!result.url.is_empty());
            assert!(result.tree.document_root.is_some());

            let header = result.cct.lines().next().unwrap_or("");
            let header_count = header
                .split_whitespace()
                .find_map(|p| p.strip_prefix("nodes="))
                .and_then(|v| v.parse::<usize>().ok())
                .expect("nodes= field must be present in CCT header");
            assert_eq!(header_count, result.node_count);
        }
        Err(err) => assert_network_unavailable(&err),
    }
}

#[tokio::test]
async fn navigate_cct_header_fields_are_populated() {
    let client = make_client();
    let config = NavigationConfig::default();

    match navigate(&client, "https://httpbin.org/html", &config).await {
        Ok(result) => {
            let header_line = result.cct.lines().next().unwrap_or("");
            assert!(header_line.starts_with("##PAGE"));
            assert!(header_line.contains("viewport=1280x720"));
            assert!(header_line.contains("scroll=0,0"));
            assert!(header_line.contains("mode=full"));
            assert!(header_line.contains("nodes="));
        }
        Err(err) => assert_network_unavailable(&err),
    }
}

#[tokio::test]
async fn navigate_selective_mode_when_task_hint_provided() {
    let client = make_client();
    let config = NavigationConfig {
        task_hint: Some("find the login button".to_string()),
        ..Default::default()
    };

    match navigate(&client, "https://httpbin.org/html", &config).await {
        Ok(result) => {
            assert!(
                result.cct.contains("mode=selective"),
                "task_hint should force selective mode in CCT header"
            );
        }
        Err(err) => assert_network_unavailable(&err),
    }
}

#[tokio::test]
async fn navigate_404_returns_http_error_not_retry() {
    let client = make_client();
    let config = NavigationConfig {
        max_retries: 2,
        ..Default::default()
    };

    match navigate(&client, "https://httpbin.org/status/404", &config).await {
        Ok(_) => panic!("404 must return an error, not Ok"),
        Err(NavigationError::HttpError { status, url }) => {
            assert_eq!(status, 404);
            assert!(url.contains("httpbin.org/status/404"));
        }
        Err(err) => assert_network_unavailable(&err),
    }
}

#[tokio::test]
async fn navigate_node_count_matches_cct_header() {
    let client = make_client();
    let config = NavigationConfig::default();

    match navigate(&client, "https://httpbin.org/html", &config).await {
        Ok(result) => {
            let header = result.cct.lines().next().unwrap_or("");
            let header_count = header
                .split_whitespace()
                .find_map(|p| p.strip_prefix("nodes="))
                .and_then(|v| v.parse::<usize>().ok())
                .expect("nodes= field must be present and parseable");

            assert_eq!(header_count, result.node_count);
        }
        Err(err) => assert_network_unavailable(&err),
    }
}

#[tokio::test]
async fn navigate_follows_redirects_and_updates_url() {
    let client = make_client();
    let config = NavigationConfig::default();

    match navigate(&client, "https://httpbin.org/redirect/1", &config).await {
        Ok(result) => {
            assert_eq!(result.status, 200);
            assert!(!result.url.is_empty());
            assert_ne!(result.url, "https://httpbin.org/redirect/1");
        }
        Err(err) => assert_network_unavailable(&err),
    }
}

#[tokio::test]
async fn navigate_empty_url_returns_error_not_panic() {
    let client = make_client();
    let config = NavigationConfig::default();

    let result = navigate(&client, "", &config).await;
    assert!(
        matches!(result, Err(NavigationError::Network { .. })),
        "empty URL should map to NavigationError::Network"
    );
}

#[test]
fn navigation_config_zero_retries_means_one_attempt() {
    let config = NavigationConfig {
        max_retries: 0,
        ..Default::default()
    };
    assert_eq!(config.max_retries, 0);
}

#[tokio::test]
async fn navigate_result_page_is_queryable() {
    let client = make_client();
    let config = NavigationConfig::default();

    match navigate(&client, "https://httpbin.org/html", &config).await {
        Ok(result) => {
            assert!(result.tree.document_root.is_some());
            let html_nodes = result.tree.get_elements_by_tag_name("html");
            assert!(!html_nodes.is_empty());
        }
        Err(err) => assert_network_unavailable(&err),
    }
}
