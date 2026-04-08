use std::time::Duration;
use std::time::Instant;
use tokio::time::timeout;

use phantom_core::process_html;
use phantom_mcp::engine::SessionPage;
use phantom_mcp::{EngineAdapter, McpServer};

const INTEGRATION_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head><meta charset="UTF-8"><title>Phase 3 Integration — Login</title></head>
<body style="width:1280px;height:720px;">
    <header style="width:1280px;height:60px;">
        <nav aria-label="Main" style="width:1280px;height:60px;">
            <a href="/" style="width:50px;height:20px;">Home</a>
        </nav>
    </header>
    <main style="width:600px;height:500px;">
        <h1 style="width:400px;height:50px;">Sign In</h1>
        <form id="login-form" style="width:400px;height:300px;">
            <input id="email-input" type="email" placeholder="Email"
                   required aria-required="true"
                   style="width:200px;height:35px;"/>
            <input id="password-input" type="password"
                   placeholder="Password" required
                   style="width:200px;height:35px;"/>
            <button id="login-btn" type="submit" data-testid="login-btn"
                    style="width:120px;height:45px;">Sign in</button>
            <a href="/forgot" style="width:150px;height:20px;">
                Forgot password?</a>
        </form>
    </main>
    <footer style="width:1280px;height:50px;">
        <p style="width:400px;height:20px;">© 2026 Example Corp</p>
    </footer>
</body>
</html>"#;

async fn setup_with_page() -> (EngineAdapter, McpServer) {
    let adapter = EngineAdapter::new(5, 0, 5, 0).await;
    let page = process_html(INTEGRATION_HTML, "https://local.test/login", 1280.0, 720.0).unwrap();
    adapter.store_page(SessionPage::new(
        page,
        "https://local.test/login".to_string(),
        200,
    ));
    let server = McpServer::new_with_adapter(None, adapter.clone());
    (adapter, server)
}

#[tokio::test]
async fn phase3_full_pipeline_navigate_and_scene_graph() {
    let (adapter, server) = setup_with_page().await;
    let req = McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_get_scene_graph","params":{}}"#,
    )
    .unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();

    let result = resp.result.unwrap();
    let cct = result.get("cct").unwrap().as_str().unwrap();

    assert!(cct.starts_with("##PAGE"));
    assert!(cct.contains("viewport=1280x720"));
    assert!(cct.contains("mode=full"));
    assert!(cct.contains("Sign In"));
    assert!(cct.contains("login-btn"));

    // node_count > 0 condition
    let node_count = result.get("node_count").unwrap().as_u64().unwrap();
    assert!(node_count > 0);

    println!("=== PHASE 3 SCENE GRAPH DUMP ===");
    println!("{}", cct);
    println!("================================");
}

#[tokio::test]
async fn phase3_click_login_button() {
    let (adapter, server) = setup_with_page().await;
    let req = McpServer::parse_request(
        r##"{"jsonrpc":"2.0","id":1,"method":"browser_click","params":{"selector":"#login-btn"}}"##,
    )
    .unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();

    assert!(resp.error.is_none());
    let result = resp.result.unwrap();
    assert_eq!(result.get("clicked").unwrap().as_bool(), Some(true));
    let hesitation = result.get("hesitation_ms").unwrap().as_u64().unwrap();
    assert!((20..=500).contains(&hesitation));
    assert_eq!(
        result.get("selector").unwrap().as_str().unwrap(),
        "#login-btn"
    );
}

#[tokio::test]
async fn phase3_type_into_email_field() {
    let (adapter, server) = setup_with_page().await;
    let req = McpServer::parse_request(r##"{"jsonrpc":"2.0","id":1,"method":"browser_type","params":{"selector":"#email-input","text":"agent@example.com","delay_ms":0}}"##).unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();

    assert!(resp.error.is_none());
    let result = resp.result.unwrap();
    assert_eq!(result.get("typed").unwrap().as_bool(), Some(true));
    assert_eq!(result.get("characters").unwrap().as_u64(), Some(17));
}

#[tokio::test]
async fn phase3_press_tab_to_advance_focus() {
    let (adapter, server) = setup_with_page().await;
    let req = McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_press_key","params":{"key":"Tab"}}"#,
    )
    .unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();

    assert!(resp.error.is_none(), "{:?}", resp.error);
    let result = resp.result.unwrap();
    assert_eq!(result.get("pressed").unwrap().as_bool(), Some(true));
    assert_eq!(result.get("key").unwrap().as_str(), Some("Tab"));
}

