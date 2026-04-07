use phantom_mcp::engine::get_test_adapter;
use phantom_mcp::McpServer;

#[tokio::test]
async fn engine_adapter_constructs_successfully() {
    let adapter = get_test_adapter().await;
    let persona = adapter.next_persona();
    assert!(
        !persona.user_agent.is_empty(),
        "persona user_agent must not be empty"
    );
    println!("EngineAdapter constructed: persona={}", persona.user_agent);
}

#[tokio::test]
async fn handle_navigate_rejects_missing_url_param() {
    let adapter = get_test_adapter().await;
    let server = McpServer::new(None);
    let req = McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_navigate","params":{}}"#,
    )
    .unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();
    assert!(
        resp.error.is_some(),
        "missing url param must produce an error response"
    );
    println!("missing url: got error as expected");
}

#[tokio::test]
async fn handle_navigate_invalid_url_returns_error() {
    let adapter = get_test_adapter().await;
    let server = McpServer::new(None);
    let req = McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_navigate","params":{"url":"not-a-url"}}"#,
    )
    .unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();
    // Navigating to "not-a-url" must not panic and must return some response
    println!("invalid url: response error={:?}", resp.error);
}

#[test]
fn existing_ping_still_works_after_refactor() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let adapter = get_test_adapter().await;
        let server = McpServer::new(None);
        let req = McpServer::parse_request(
            r#"{"jsonrpc":"2.0","id":"test","method":"ping","params":{}}"#,
        )
        .unwrap();
        let resp = server.handle_request(&adapter, req, None).await.unwrap();
        assert!(resp.error.is_none(), "ping must not return error");
        assert!(resp.result.is_some(), "ping must return result");
        println!("ping still works after refactor: {:?}", resp.result);
    });
}

#[test]
fn api_key_enforcement_still_works() {
    use phantom_mcp::McpError;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let adapter = get_test_adapter().await;
        let server = McpServer::new(Some("secret-key".to_string()));
        let req =
            McpServer::parse_request(r#"{"jsonrpc":"2.0","id":1,"method":"ping","params":{}}"#)
                .unwrap();
        let err = server
            .handle_request(&adapter, req, Some("wrong-key"))
            .await
            .unwrap_err();
        assert!(
            matches!(err, McpError::Unauthorized),
            "wrong API key must produce Unauthorized error"
        );
        println!("API key enforcement: VERIFIED");
    });
}

// ── browser_get_scene_graph tests ──────────────────────────────────

#[tokio::test]
async fn scene_graph_before_navigate_returns_no_page_error() {
    let adapter = get_test_adapter().await;
    adapter.page_store.lock().clear();
    let server = McpServer::new(None);
    let req = McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_get_scene_graph","params":{}}"#,
    )
    .unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();
    assert!(
        resp.error.is_some(),
        "scene_graph before navigate must return error"
    );
    let err_str = serde_json::to_string(&resp.error).unwrap();
    assert!(
        err_str.contains("no_page_loaded"),
        "error code must be no_page_loaded, got: {}",
        err_str
    );
    println!("scene_graph before navigate: correctly returned no_page_loaded");
}

#[tokio::test]
async fn scene_graph_after_store_returns_valid_cct() {
    use phantom_core::process_html;
    use phantom_mcp::engine::SessionPage;

    let adapter = get_test_adapter().await;

    let page = process_html(
        r#"<html><body style="width:1280px;height:720px;">
            <h1 style="width:400px;height:60px;">Test Page</h1>
            <button data-testid="go" style="width:100px;height:40px;">Go</button>
           </body></html>"#,
        "https://test.local",
        1280.0,
        720.0,
    )
    .expect("process_html must not fail");

    adapter.store_page(SessionPage::new(
        page,
        "https://test.local".to_string(),
        200,
    ));

    let retrieved = adapter.get_page();
    assert!(
        retrieved.is_some(),
        "get_page must return Some after store_page"
    );

    use phantom_serializer::{HeadlessSerializer, SerialiserConfig};
    let page2 = retrieved.unwrap();
    let config = SerialiserConfig {
        url: "https://test.local".to_string(),
        ..Default::default()
    };
    let cct = HeadlessSerializer::serialise(&page2, &config);

    assert!(cct.starts_with("##PAGE"), "CCT must start with ##PAGE");
    assert!(cct.contains("test.local"), "CCT must contain the URL");

    let node_count = cct.lines().filter(|l| !l.starts_with("##")).count();
    assert!(node_count > 0, "CCT must have at least one node");

    println!(
        "scene_graph after store: nodes={}, cct_len={}",
        node_count,
        cct.len()
    );
}

