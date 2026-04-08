use phantom_mcp::engine::{get_test_adapter, init_v8};
use phantom_mcp::{EngineAdapter, McpServer};

#[tokio::test]
async fn engine_adapter_constructs_successfully() {
    let adapter = get_test_adapter().await.clone();
    let persona = adapter.next_persona();
    assert!(
        !persona.user_agent.is_empty(),
        "persona user_agent must not be empty"
    );
    println!("EngineAdapter constructed: persona={}", persona.user_agent);
}

#[tokio::test]
async fn handle_navigate_rejects_missing_url_param() {
    let adapter = get_test_adapter().await.clone();
    let server = McpServer::new_with_adapter(None, adapter.clone());
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
    let adapter = get_test_adapter().await.clone();
    let server = McpServer::new_with_adapter(None, adapter.clone());
    let req = McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_navigate","params":{"url":"not-a-url"}}"#,
    )
    .unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();
    assert!(
        resp.error.is_some(),
        "invalid URL must produce an error response"
    );
    assert!(
        resp.result.is_none(),
        "invalid URL must not produce a success result"
    );
    let err = resp.error.unwrap();
    assert!(
        err.message.contains("network_error"),
        "invalid URL should map to network_error, got: {}",
        err.message
    );
}

#[test]
fn existing_ping_still_works_after_refactor() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let adapter = get_test_adapter().await.clone();
        let server = McpServer::new_with_adapter(None, adapter.clone());
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
        let adapter = get_test_adapter().await.clone();
        let server = McpServer::new_with_adapter(Some("secret-key".to_string()), adapter.clone());
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

#[tokio::test]
async fn inject_delta_is_retained_without_subscribers() {
    init_v8();
    let adapter = EngineAdapter::new(2, 0, 2, 0).await;
    assert!(
        adapter.delta_replay_snapshot().is_empty(),
        "new adapter should start with empty replay buffer"
    );

    let receivers = adapter.inject_delta("##SCROLL 0,99".to_string());
    assert_eq!(
        receivers, 0,
        "send count should be zero without subscribers"
    );

    let replay = adapter.delta_replay_snapshot();
    assert_eq!(replay.last().map(String::as_str), Some("##SCROLL 0,99"));
}

// ── browser_get_scene_graph tests ──────────────────────────────────

#[tokio::test]
async fn scene_graph_before_navigate_returns_no_page_error() {
    let adapter = get_test_adapter().await.clone();
    adapter.page_store.lock().clear();
    let server = McpServer::new_with_adapter(None, adapter.clone());
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

    let adapter = get_test_adapter().await.clone();

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

    let adapter = get_test_adapter().await.clone();
    let server = McpServer::new_with_adapter(None, adapter.clone());

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

    let adapter = get_test_adapter().await.clone();
    let server = McpServer::new_with_adapter(None, adapter.clone());

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
async fn scene_graph_rejects_invalid_params() {
    let adapter = get_test_adapter().await.clone();
    let server = McpServer::new_with_adapter(None, adapter.clone());
    let req = McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_get_scene_graph","params":{"scroll_x":"bad"}}"#,
    )
    .unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();
    assert!(
        resp.error.is_some(),
        "invalid params should return structured invalid_params error"
    );
    let err_str = serde_json::to_string(&resp.error).unwrap();
    assert!(
        err_str.contains("invalid_params"),
        "expected invalid_params, got: {}",
        err_str
    );
}

#[tokio::test]
async fn scene_graph_cct_header_contains_correct_url() {
    use phantom_core::process_html;
    use phantom_mcp::engine::SessionPage;
    use phantom_serializer::{HeadlessSerializer, SerialiserConfig};

    let adapter = get_test_adapter().await.clone();

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
    let adapter = get_test_adapter().await.clone();
    let server = McpServer::new_with_adapter(None, adapter.clone());
    let req = McpServer::parse_request(
        r##"{"jsonrpc":"2.0","id":1,"method":"browser_click",
            "params":{"selector":"button"}}"##,
    )
    .unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();
    assert!(
        resp.error.is_some(),
        "click without navigate must return error"
    );
    let err_str = serde_json::to_string(&resp.error).unwrap();
    assert!(
        err_str.contains("no_page_loaded"),
        "error must be no_page_loaded, got: {}",
        err_str
    );
    println!("click no page: correctly returned no_page_loaded");
}