#[tokio::test]
async fn phase3_evaluate_document_title() {
    let (adapter, server) = setup_with_page().await;
    let req = McpServer::parse_request(r#"{"jsonrpc":"2.0","id":1,"method":"browser_evaluate","params":{"script":"document.title"}}"#).unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();

    assert!(resp.error.is_none());
    let result = resp.result.unwrap();
    assert_eq!(
        result.get("type").and_then(|v| v.as_str()),
        Some("string"),
        "document.title should evaluate to a string"
    );
    assert_eq!(
        result.get("result").and_then(|v| v.as_str()),
        Some("Phase 3 Integration — Login"),
        "document.title value should match the fixture title"
    );
}

#[tokio::test]
async fn phase3_evaluate_query_selector_count() {
    let (adapter, server) = setup_with_page().await;
    let req = McpServer::parse_request(r#"{"jsonrpc":"2.0","id":1,"method":"browser_evaluate","params":{"script":"document.querySelectorAll('input').length"}}"#).unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();

    assert!(resp.error.is_none());
    let result = resp.result.unwrap();
    assert_eq!(
        result.get("type").and_then(|v| v.as_str()),
        Some("number"),
        "querySelectorAll(...).length should evaluate to a number"
    );
    assert_eq!(
        result.get("result").and_then(|v| v.as_u64()),
        Some(2),
        "fixture should contain exactly two input elements"
    );
}

#[tokio::test]
async fn phase3_cookies_workflow() {
    let (adapter, server) = setup_with_page().await;
    let req = McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_get_cookies","params":{}}"#,
    )
    .unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();
    assert_eq!(
        resp.result
            .unwrap()
            .get("cookies")
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        0
    );
    // 2
    let req2 = McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":2,"method":"browser_clear_cookies","params":{}}"#,
    )
    .unwrap();
    let resp2 = server.handle_request(&adapter, req2, None).await.unwrap();
    assert_eq!(
        resp2.result.unwrap().get("cleared").unwrap().as_bool(),
        Some(true)
    );
    // 3
    let req3 = McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":3,"method":"browser_get_cookies","params":{}}"#,
    )
    .unwrap();
    let resp3 = server.handle_request(&adapter, req3, None).await.unwrap();
    assert_eq!(
        resp3
            .result
            .unwrap()
            .get("cookies")
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        0
    );
}