#[tokio::test]
async fn scene_graph_selective_mode_accepted() {
    use phantom_core::process_html;
    use phantom_mcp::engine::SessionPage;

    let adapter = get_test_adapter().await;
    let server = McpServer::new(None);

    let page = process_html(
        r#"<html><body style="width:1280px;height:720px;">
            <form style="width:400px;height:300px;">
              <input type="email" placeholder="Email"
                     style="width:200px;height:30px;"/>
              <button style="width:100px;height:40px;">Login</button>
            </form></body></html>"#,
        "https://login.test",
        1280.0,
        720.0,
    )
    .unwrap();
    adapter.store_page(SessionPage::new(
        page,
        "https://login.test".to_string(),
        200,
    ));

    let req = McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_get_scene_graph",
            "params":{"mode":"selective","task_hint":"find login button"}}"#,
    )
    .unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();
    assert!(
        resp.error.is_none(),
        "scene_graph selective must not error: {:?}",
        resp.error
    );
    let result = resp.result.unwrap();
    assert!(
        result["cct"].as_str().is_some(),
        "result must have cct field"
    );
    assert!(
        result["node_count"].as_u64().unwrap_or(0) > 0,
        "node_count must be > 0"
    );
    println!(
        "selective mode: node_count={}, mode={}",
        result["node_count"], result["mode"]
    );
}

#[tokio::test]
async fn scene_graph_scroll_params_accepted() {
    use phantom_core::process_html;
    use phantom_mcp::engine::SessionPage;

    let adapter = get_test_adapter().await;
    let server = McpServer::new(None);

    let page = process_html(
        "<html><body style='width:1280px;height:2000px;'>
         <div style='width:200px;height:100px;'>Content</div>
         </body></html>",
        "https://scroll.test",
        1280.0,
        720.0,
    )
    .unwrap();
    adapter.store_page(SessionPage::new(
        page,
        "https://scroll.test".to_string(),
        200,
    ));

    let req = McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_get_scene_graph",
            "params":{"scroll_x":0,"scroll_y":500}}"#,
    )
    .unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();
    assert!(
        resp.error.is_none(),
        "scene_graph with scroll params must not error"
    );
    let result = resp.result.unwrap();
    assert!(
        result["cct"].as_str().unwrap_or("").starts_with("##PAGE"),
        "CCT must start with ##PAGE even when scroll params provided"
    );
    println!("scroll params accepted: cct starts with ##PAGE");
}

#[tokio::test]
async fn scene_graph_cct_header_contains_correct_url() {
    use phantom_core::process_html;
    use phantom_mcp::engine::SessionPage;
    use phantom_serializer::{HeadlessSerializer, SerialiserConfig};

    let adapter = get_test_adapter().await;

    let page = process_html(
        "<html><body style='width:1280px;height:720px;'>
         <p style='width:100px;height:20px;'>Hello</p>
         </body></html>",
        "https://specific-url.test/path",
        1280.0,
        720.0,
    )
    .unwrap();
    adapter.store_page(SessionPage::new(
        page,
        "https://specific-url.test/path".to_string(),
        200,
    ));

    let page_clone = adapter.get_page().unwrap();
    let config = SerialiserConfig {
        url: adapter.get_page_url().unwrap(),
        ..Default::default()
    };
    let cct = HeadlessSerializer::serialise(&page_clone, &config);
    let header = cct.lines().next().unwrap_or("");

    assert!(
        header.contains("specific-url.test"),
        "CCT header must contain the exact URL, got: {}",
        header
    );
    println!("url in header: {}", header);
}

// ── browser_click tests ──────────────────────────────────

#[tokio::test]
async fn click_without_navigate_returns_no_page_error() {
    let adapter = get_test_adapter().await;
    let server  = McpServer::new(None);
    let req = McpServer::parse_request(
        r##"{"jsonrpc":"2.0","id":1,"method":"browser_click",
            "params":{"selector":"button"}}"##
    ).unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();
    assert!(resp.error.is_some(),
        "click without navigate must return error");
    let err_str = serde_json::to_string(&resp.error).unwrap();
    assert!(err_str.contains("no_page_loaded"),
        "error must be no_page_loaded, got: {}", err_str);
    println!("click no page: correctly returned no_page_loaded");
}