#[tokio::test]
async fn click_nonexistent_selector_returns_element_not_found() {
    use phantom_core::process_html;
    use phantom_mcp::{engine::SessionPage, EngineAdapter, McpServer};

    let adapter = EngineAdapter::new(5, 0, 5, 0).await;
    let server = McpServer::new_with_adapter(None, adapter.clone());

    // Store a page with NO button
    let page = process_html(
        "<html><body style='width:1280px;height:720px;'>
         <p style='width:200px;height:20px;'>No button here</p>
         </body></html>",
        "https://click.test",
        1280.0,
        720.0,
    )
    .unwrap();
    adapter.store_page(SessionPage::new(
        page,
        "https://click.test".to_string(),
        200,
    ));

    let req = McpServer::parse_request(
        r##"{"jsonrpc":"2.0","id":1,"method":"browser_click",
            "params":{"selector":"button#nonexistent-id-xyz"}}"##,
    )
    .unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();
    assert!(
        resp.error.is_some(),
        "clicking nonexistent element must return error"
    );
    let err_str = serde_json::to_string(&resp.error).unwrap();
    assert!(
        err_str.contains("element_not_found") || err_str.contains("not_found"),
        "error must indicate element not found, got: {}",
        err_str
    );
    println!("nonexistent selector: correctly returned element_not_found");
}

#[tokio::test]
async fn click_existing_button_returns_success() {
    use phantom_core::process_html;
    use phantom_mcp::{engine::SessionPage, EngineAdapter, McpServer};

    let adapter = EngineAdapter::new(5, 0, 5, 0).await;
    let server = McpServer::new_with_adapter(None, adapter.clone());

    let page = process_html(
        r#"<html><body style="width:1280px;height:720px;">
            <button id="submit-btn"
                    data-testid="submit"
                    style="width:120px;height:40px;">Submit</button>
           </body></html>"#,
        "https://click.test",
        1280.0,
        720.0,
    )
    .unwrap();
    adapter.store_page(SessionPage::new(
        page,
        "https://click.test".to_string(),
        200,
    ));

    let req = McpServer::parse_request(
        r##"{"jsonrpc":"2.0","id":1,"method":"browser_click",
            "params":{"selector":"#submit-btn"}}"##,
    )
    .unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();
    assert!(
        resp.error.is_none(),
        "clicking existing button must not error: {:?}",
        resp.error
    );
    let result = resp.result.unwrap();
    assert_eq!(
        result["clicked"].as_bool(),
        Some(true),
        "result.clicked must be true"
    );
    assert!(
        result["hesitation_ms"].as_u64().is_some(),
        "result.hesitation_ms must be present"
    );
    let hesitation = result["hesitation_ms"].as_u64().unwrap();
    assert!(
        (20..=500).contains(&hesitation),
        "hesitation must be in LogNormal clamp range [20,500], got {}",
        hesitation
    );
    println!("click success: hesitation={}ms", hesitation);
}

#[tokio::test]
async fn click_hesitation_is_in_lognormal_range() {
    use phantom_js::BehaviorEngine;
    let engine = BehaviorEngine::new();
    for i in 0..10 {
        let h = engine.click_hesitation_ms();
        assert!(
            (20..=500).contains(&h),
            "hesitation[{}] = {}ms must be in [20,500]",
            i,
            h
        );
    }
    println!("10 hesitation samples all in [20,500]ms");
}

#[tokio::test]
async fn click_defaults_to_element_center() {
    use phantom_core::process_html;
    use phantom_mcp::{engine::SessionPage, EngineAdapter, McpServer};

    let adapter = EngineAdapter::new(5, 0, 5, 0).await;
    let server = McpServer::new_with_adapter(None, adapter.clone());

    let page = process_html(
        r#"<html><body style="width:1280px;height:720px;">
            <button id="center-btn" style="width:120px;height:40px;">Center</button>
           </body></html>"#,
        "https://click.test",
        1280.0,
        720.0,
    )
    .unwrap();
    adapter.store_page(SessionPage::new(
        page,
        "https://click.test".to_string(),
        200,
    ));

    let req = McpServer::parse_request(
        r##"{"jsonrpc":"2.0","id":1,"method":"browser_click","params":{"selector":"#center-btn"}}"##,
    )
    .unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();
    assert!(resp.error.is_none(), "{:?}", resp.error);
    let result = resp.result.unwrap();

    assert_eq!(result["x"].as_f64(), Some(60.0));
    assert_eq!(result["y"].as_f64(), Some(20.0));
}