#[tokio::test]
async fn phase3_snapshot_and_verify() {
    let (adapter, server) = setup_with_page().await;
    let req = McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_session_snapshot","params":{}}"#,
    )
    .unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();
    let res = resp.result.unwrap();
    let path = res.get("snapshot_path").unwrap().as_str().unwrap();
    let size = res.get("size_bytes").unwrap().as_u64().unwrap();
    assert!(size > 0);
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(&bytes[0..4], &[0x28, 0xB5, 0x2F, 0xFD]); // zstd magic
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn phase3_tab_full_lifecycle() {
    let adapter = EngineAdapter::new(5, 0, 5, 0).await;
    let server = McpServer::new_with_adapter(None, adapter.clone());

    // new_tab
    let req1 = McpServer::parse_request(r#"{"jsonrpc":"2.0","id":1,"method":"browser_new_tab","params":{"url":"https://tab1.test"}}"#).unwrap();
    let resp1 = server.handle_request(&adapter, req1, None).await.unwrap();
    let id1 = resp1
        .result
        .unwrap()
        .get("tab_id")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();

    // new_tab
    let req2 = McpServer::parse_request(r#"{"jsonrpc":"2.0","id":2,"method":"browser_new_tab","params":{"url":"https://tab2.test"}}"#).unwrap();
    let resp2 = server.handle_request(&adapter, req2, None).await.unwrap();
    let id2 = resp2
        .result
        .unwrap()
        .get("tab_id")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();

    // list_tabs
    let req3 = McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":3,"method":"browser_list_tabs","params":{}}"#,
    )
    .unwrap();
    let resp3 = server.handle_request(&adapter, req3, None).await.unwrap();
    let tabs = resp3
        .result
        .unwrap()
        .get("tabs")
        .unwrap()
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(tabs.len(), 2);

    // switch_tab
    let req4 = McpServer::parse_request(&format!(
        r#"{{"jsonrpc":"2.0","id":4,"method":"browser_switch_tab","params":{{"tab_id":"{}"}}}}"#,
        id1
    ))
    .unwrap();
    server.handle_request(&adapter, req4, None).await.unwrap();

    // list active
    let req5 = McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":5,"method":"browser_list_tabs","params":{}}"#,
    )
    .unwrap();
    let resp5 = server.handle_request(&adapter, req5, None).await.unwrap();
    let tabs_after = resp5
        .result
        .unwrap()
        .get("tabs")
        .unwrap()
        .as_array()
        .unwrap()
        .clone();
    let active_tab = tabs_after
        .iter()
        .find(|t| t.get("active").unwrap().as_bool().unwrap())
        .unwrap();
    assert_eq!(active_tab.get("id").unwrap().as_str().unwrap(), id1);

    // close
    let req6 = McpServer::parse_request(&format!(
        r#"{{"jsonrpc":"2.0","id":6,"method":"browser_close_tab","params":{{"tab_id":"{}"}}}}"#,
        id2
    ))
    .unwrap();
    server.handle_request(&adapter, req6, None).await.unwrap();

    // list remaining
    let req7 = McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":7,"method":"browser_list_tabs","params":{}}"#,
    )
    .unwrap();
    let resp7 = server.handle_request(&adapter, req7, None).await.unwrap();
    assert_eq!(
        resp7
            .result
            .unwrap()
            .get("tabs")
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn phase3_sse_subscribes_and_receives_delta() {
    let (adapter, _server) = setup_with_page().await;
    let mut rx = adapter.delta_tx.subscribe();
    adapter.inject_delta("##SCROLL 0,100".to_string());
    let msg = timeout(Duration::from_millis(100), rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(msg, "##SCROLL 0,100");
}

#[tokio::test]
async fn phase3_selective_mode_on_large_page() {
    let adapter = EngineAdapter::new(5, 0, 5, 0).await;
    let mut html = String::from("<html><body style='width:1280px;height:720px;'>");
    for i in 0..600 {
        html.push_str(&format!(
            "<div id='d{}' style='width:20px;height:10px;'>div {}</div>",
            i, i
        ));
    }
    html.push_str("</body></html>");
    let page = process_html(&html, "https://large.test", 1280.0, 720.0).unwrap();
    adapter.store_page(SessionPage::new(
        page,
        "https://large.test".to_string(),
        200,
    ));

    let server = McpServer::new_with_adapter(None, adapter.clone());
    let req = McpServer::parse_request(r#"{"jsonrpc":"2.0","id":1,"method":"browser_get_scene_graph","params":{"mode":"selective"}}"#).unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();

    let result = resp.result.unwrap();
    let cct = result.get("cct").unwrap().as_str().unwrap();
    assert_eq!(
        result.get("mode").and_then(|v| v.as_str()),
        Some("selective")
    );
    assert!(
        cct.contains("mode=selective"),
        "CCT header should reflect selective mode"
    );
    let count = result.get("node_count").unwrap().as_u64().unwrap();
    let emitted_count = cct.lines().filter(|line| !line.starts_with("##")).count() as u64;
    assert_eq!(
        count, emitted_count,
        "node_count should match emitted nodes"
    );
}

#[tokio::test]
async fn phase3_all_mcp_tools_are_registered() {
    let adapter = EngineAdapter::new(5, 0, 5, 0).await;
    let server = McpServer::new_with_adapter(None, adapter.clone());
    let tools = vec![
        "browser_navigate",
        "browser_get_scene_graph",
        "browser_click",
        "browser_type",
        "browser_press_key",
        "browser_evaluate",
        "browser_new_tab",
        "browser_switch_tab",
        "browser_list_tabs",
        "browser_close_tab",
        "browser_get_cookies",
        "browser_set_cookie",
        "browser_clear_cookies",
        "browser_session_snapshot",
    ];
    for t in tools {
        let req = McpServer::parse_request(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"{}","params":{{}}}}"#,
            t
        ))
        .unwrap();
        let resp = server.handle_request(&adapter, req, None).await.unwrap();
        if let Some(err) = resp.error {
            assert_ne!(err.code, -32601, "{} is not registered", t);
        }
    }
}

#[tokio::test]
async fn phase3_real_page_navigation() {
    let adapter = EngineAdapter::new(5, 0, 5, 0).await;
    let server = McpServer::new_with_adapter(None, adapter.clone());

    let start = Instant::now();
    let req = McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_navigate","params":{"url":"https://httpbin.org/html"}}"#,
    )
    .unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();
    match resp.error {
        None => {
            let req2 = McpServer::parse_request(
                r#"{"jsonrpc":"2.0","id":2,"method":"browser_get_scene_graph","params":{}}"#,
            )
            .unwrap();
            let resp2 = server.handle_request(&adapter, req2, None).await.unwrap();
            let result = resp2.result.unwrap();
            let cct = result.get("cct").unwrap().as_str().unwrap();
            assert!(cct.starts_with("##PAGE"));
            assert!(cct.contains("httpbin.org"));
            println!("Navigation took: {}ms", start.elapsed().as_millis());
        }
        Some(err) => {
            assert!(
                err.message.contains("all_attempts_failed")
                    || err.message.contains("network_error"),
                "unexpected browser_navigate failure: {}",
                err.message
            );
        }
    }
}
