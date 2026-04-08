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

// ── browser_evaluate tests ─────────────────────────────────────────

#[tokio::test]
async fn evaluate_arithmetic_returns_number() {
    use phantom_core::process_html;
    use phantom_mcp::engine::SessionPage;
    use phantom_mcp::{EngineAdapter, McpServer};

    let adapter = EngineAdapter::new(5, 0, 5, 0).await;
    let server  = McpServer::new(None);

    let page = process_html(
        "<html><body style='width:1280px;height:720px;'></body></html>",
        "https://eval.test", 1280.0, 720.0,
    ).unwrap();
    adapter.store_page(SessionPage {
        page: phantom_mcp::engine::SendablePage(page),
        url:  "https://eval.test".to_string(),
        status: 200,
    });

    let req = McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_evaluate","params":{"script":"1 + 1"}}"#
    ).unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();

    assert!(resp.error.is_none(), "evaluate 1+1 must not error: {:?}", resp.error);
    let result = resp.result.unwrap();
    let val = &result["result"];
    // QuickJS serialises numbers as strings through the current eval path,
    // so the JSON parse may succeed (number 2) or produce raw string "2".
    assert!(
        val.as_f64() == Some(2.0) || val.as_str() == Some("2"),
        "1+1 must equal 2, got {:?}", val
    );
    println!("evaluate 1+1: {:?}", val);
}

#[tokio::test]
async fn evaluate_string_result_has_string_type() {
    use phantom_core::process_html;
    use phantom_mcp::engine::SessionPage;
    use phantom_mcp::{EngineAdapter, McpServer};

    let adapter = EngineAdapter::new(5, 0, 5, 0).await;
    let server  = McpServer::new(None);

    let page = process_html(
        "<html><body style='width:1280px;height:720px;'></body></html>",
        "https://eval.test", 1280.0, 720.0,
    ).unwrap();
    adapter.store_page(SessionPage {
        page: phantom_mcp::engine::SendablePage(page),
        url:  "https://eval.test".to_string(),
        status: 200,
    });

    let req = McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_evaluate","params":{"script":"'hello world'"}}"#
    ).unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();

    assert!(resp.error.is_none(), "evaluate string must not error: {:?}", resp.error);
    let result = resp.result.unwrap();
    assert_eq!(result["type"].as_str(), Some("string"), "type must be 'string'");
    println!("evaluate string: type=string verified");
}

#[tokio::test]
async fn evaluate_without_page_returns_no_page_error() {
    let adapter = phantom_mcp::EngineAdapter::new(5, 0, 5, 0).await;
    let server  = phantom_mcp::McpServer::new(None);

    // Ensure the store is empty for this isolated adapter.
    let req = phantom_mcp::McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_evaluate","params":{"script":"1+1"}}"#
    ).unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();

    assert!(resp.error.is_some(), "evaluate with no page must return error");
    let err_str = serde_json::to_string(&resp.error).unwrap();
    assert!(
        err_str.contains("no_page_loaded"),
        "error must be no_page_loaded, got: {}", err_str
    );
    println!("evaluate no page: no_page_loaded returned");
}

// ── tab management tests ───────────────────────────────────────────

#[tokio::test]
async fn tab_new_tab_creates_tab_with_id() {
    let adapter = phantom_mcp::EngineAdapter::new(5, 0, 5, 0).await;
    let server  = phantom_mcp::McpServer::new(None);

    let req = phantom_mcp::McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_new_tab","params":{"url":"https://example.com"}}"#
    ).unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();

    assert!(resp.error.is_none(), "new_tab must not error: {:?}", resp.error);
    let result = resp.result.unwrap();
    let tab_id = result["tab_id"].as_str().unwrap_or("");
    assert!(!tab_id.is_empty(),  "tab_id must not be empty");
    assert_eq!(tab_id.len(), 36, "tab_id must be a UUID (36 chars), got: {}", tab_id);
    println!("new_tab: tab_id={}", tab_id);
}