#[tokio::test]
async fn click_missing_selector_param_returns_error() {
    use phantom_core::process_html;
    use phantom_mcp::{engine::SessionPage, EngineAdapter, McpServer};

    let adapter = EngineAdapter::new(5, 0, 5, 0).await;
    let server = McpServer::new_with_adapter(None, adapter.clone());

    let page = process_html(
        "<html><body style='width:1280px;height:720px;'></body></html>",
        "https://click.test",
        1280.0,
        720.0,
    )
    .unwrap();
    adapter.store_page(SessionPage::new(
        page,
        "https://click.test".to_string(),
        200,
    ));

    let req = McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_click","params":{}}"#,
    )
    .unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();
    assert!(resp.error.is_some(), "missing selector must return error");
    println!("missing selector: error returned as expected");
}

#[tokio::test]
async fn click_selector_with_single_quote_does_not_panic() {
    use phantom_core::process_html;
    use phantom_mcp::{engine::SessionPage, EngineAdapter, McpServer};

    let adapter = EngineAdapter::new(5, 0, 5, 0).await;
    let server = McpServer::new_with_adapter(None, adapter.clone());

    let page = process_html(
        "<html><body style='width:1280px;height:720px;'></body></html>",
        "https://click.test",
        1280.0,
        720.0,
    )
    .unwrap();
    adapter.store_page(SessionPage::new(
        page,
        "https://click.test".to_string(),
        200,
    ));

    let req = McpServer::parse_request(
        r##"{"jsonrpc":"2.0","id":1,"method":"browser_click",
            "params":{"selector":"[data-label='test']"}}"##,
    )
    .unwrap();
    let resp = server.handle_request(&adapter, req, None).await;
    assert!(
        resp.is_ok(),
        "single quote in selector must not panic, got: {:?}",
        resp.err()
    );
    println!("single quote in selector: no panic");
}

#[tokio::test]
async fn type_nonexistent_selector_returns_element_not_found() {
    use phantom_core::process_html;
    use phantom_mcp::{engine::SessionPage, EngineAdapter, McpServer};

    let adapter = EngineAdapter::new(5, 0, 5, 0).await;
    let server = McpServer::new_with_adapter(None, adapter.clone());
    let page = process_html(
        "<html><body style='width:1280px;height:720px;'><input id='present'/></body></html>",
        "https://type.test",
        1280.0,
        720.0,
    )
    .unwrap();
    adapter.store_page(SessionPage::new(page, "https://type.test".to_string(), 200));

    let req = McpServer::parse_request(
        r##"{"jsonrpc":"2.0","id":1,"method":"browser_type","params":{"selector":"#missing","text":"abc"}}"##,
    )
    .unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();
    assert!(resp.error.is_some(), "missing input should return error");
    let err_str = serde_json::to_string(&resp.error).unwrap();
    assert!(
        err_str.contains("element_not_found"),
        "expected element_not_found, got: {}",
        err_str
    );
}

#[tokio::test]
async fn press_key_requires_non_empty_key() {
    let adapter = phantom_mcp::EngineAdapter::new(5, 0, 5, 0).await;
    let server = phantom_mcp::McpServer::new_with_adapter(None, adapter.clone());
    let req = phantom_mcp::McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_press_key","params":{"key":""}}"#,
    )
    .unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();
    assert!(resp.error.is_some(), "empty key should fail");
    let err_str = serde_json::to_string(&resp.error).unwrap();
    assert!(
        err_str.contains("invalid_params"),
        "expected invalid_params, got: {}",
        err_str
    );
}

// ── browser_evaluate tests ─────────────────────────────────────────