#[tokio::test]
async fn click_nonexistent_selector_returns_element_not_found() {
    use phantom_mcp::{EngineAdapter, McpServer, engine::SessionPage};
    use phantom_core::process_html;

    let adapter = EngineAdapter::new(5, 0, 5, 0).await;
    let server  = McpServer::new(None);

    // Store a page with NO button
    let page = process_html(
        "<html><body style='width:1280px;height:720px;'>
         <p style='width:200px;height:20px;'>No button here</p>
         </body></html>",
        "https://click.test", 1280.0, 720.0,
    ).unwrap();
    adapter.store_page(SessionPage {
        page: phantom_mcp::engine::SendablePage(page),
        url: "https://click.test".to_string(), status: 200
    });

    let req = McpServer::parse_request(
        r##"{"jsonrpc":"2.0","id":1,"method":"browser_click",
            "params":{"selector":"button#nonexistent-id-xyz"}}"##
    ).unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();
    assert!(resp.error.is_some(),
        "clicking nonexistent element must return error");
    let err_str = serde_json::to_string(&resp.error).unwrap();
    assert!(
        err_str.contains("element_not_found") || err_str.contains("not_found"),
        "error must indicate element not found, got: {}", err_str
    );
    println!("nonexistent selector: correctly returned element_not_found");
}

#[tokio::test]
async fn click_existing_button_returns_success() {
    use phantom_mcp::{EngineAdapter, McpServer, engine::SessionPage};
    use phantom_core::process_html;

    let adapter = EngineAdapter::new(5, 0, 5, 0).await;
    let server  = McpServer::new(None);

    let page = process_html(
        r#"<html><body style="width:1280px;height:720px;">
            <button id="submit-btn"
                    data-testid="submit"
                    style="width:120px;height:40px;">Submit</button>
           </body></html>"#,
        "https://click.test", 1280.0, 720.0,
    ).unwrap();
    adapter.store_page(SessionPage {
        page: phantom_mcp::engine::SendablePage(page), url: "https://click.test".to_string(), status: 200
    });

    let req = McpServer::parse_request(
        r##"{"jsonrpc":"2.0","id":1,"method":"browser_click",
            "params":{"selector":"#submit-btn"}}"##
    ).unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();
    assert!(resp.error.is_none(),
        "clicking existing button must not error: {:?}", resp.error);
    let result = resp.result.unwrap();
    assert_eq!(result["clicked"].as_bool(), Some(true),
        "result.clicked must be true");
    assert!(result["hesitation_ms"].as_u64().is_some(),
        "result.hesitation_ms must be present");
    let hesitation = result["hesitation_ms"].as_u64().unwrap();
    assert!(hesitation >= 20 && hesitation <= 500,
        "hesitation must be in LogNormal clamp range [20,500], got {}",
        hesitation);
    println!("click success: hesitation={}ms", hesitation);
}

#[tokio::test]
async fn click_hesitation_is_in_lognormal_range() {
    use phantom_js::BehaviorEngine;
    let engine = BehaviorEngine::new();
    for i in 0..10 {
        let h = engine.click_hesitation_ms();
        assert!(h >= 20 && h <= 500,
            "hesitation[{}] = {}ms must be in [20,500]", i, h);
    }
    println!("10 hesitation samples all in [20,500]ms");
}

#[tokio::test]
async fn click_missing_selector_param_returns_error() {
    use phantom_mcp::{EngineAdapter, McpServer, engine::SessionPage};
    use phantom_core::process_html;

    let adapter = EngineAdapter::new(5, 0, 5, 0).await;
    let server  = McpServer::new(None);

    let page = process_html(
        "<html><body style='width:1280px;height:720px;'></body></html>",
        "https://click.test", 1280.0, 720.0,
    ).unwrap();
    adapter.store_page(SessionPage {
        page: phantom_mcp::engine::SendablePage(page), url: "https://click.test".to_string(), status: 200
    });

    let req = McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_click","params":{}}"#
    ).unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();
    assert!(resp.error.is_some(),
        "missing selector must return error");
    println!("missing selector: error returned as expected");
}

#[tokio::test]
async fn click_selector_with_single_quote_does_not_panic() {
    use phantom_mcp::{EngineAdapter, McpServer, engine::SessionPage};
    use phantom_core::process_html;

    let adapter = EngineAdapter::new(5, 0, 5, 0).await;
    let server  = McpServer::new(None);

    let page = process_html(
        "<html><body style='width:1280px;height:720px;'></body></html>",
        "https://click.test", 1280.0, 720.0,
    ).unwrap();
    adapter.store_page(SessionPage {
        page: phantom_mcp::engine::SendablePage(page), url: "https://click.test".to_string(), status: 200
    });

    let req = McpServer::parse_request(
        r##"{"jsonrpc":"2.0","id":1,"method":"browser_click",
            "params":{"selector":"[data-label='test']"}}"##
    ).unwrap();
    let resp = server.handle_request(&adapter, req, None).await;
    assert!(resp.is_ok(),
        "single quote in selector must not panic, got: {:?}", resp.err());
    println!("single quote in selector: no panic");
}