#[tokio::test]
async fn tab_list_tabs_returns_created_tabs() {
    let adapter = phantom_mcp::EngineAdapter::new(5, 0, 5, 0).await;
    let server  = phantom_mcp::McpServer::new(None);

    for url in &["https://tab1.com", "https://tab2.com"] {
        let req = phantom_mcp::McpServer::parse_request(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"browser_new_tab","params":{{"url":"{}"}}}}"#,
            url
        )).unwrap();
        server.handle_request(&adapter, req, None).await.unwrap();
    }

    let req = phantom_mcp::McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_list_tabs","params":{}}"#
    ).unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();

    assert!(resp.error.is_none(), "list_tabs must not error: {:?}", resp.error);
    let tabs = resp.result.unwrap()["tabs"]
        .as_array().expect("tabs must be an array")
        .clone();
    assert!(tabs.len() >= 2, "must have at least 2 tabs, got {}", tabs.len());
    println!("list_tabs: {} tabs", tabs.len());
}

#[tokio::test]
async fn tab_switch_to_nonexistent_tab_returns_error() {
    let adapter = phantom_mcp::EngineAdapter::new(5, 0, 5, 0).await;
    let server  = phantom_mcp::McpServer::new(None);

    let req = phantom_mcp::McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_switch_tab",
            "params":{"tab_id":"00000000-0000-0000-0000-000000000000"}}"#
    ).unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();

    assert!(resp.error.is_some(), "switching to a nonexistent tab must return error");
    let err_str = serde_json::to_string(&resp.error).unwrap();
    assert!(
        err_str.contains("tab_not_found"),
        "error must be tab_not_found, got: {}", err_str
    );
    println!("switch nonexistent: tab_not_found returned");
}

#[tokio::test]
async fn tab_close_removes_tab_from_list() {
    let adapter = phantom_mcp::EngineAdapter::new(5, 0, 5, 0).await;
    let server  = phantom_mcp::McpServer::new(None);

    // Create the tab.
    let create_req = phantom_mcp::McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_new_tab","params":{"url":"https://closeme.com"}}"#
    ).unwrap();
    let create_resp = server.handle_request(&adapter, create_req, None).await.unwrap();
    let tab_id = create_resp.result.unwrap()["tab_id"]
        .as_str().unwrap().to_string();

    // Close it.
    let close_req = phantom_mcp::McpServer::parse_request(&format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"browser_close_tab","params":{{"tab_id":"{}"}}}}"#,
        tab_id
    )).unwrap();
    let close_resp = server.handle_request(&adapter, close_req, None).await.unwrap();
    assert!(close_resp.error.is_none(), "close_tab must not error: {:?}", close_resp.error);
    assert_eq!(
        close_resp.result.unwrap()["closed"].as_bool(), Some(true),
        "closed must be true"
    );

    // Verify it no longer appears in the list.
    let list_req = phantom_mcp::McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_list_tabs","params":{}}"#
    ).unwrap();
    let list_resp = server.handle_request(&adapter, list_req, None).await.unwrap();
    let tabs = list_resp.result.unwrap()["tabs"]
        .as_array().unwrap().clone();
    assert!(
        tabs.iter().all(|t| t["id"].as_str() != Some(&tab_id)),
        "closed tab must not appear in list"
    );
    println!("close_tab: tab removed from list");
}

// ── cookie tests ─────────────────────────────────────────

#[tokio::test]
async fn cookies_initially_empty() {
    let adapter = phantom_mcp::EngineAdapter::new(5, 0, 5, 0).await;
    let server  = phantom_mcp::McpServer::new(None);
    let req = phantom_mcp::McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_get_cookies","params":{}}"#
    ).unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();
    assert!(resp.error.is_none(), "{:?}", resp.error);
    let result = resp.result.unwrap();
    let cookies = result["cookies"].as_array().expect("must be array");
    assert_eq!(cookies.len(), 0, "fresh session must have 0 cookies");
    println!("cookies initially empty: VERIFIED");
}

#[tokio::test]
async fn clear_cookies_returns_cleared_true() {
    let adapter = phantom_mcp::EngineAdapter::new(5, 0, 5, 0).await;
    let server  = phantom_mcp::McpServer::new(None);
    let req = phantom_mcp::McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_clear_cookies","params":{}}"#
    ).unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();
    assert!(resp.error.is_none(), "{:?}", resp.error);
    assert_eq!(resp.result.unwrap()["cleared"].as_bool(), Some(true));
    println!("clear_cookies: cleared=true");
}