#[tokio::test]
async fn evaluate_arithmetic_returns_number() {
    use phantom_core::process_html;
    use phantom_mcp::engine::SessionPage;
    use phantom_mcp::{EngineAdapter, McpServer};

    let adapter = EngineAdapter::new(5, 0, 5, 0).await;
    let server = McpServer::new_with_adapter(None, adapter.clone());

    let page = process_html(
        "<html><body style='width:1280px;height:720px;'></body></html>",
        "https://eval.test",
        1280.0,
        720.0,
    )
    .unwrap();
    adapter.store_page(SessionPage::new(page, "https://eval.test".to_string(), 200));

    let req = McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_evaluate","params":{"script":"1 + 1"}}"#,
    )
    .unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();

    assert!(
        resp.error.is_none(),
        "evaluate 1+1 must not error: {:?}",
        resp.error
    );
    let result = resp.result.unwrap();
    let val = &result["result"];
    // QuickJS serialises numbers as strings through the current eval path,
    // so the JSON parse may succeed (number 2) or produce raw string "2".
    assert!(
        val.as_f64() == Some(2.0) || val.as_str() == Some("2"),
        "1+1 must equal 2, got {:?}",
        val
    );
    println!("evaluate 1+1: {:?}", val);
}

#[tokio::test]
async fn evaluate_string_result_has_string_type() {
    use phantom_core::process_html;
    use phantom_mcp::engine::SessionPage;
    use phantom_mcp::{EngineAdapter, McpServer};

    let adapter = EngineAdapter::new(5, 0, 5, 0).await;
    let server = McpServer::new_with_adapter(None, adapter.clone());

    let page = process_html(
        "<html><body style='width:1280px;height:720px;'></body></html>",
        "https://eval.test",
        1280.0,
        720.0,
    )
    .unwrap();
    adapter.store_page(SessionPage::new(page, "https://eval.test".to_string(), 200));

    let req = McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_evaluate","params":{"script":"'hello world'"}}"#
    ).unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();

    assert!(
        resp.error.is_none(),
        "evaluate string must not error: {:?}",
        resp.error
    );
    let result = resp.result.unwrap();
    assert_eq!(
        result["type"].as_str(),
        Some("string"),
        "type must be 'string'"
    );
    println!("evaluate string: type=string verified");
}

#[tokio::test]
async fn evaluate_object_result_returns_json_object() {
    use phantom_core::process_html;
    use phantom_mcp::engine::SessionPage;
    use phantom_mcp::{EngineAdapter, McpServer};

    let adapter = EngineAdapter::new(5, 0, 5, 0).await;
    let server = McpServer::new_with_adapter(None, adapter.clone());

    let page = process_html(
        "<html><body style='width:1280px;height:720px;'></body></html>",
        "https://eval.test",
        1280.0,
        720.0,
    )
    .unwrap();
    adapter.store_page(SessionPage::new(page, "https://eval.test".to_string(), 200));

    let req = McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_evaluate","params":{"script":"({ok:true,count:2})"}}"#,
    )
    .unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();
    assert!(resp.error.is_none(), "evaluate object must not error");
    let result = resp.result.unwrap();
    assert_eq!(result["type"].as_str(), Some("object"));
    assert_eq!(result["result"]["ok"].as_bool(), Some(true));
    assert_eq!(result["result"]["count"].as_u64(), Some(2));
}

#[tokio::test]
async fn evaluate_without_page_returns_no_page_error() {
    let adapter = phantom_mcp::EngineAdapter::new(5, 0, 5, 0).await;
    let server = phantom_mcp::McpServer::new_with_adapter(None, adapter.clone());

    // Ensure the store is empty for this isolated adapter.
    let req = phantom_mcp::McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_evaluate","params":{"script":"1+1"}}"#,
    )
    .unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();

    assert!(
        resp.error.is_some(),
        "evaluate with no page must return error"
    );
    let err_str = serde_json::to_string(&resp.error).unwrap();
    assert!(
        err_str.contains("no_page_loaded"),
        "error must be no_page_loaded, got: {}",
        err_str
    );
    println!("evaluate no page: no_page_loaded returned");
}

// ── tab management tests ───────────────────────────────────────────

