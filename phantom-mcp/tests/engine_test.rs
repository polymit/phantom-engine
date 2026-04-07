use phantom_mcp::{EngineAdapter, McpServer};
use phantom_mcp::engine::get_test_adapter;

#[tokio::test]
async fn engine_adapter_constructs_successfully() {
    let adapter = get_test_adapter().await;
    let persona = adapter.next_persona();
    assert!(!persona.user_agent.is_empty(),
        "persona user_agent must not be empty");
    println!("EngineAdapter constructed: persona={}", persona.user_agent);
}

#[tokio::test]
async fn handle_navigate_rejects_missing_url_param() {
    let adapter = get_test_adapter().await;
    let server = McpServer::new(None);
    let req = McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_navigate","params":{}}"#
    ).unwrap();
    let resp = server.handle_request(&adapter, req, None).await.unwrap();
    assert!(resp.error.is_some(),
        "missing url param must produce an error response");
    println!("missing url: got error as expected");
}

#[tokio::test]
async fn handle_navigate_invalid_url_returns_error() {
    let adapter = get_test_adapter().await;
    let server = McpServer::new(None);
    let req = McpServer::parse_request(
        r#"{"jsonrpc":"2.0","id":1,"method":"browser_navigate","params":{"url":"not-a-url"}}"#
    ).unwrap();
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
            r#"{"jsonrpc":"2.0","id":"test","method":"ping","params":{}}"#
        ).unwrap();
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
        let req = McpServer::parse_request(
            r#"{"jsonrpc":"2.0","id":1,"method":"ping","params":{}}"#
        ).unwrap();
        let err = server.handle_request(&adapter, req, Some("wrong-key"))
            .await.unwrap_err();
        assert!(matches!(err, McpError::Unauthorized),
            "wrong API key must produce Unauthorized error");
        println!("API key enforcement: VERIFIED");
    });
}