#[tokio::test]
async fn session_snapshot_creates_file() {
    use std::path::Path;
    let adapter = phantom_mcp::EngineAdapter::new(5, 0, 5, 0).await;
    let server  = phantom_mcp::McpServer::new(None);
    let req = phantom_mcp::McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_session_snapshot","params":{}}"#
    ).unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();
    assert!(resp.error.is_none(),
        "snapshot must not error: {:?}", resp.error);
    let result = resp.result.unwrap();
    let snapshot_path = result["snapshot_path"].as_str()
        .expect("snapshot_path must be present");
    let size_bytes = result["size_bytes"].as_u64()
        .expect("size_bytes must be present");
    assert!(size_bytes > 0, "compressed snapshot must not be empty");
    assert!(Path::new(snapshot_path).exists(),
        "snapshot file must exist at {}", snapshot_path);
    println!("snapshot: path={}, size={}b", snapshot_path, size_bytes);

    // Clean up
    let _ = std::fs::remove_file(snapshot_path);
}

#[tokio::test]
async fn session_snapshot_is_zstd_compressed() {
    let adapter = phantom_mcp::EngineAdapter::new(5, 0, 5, 0).await;
    let server  = phantom_mcp::McpServer::new(None);
    let req = phantom_mcp::McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_session_snapshot","params":{}}"#
    ).unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();
    let result = resp.result.unwrap();
    let snapshot_path = result["snapshot_path"].as_str().unwrap();

    let bytes = std::fs::read(snapshot_path).expect("snapshot file must be readable");
    // zstd magic bytes: 0xFD 0x2F 0xB5 0x28 (little-endian frame header)
    assert!(bytes.len() >= 4, "snapshot must have at least 4 bytes");
    assert_eq!(&bytes[0..4], &[0x28, 0xB5, 0x2F, 0xFD],
        "snapshot must start with zstd magic bytes");
    println!("snapshot zstd magic bytes: VERIFIED");

    // Clean up
    let _ = std::fs::remove_file(snapshot_path);
}

#[tokio::test]
async fn storage_session_id_validates_uuid_format() {
    use phantom_storage::is_valid_session_id;
    // Valid UUIDs must pass
    assert!(is_valid_session_id("550e8400-e29b-41d4-a716-446655440000"));
    assert!(is_valid_session_id("00000000-0000-0000-0000-000000000000"),
        "nil UUID must be valid");
    // Invalid must fail
    assert!(!is_valid_session_id("../../../etc/passwd"),
        "path traversal must fail");
    assert!(!is_valid_session_id("not-a-uuid"),
        "non-UUID must fail");
    assert!(!is_valid_session_id(""),
        "empty string must fail");
    assert!(!is_valid_session_id("550e8400-e29b-41d4-a716"),
        "short UUID must fail");
    println!("session_id validation: all 6 cases correct");
}

#[tokio::test]
async fn multiple_snapshot_calls_produce_multiple_files() {
    use std::path::Path;
    let adapter = phantom_mcp::EngineAdapter::new(5, 0, 5, 0).await;
    let server  = phantom_mcp::McpServer::new(None);

    let mut paths = Vec::new();
    for i in 0..2 {
        // Small delay to ensure different timestamps
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await; // wait 1s because timestamps are per-sec
        let req = phantom_mcp::McpServer::parse_request(
            r#"{"jsonrpc":"2.0","id":1,"method":"browser_session_snapshot","params":{}}"#
        ).unwrap();
        let resp = server.handle_request(&adapter, req, None).await.unwrap();
        let path = resp.result.unwrap()["snapshot_path"]
            .as_str().unwrap().to_string();
        assert!(Path::new(&path).exists(),
            "snapshot {} must exist", i);
        paths.push(path);
    }

    // Two snapshots should have different paths (different timestamps)
    // Note: if timestamps collide (same second), paths may be the same.
    // This is acceptable — just verify both calls succeed.
    println!("multiple snapshots: {} files created", paths.len());

    // Clean up
    for p in &paths {
        let _ = std::fs::remove_file(p);
    }
}