#[tokio::test]
async fn tab_new_tab_creates_tab_with_id() {
    let adapter = phantom_mcp::EngineAdapter::new(5, 0, 5, 0).await;
    let server = phantom_mcp::McpServer::new_with_adapter(None, adapter.clone());

    let req = phantom_mcp::McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_new_tab","params":{"url":"https://example.com"}}"#
    ).unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();

    assert!(
        resp.error.is_none(),
        "new_tab must not error: {:?}",
        resp.error
    );
    let result = resp.result.unwrap();
    let tab_id = result["tab_id"].as_str().unwrap_or("");
    assert!(!tab_id.is_empty(), "tab_id must not be empty");
    assert_eq!(
        tab_id.len(),
        36,
        "tab_id must be a UUID (36 chars), got: {}",
        tab_id
    );
    println!("new_tab: tab_id={}", tab_id);
}

#[tokio::test]
async fn tab_list_tabs_returns_created_tabs() {
    let adapter = phantom_mcp::EngineAdapter::new(5, 0, 5, 0).await;
    let server = phantom_mcp::McpServer::new_with_adapter(None, adapter.clone());

    for url in &["https://tab1.com", "https://tab2.com"] {
        let req = phantom_mcp::McpServer::parse_request(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"browser_new_tab","params":{{"url":"{}"}}}}"#,
            url
        ))
        .unwrap();
        server.handle_request(&adapter, req, None).await.unwrap();
    }

    let req = phantom_mcp::McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_list_tabs","params":{}}"#,
    )
    .unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();

    assert!(
        resp.error.is_none(),
        "list_tabs must not error: {:?}",
        resp.error
    );
    let tabs = resp.result.unwrap()["tabs"]
        .as_array()
        .expect("tabs must be an array")
        .clone();
    assert!(
        tabs.len() >= 2,
        "must have at least 2 tabs, got {}",
        tabs.len()
    );
    println!("list_tabs: {} tabs", tabs.len());
}

#[tokio::test]
async fn tab_switch_changes_scene_graph_context() {
    use phantom_core::process_html;
    use phantom_mcp::engine::SessionPage;

    let adapter = phantom_mcp::EngineAdapter::new(5, 0, 5, 0).await;
    let server = phantom_mcp::McpServer::new_with_adapter(None, adapter.clone());

    let req1 = phantom_mcp::McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_new_tab","params":{"url":"https://tab1.test"}}"#,
    )
    .unwrap();
    let resp1 = server.handle_request(&adapter, req1, None).await.unwrap();
    let tab1 = resp1.result.unwrap()["tab_id"]
        .as_str()
        .unwrap()
        .to_string();

    let page1 = process_html(
        "<html><body style='width:1280px;height:720px;'><h1>Tab One</h1></body></html>",
        "https://tab1.test",
        1280.0,
        720.0,
    )
    .unwrap();
    adapter.store_page(SessionPage::new(
        page1,
        "https://tab1.test".to_string(),
        200,
    ));

    let req2 = phantom_mcp::McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":2,"method":"browser_new_tab","params":{"url":"https://tab2.test"}}"#,
    )
    .unwrap();
    server.handle_request(&adapter, req2, None).await.unwrap();

    let page2 = process_html(
        "<html><body style='width:1280px;height:720px;'><h1>Tab Two</h1></body></html>",
        "https://tab2.test",
        1280.0,
        720.0,
    )
    .unwrap();
    adapter.store_page(SessionPage::new(
        page2,
        "https://tab2.test".to_string(),
        200,
    ));

    let switch_req = phantom_mcp::McpServer::parse_request(&format!(
        r#"{{"jsonrpc":"2.0","id":3,"method":"browser_switch_tab","params":{{"tab_id":"{}"}}}}"#,
        tab1
    ))
    .unwrap();
    server
        .handle_request(&adapter, switch_req, None)
        .await
        .unwrap();

    let scene_req = phantom_mcp::McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":4,"method":"browser_get_scene_graph","params":{}}"#,
    )
    .unwrap();
    let scene_resp = server
        .handle_request(&adapter, scene_req, None)
        .await
        .unwrap();
    assert_eq!(
        scene_resp.result.unwrap()["url"].as_str(),
        Some("https://tab1.test"),
        "scene graph URL should track active tab context"
    );
}

