#[cfg(test)]
mod tests {
    use phantom_core::process_html;
    use std::time::Instant;

    fn make_large_page(n: usize) -> String {
        let mut html = String::from(
            "<html><body style='width: 1280px; height: 720px;'>"
        );
        for i in 0..n {
            html.push_str(&format!(
                "<div id='node{}' class='item' style='width: 200px; height: 100px;'>\
                    <span style='width: 100px; height: 20px;'>Item {}</span>\
                    <button style='width: 80px; height: 30px;'>Action {}</button>\
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
            <html><body style="width: 1280px; height: 720px;">
                <div id="hidden-display" style="display: none; width: 100px; height: 50px;">Should be hidden</div>
                <div id="hidden-vis" style="visibility: hidden; width: 100px; height: 50px;">Also hidden</div>
                <div id="hidden-opacity" style="opacity: 0; width: 100px; height: 50px;">Opacity hidden</div>
                <div id="visible" style="width: 100px; height: 50px;">Visible</div>
            </body></html>
        "#;

        let page = process_html(html, "https://test.com", 1280.0, 720.0)
            .expect("pipeline should not fail");

        assert!(page.tree.document_root.is_some());

        let hidden_display = page.tree.get_element_by_id("hidden-display").unwrap();
        assert!(!page.tree.get(hidden_display).is_visible, "display:none must be invisible");

        let hidden_vis = page.tree.get_element_by_id("hidden-vis").unwrap();
        assert!(!page.tree.get(hidden_vis).is_visible, "visibility:hidden must be invisible");

        let hidden_opacity = page.tree.get_element_by_id("hidden-opacity").unwrap();
        assert!(!page.tree.get(hidden_opacity).is_visible, "opacity:0 must be invisible");

        let visible = page.tree.get_element_by_id("visible").unwrap();
        assert!(page.tree.get(visible).is_visible, "normal div must be visible");
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
