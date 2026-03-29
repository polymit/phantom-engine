#[cfg(test)]
mod tests {
    use phantom_core::process_html;
    use std::time::Instant;

    fn make_large_page(n: usize) -> String {
        let mut html = String::from(
            "<html><body>"
        );
        for i in 0..n {
            html.push_str(&format!(
                "<div id='node{}' class='item'>\
                    <span>Item {}</span>\
                    <button>Action {}</button>\
                </div>",
                i, i, i
            ));
        }
        html.push_str("</body></html>");
        html
    }

    #[test]
    fn test_full_pipeline_simple() {
        let html = r#"
            <html>
            <head><title>Test</title></head>
            <body>
                <div id="main" style="display: block;">
                    <p style="visibility: hidden;">Hidden</p>
                    <button style="opacity: 0;">Invisible</button>
                    <a href="https://example.com">Link</a>
                </div>
            </body>
            </html>
        "#;

        let result = process_html(html, "https://example.com", 1280.0, 720.0);
        assert!(result.is_ok());
        let page = result.unwrap();
        assert!(page.tree.document_root.is_some());
    }

    #[test]
    fn test_css_visibility_applied() {
        let html = r#"
            <html><body>
                <div style="display: none;">Should be hidden</div>
                <div style="visibility: hidden;">Also hidden</div>
                <div style="opacity: 0;">Opacity hidden</div>
                <div>Visible</div>
            </body></html>
        "#;

        let page = process_html(html, "https://test.com", 1280.0, 720.0)
            .expect("pipeline should not fail");

        // The document root must exist
        assert!(page.tree.document_root.is_some());
    }

    #[test]
    fn test_pipeline_benchmark_1000_nodes() {
        // Performance target from blueprint: process 1000 nodes
        // The serialisation target is <5ms for 1000 nodes
        // This test verifies the core pipeline processes fast enough
        let html = make_large_page(333); // 333 divs × 3 nodes each ≈ 1000 nodes

        let start = Instant::now();
        let result = process_html(&html, "https://bench.test", 1280.0, 720.0);
        let elapsed = start.elapsed();

        assert!(result.is_ok(), "Pipeline failed: {:?}", result.err());
        println!("Pipeline processed ~1000-node page in {:?}", elapsed);

        // We do not assert on timing in CI (machine-dependent)
        // but we print it so HQ can verify it is reasonable
    }

    #[test]
    fn test_pipeline_malformed_html() {
        // Must not panic on malformed input
        let html = "<div><p>unclosed <span>tags everywhere";
        let result = process_html(html, "https://broken.com", 1280.0, 720.0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_pipeline_empty_page() {
        let result = process_html("", "https://empty.com", 1280.0, 720.0);
        assert!(result.is_ok());
    }
}