#[tokio::test]
async fn tab_switch_to_nonexistent_tab_returns_error() {
    let adapter = phantom_mcp::EngineAdapter::new(5, 0, 5, 0).await;
    let server = phantom_mcp::McpServer::new_with_adapter(None, adapter.clone());

    let req = phantom_mcp::McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_switch_tab",
            "params":{"tab_id":"00000000-0000-0000-0000-000000000000"}}"#,
    )
    .unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();

    assert!(
        resp.error.is_some(),
        "switching to a nonexistent tab must return error"
    );
    let err_str = serde_json::to_string(&resp.error).unwrap();
    assert!(
        err_str.contains("tab_not_found"),
        "error must be tab_not_found, got: {}",
        err_str
    );
    println!("switch nonexistent: tab_not_found returned");
}

#[tokio::test]
async fn tab_close_removes_tab_from_list() {
    let adapter = phantom_mcp::EngineAdapter::new(5, 0, 5, 0).await;
    let server = phantom_mcp::McpServer::new_with_adapter(None, adapter.clone());

    // Create the tab.
    let create_req = phantom_mcp::McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_new_tab","params":{"url":"https://closeme.com"}}"#
    ).unwrap();
    let create_resp = server
        .handle_request(&adapter, create_req, None)
        .await
        .unwrap();
    let tab_id = create_resp.result.unwrap()["tab_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Close it.
    let close_req = phantom_mcp::McpServer::parse_request(&format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"browser_close_tab","params":{{"tab_id":"{}"}}}}"#,
        tab_id
    ))
    .unwrap();
    let close_resp = server
        .handle_request(&adapter, close_req, None)
        .await
        .unwrap();
    assert!(
        close_resp.error.is_none(),
        "close_tab must not error: {:?}",
        close_resp.error
    );
    assert_eq!(
        close_resp.result.unwrap()["closed"].as_bool(),
        Some(true),
        "closed must be true"
    );

    // Verify it no longer appears in the list.
    let list_req = phantom_mcp::McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_list_tabs","params":{}}"#,
    )
    .unwrap();
    let list_resp = server
        .handle_request(&adapter, list_req, None)
        .await
        .unwrap();
    let tabs = list_resp.result.unwrap()["tabs"]
        .as_array()
        .unwrap()
        .clone();
    assert!(
        tabs.iter().all(|t| t["id"].as_str() != Some(&tab_id)),
        "closed tab must not appear in list"
    );
    println!("close_tab: tab removed from list");
}

#[tokio::test]
async fn tab_close_keeps_active_page_and_store_in_sync() {
    use phantom_core::process_html;
    use phantom_mcp::engine::SessionPage;

    let adapter = phantom_mcp::EngineAdapter::new(5, 0, 5, 0).await;

    let tab1 = adapter
        .open_tab(Some("https://sync-tab-1.test".to_string()))
        .await;
    let page1 = process_html(
        "<html><body style='width:1280px;height:720px;'><h1>Tab One</h1></body></html>",
        "https://sync-tab-1.test",
        1280.0,
        720.0,
    )
    .unwrap();
    adapter.store_page(SessionPage::new(
        page1,
        "https://sync-tab-1.test".to_string(),
        200,
    ));

    let tab2 = adapter
        .open_tab(Some("https://sync-tab-2.test".to_string()))
        .await;
    let page2 = process_html(
        "<html><body style='width:1280px;height:720px;'><h1>Tab Two</h1></body></html>",
        "https://sync-tab-2.test",
        1280.0,
        720.0,
    )
    .unwrap();
    adapter.store_page(SessionPage::new(
        page2,
        "https://sync-tab-2.test".to_string(),
        200,
    ));

    let remaining = adapter.close_tab(tab2).await;
    assert_eq!(remaining, Some(1), "one tab should remain after close");

    assert_eq!(
        *adapter.active_page_key.lock(),
        Some(tab1),
        "active page key should point at the remaining tab"
    );
    let pages = adapter.page_store.lock();
    assert!(
        pages.contains_key(&tab1),
        "remaining tab page should stay in page store"
    );
    assert!(
        !pages.contains_key(&tab2),
        "closed tab page should be removed from page store"
    );
}

// ── cookie tests ─────────────────────────────────────────

#[tokio::test]
async fn cookies_initially_empty() {
    let adapter = phantom_mcp::EngineAdapter::new(5, 0, 5, 0).await;
    let server = phantom_mcp::McpServer::new_with_adapter(None, adapter.clone());
    let req = phantom_mcp::McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_get_cookies","params":{}}"#,
    )
    .unwrap();
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
    let server = phantom_mcp::McpServer::new_with_adapter(None, adapter.clone());
    let req = phantom_mcp::McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_clear_cookies","params":{}}"#,
    )
    .unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();
    assert!(resp.error.is_none(), "{:?}", resp.error);
    assert_eq!(resp.result.unwrap()["cleared"].as_bool(), Some(true));
    println!("clear_cookies: cleared=true");
}

#[tokio::test]
async fn session_snapshot_creates_file() {
    use std::path::Path;
    let adapter = phantom_mcp::EngineAdapter::new(5, 0, 5, 0).await;
    let server = phantom_mcp::McpServer::new_with_adapter(None, adapter.clone());
    let req = phantom_mcp::McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_session_snapshot","params":{}}"#,
    )
    .unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();
    assert!(
        resp.error.is_none(),
        "snapshot must not error: {:?}",
        resp.error
    );
    let result = resp.result.unwrap();
    let snapshot_path = result["snapshot_path"]
        .as_str()
        .expect("snapshot_path must be present");
    let size_bytes = result["size_bytes"]
        .as_u64()
        .expect("size_bytes must be present");
    assert!(size_bytes > 0, "compressed snapshot must not be empty");
    assert!(
        Path::new(snapshot_path).exists(),
        "snapshot file must exist at {}",
        snapshot_path
    );
    println!("snapshot: path={}, size={}b", snapshot_path, size_bytes);

    // Clean up
    let _ = std::fs::remove_file(snapshot_path);
}

#[tokio::test]
async fn session_snapshot_is_zstd_compressed() {
    let adapter = phantom_mcp::EngineAdapter::new(5, 0, 5, 0).await;
    let server = phantom_mcp::McpServer::new_with_adapter(None, adapter.clone());
    let req = phantom_mcp::McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_session_snapshot","params":{}}"#,
    )
    .unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();
    let result = resp.result.unwrap();
    let snapshot_path = result["snapshot_path"].as_str().unwrap();

    let bytes = std::fs::read(snapshot_path).expect("snapshot file must be readable");
    // zstd magic bytes: 0xFD 0x2F 0xB5 0x28 (little-endian frame header)
    assert!(bytes.len() >= 4, "snapshot must have at least 4 bytes");
    assert_eq!(
        &bytes[0..4],
        &[0x28, 0xB5, 0x2F, 0xFD],
        "snapshot must start with zstd magic bytes"
    );
    println!("snapshot zstd magic bytes: VERIFIED");

    // Clean up
    let _ = std::fs::remove_file(snapshot_path);
}

#[tokio::test]
async fn storage_session_id_validates_uuid_format() {
    use phantom_storage::is_valid_session_id;
    // Valid UUIDs must pass
    assert!(is_valid_session_id("550e8400-e29b-41d4-a716-446655440000"));
    assert!(
        is_valid_session_id("00000000-0000-0000-0000-000000000000"),
        "nil UUID must be valid"
    );
    // Invalid must fail
    assert!(
        !is_valid_session_id("../../../etc/passwd"),
        "path traversal must fail"
    );
    assert!(!is_valid_session_id("not-a-uuid"), "non-UUID must fail");
    assert!(!is_valid_session_id(""), "empty string must fail");
    assert!(
        !is_valid_session_id("550e8400-e29b-41d4-a716"),
        "short UUID must fail"
    );
    println!("session_id validation: all 6 cases correct");
}

#[tokio::test]
async fn multiple_snapshot_calls_produce_multiple_files() {
    use std::path::Path;
    let adapter = phantom_mcp::EngineAdapter::new(5, 0, 5, 0).await;
    let server = phantom_mcp::McpServer::new_with_adapter(None, adapter.clone());

    let mut paths = Vec::new();
    for i in 0..2 {
        let req = phantom_mcp::McpServer::parse_request(
            r#"{"jsonrpc":"2.0","id":1,"method":"browser_session_snapshot","params":{}}"#,
        )
        .unwrap();
        let resp = server.handle_request(&adapter, req, None).await.unwrap();
        let path = resp.result.unwrap()["snapshot_path"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(Path::new(&path).exists(), "snapshot {} must exist", i);
        paths.push(path);
    }

    assert_ne!(
        paths[0], paths[1],
        "two snapshot calls should produce distinct paths"
    );
    println!("multiple snapshots: {} files created", paths.len());

    // Clean up
    for p in &paths {
        let _ = std::fs::remove_file(p);
    }
}

// ── SSE tests ─────────────────────────────────────────

#[tokio::test]
async fn inject_delta_with_no_subscribers_returns_zero() {
    let adapter = phantom_mcp::EngineAdapter::new(5, 0, 5, 0).await;
    // No subscribers — send returns RecvError or 0 receivers
    let receivers = adapter.inject_delta("## SCROLL 0,100".to_string());
    assert_eq!(
        receivers, 0,
        "inject_delta with no subscribers must return 0"
    );
    println!("inject_delta no subscribers: returns 0");
}

#[tokio::test]
async fn inject_delta_with_one_subscriber_delivers_message() {
    use std::time::Duration;
    use tokio::time::timeout;

    let adapter = phantom_mcp::EngineAdapter::new(5, 0, 5, 0).await;
    let mut rx = adapter.delta_tx.subscribe();

    let delta = "## SCROLL 0,200".to_string();
    let sent = adapter.inject_delta(delta.clone());
    assert!(sent >= 1, "must have at least 1 receiver");

    let received = timeout(Duration::from_millis(100), rx.recv())
        .await
        .expect("must receive within 100ms")
        .expect("channel must not be closed");

    assert_eq!(received, delta, "received delta must match sent delta");
    println!("inject_delta with subscriber: message delivered");
}

#[tokio::test]
async fn sse_endpoint_exists_and_returns_text_event_stream() {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    let adapter = phantom_mcp::EngineAdapter::new(5, 0, 5, 0).await;
    let server = phantom_mcp::McpServer::new_with_adapter(None, adapter.clone());
    let app = server.router();

    // GET /sse must return 200 with content-type text/event-stream
    let req = Request::builder()
        .method("GET")
        .uri("/sse")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "SSE endpoint must return 200"
    );
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        content_type.contains("text/event-stream"),
        "content-type must be text/event-stream, got: {}",
        content_type
    );
    println!("SSE endpoint: 200 + text/event-stream verified");
}

#[tokio::test]
async fn rpc_endpoint_still_works_after_sse_route_added() {
    use axum::body::to_bytes;
    use axum::body::Body;
    use axum::http::{header, Request, StatusCode};
    use tower::ServiceExt;

    let adapter = phantom_mcp::EngineAdapter::new(5, 0, 5, 0).await;
    let server = phantom_mcp::McpServer::new_with_adapter(None, adapter.clone());
    let app = server.router();

    let body = r#"{"jsonrpc":"2.0","id":1,"method":"ping","params":{}}"#;
    let req = Request::builder()
        .method("POST")
        .uri("/rpc")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "/rpc must still return 200 after SSE route added"
    );
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        json["result"]["ok"].as_bool(),
        Some(true),
        "ping must return ok:true"
    );
    println!("rpc endpoint regression: PASSING");
}

#[tokio::test]
async fn broadcast_channel_capacity_is_128() {
    // Verify that 128 deltas can be queued without dropping.
    let adapter = phantom_mcp::EngineAdapter::new(5, 0, 5, 0).await;
    let mut rx = adapter.delta_tx.subscribe();

    // Subscribe then send 128 messages before reading
    for i in 0..128 {
        adapter.inject_delta(format!("delta_{}", i));
    }

    // All 128 must be receivable without lagging
    let mut count = 0;
    while let Ok(msg) = rx.try_recv() {
        assert!(
            msg.starts_with("delta_"),
            "received message must start with delta_"
        );
        count += 1;
    }
    assert_eq!(
        count, 128,
        "all 128 messages must be receivable from broadcast channel"
    );
    println!("broadcast capacity 128: all {} messages received", count);
}
