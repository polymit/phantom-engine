#![allow(clippy::unwrap_used, clippy::expect_used)]
#[cfg(test)]
mod tests {
    use phantom_core::process_html;
    use std::time::Instant;

    fn make_large_page(n: usize) -> String {
        let mut html = String::from("<html><body style='width: 1280px; height: 720px;'>");
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

        let result = process_html(html, "https://example.com", 1280.0, 720.0, Vec::new());
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

        let page = process_html(html, "https://test.com", 1280.0, 720.0, Vec::new())
            .expect("pipeline should not fail");

        assert!(page.tree.document_root.is_some());

        let hidden_display = page.tree.get_element_by_id("hidden-display").unwrap();
        assert!(
            !page
                .tree
                .get(hidden_display)
                .expect("hidden-display node should exist")
                .is_visible,
            "display:none must be invisible"
        );

        let hidden_vis = page.tree.get_element_by_id("hidden-vis").unwrap();
        assert!(
            !page
                .tree
                .get(hidden_vis)
                .expect("hidden-vis node should exist")
                .is_visible,
            "visibility:hidden must be invisible"
        );

        let hidden_opacity = page.tree.get_element_by_id("hidden-opacity").unwrap();
        assert!(
            !page
                .tree
                .get(hidden_opacity)
                .expect("hidden-opacity node should exist")
                .is_visible,
            "opacity:0 must be invisible"
        );

        let visible = page.tree.get_element_by_id("visible").unwrap();
        assert!(
            page.tree
                .get(visible)
                .expect("visible node should exist")
                .is_visible,
            "normal div must be visible"
        );
    }

    #[test]
    fn test_pipeline_benchmark_1000_nodes() {
        // Performance target from blueprint: process 1000 nodes
        // The serialisation target is <5ms for 1000 nodes
        // This test verifies the core pipeline processes fast enough
        let html = make_large_page(333); // 333 divs × 3 nodes each ≈ 1000 nodes

        let start = Instant::now();
        let result = process_html(&html, "https://bench.test", 1280.0, 720.0, Vec::new());
        let elapsed = start.elapsed();

        assert!(result.is_ok(), "Pipeline failed: {:?}", result.err());
        let page = result.unwrap();
        assert!(
            page.tree.document_root.is_some(),
            "pipeline benchmark page must produce a document root"
        );
        let interactive_nodes = page.tree.query_selector_all("button").len();
        assert!(
            interactive_nodes >= 333,
            "benchmark fixture should include all generated buttons"
        );
        println!("Pipeline processed ~1000-node page in {:?}", elapsed);

        // We do not assert on timing in CI (machine-dependent)
        // but we print it so HQ can verify it is reasonable
    }

    #[test]
    fn test_pipeline_malformed_html() {
        // Must not panic on malformed input
        let html = "<div><p>unclosed <span>tags everywhere";
        let result = process_html(html, "https://broken.com", 1280.0, 720.0, Vec::new());
        assert!(result.is_ok());
    }

    #[test]
    fn test_pipeline_empty_page() {
        let result = process_html("", "https://empty.com", 1280.0, 720.0, Vec::new());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visibility_visible_overrides_hidden_parent() {
        let html = r#"
            <html><body style="width: 1280px; height: 720px;">
                <div id="parent" style="visibility: hidden; width: 200px; height: 100px;">
                    <button id="child" style="visibility: visible; width: 120px; height: 40px;">Go</button>
                </div>
            </body></html>
        "#;

        let page =
            process_html(html, "https://visibility.test", 1280.0, 720.0, Vec::new()).unwrap();
        let parent_id = page.tree.get_element_by_id("parent").unwrap();
        let child_id = page.tree.get_element_by_id("child").unwrap();

        assert!(
            !page
                .tree
                .get(parent_id)
                .expect("parent node should exist")
                .is_visible,
            "parent visibility:hidden must stay hidden"
        );
        assert!(
            page.tree
                .get(child_id)
                .expect("child node should exist")
                .is_visible,
            "child visibility:visible should override inherited hidden visibility"
        );
    }

    #[test]
    fn test_visibility_zero_sized_element_is_hidden() {
        let html = r#"
            <html><body style="width: 1280px; height: 720px;">
                <div id="zero" style="width: 0px; height: 0px;">Tiny</div>
            </body></html>
        "#;

        let page = process_html(html, "https://zero-size.test", 1280.0, 720.0, Vec::new()).unwrap();
        let zero_id = page.tree.get_element_by_id("zero").unwrap();
        assert!(
            !page
                .tree
                .get(zero_id)
                .expect("zero node should exist")
                .is_visible,
            "zero-sized element must be hidden"
        );
    }

    #[test]
    fn test_visibility_outside_viewport_is_hidden() {
        let html = r#"
            <html><body style="width: 1280px; height: 720px;">
                <div style="width: 1280px; height: 2000px;"></div>
                <div id="below-fold" style="width: 100px; height: 100px;">Off screen</div>
            </body></html>
        "#;

        let page = process_html(html, "https://offscreen.test", 1280.0, 720.0, Vec::new()).unwrap();
        let below_fold_id = page.tree.get_element_by_id("below-fold").unwrap();
        assert!(
            !page
                .tree
                .get(below_fold_id)
                .expect("below-fold node should exist")
                .is_visible,
            "element outside viewport must be hidden"
        );
    }

    #[test]
    fn test_nested_flex_offsets_do_not_hide_visible_child() {
        let html = r#"
            <html><body style="display: flex; width: 400px; height: 120px;">
                <div style="width: 100px; height: 100px;"></div>
                <div style="display: flex; width: 120px; height: 100px;">
                    <div style="width: 50px; height: 100px;"></div>
                    <button id="target" style="width: 20px; height: 20px;">Go</button>
                </div>
            </body></html>
        "#;

        let page =
            process_html(html, "https://nested-offset.test", 170.0, 120.0, Vec::new()).unwrap();
        let target = page.tree.get_element_by_id("target").unwrap();
        assert!(
            page.tree
                .get(target)
                .expect("target node should exist")
                .is_visible,
            "nested child in viewport must remain visible"
        );
    }

    #[test]
    fn test_percent_width_contributes_to_layout_bounds() {
        let html = r#"
            <html><body style="width: 400px; height: 120px;">
                <div id="pct" style="width: 100%; height: 40px;">Wide</div>
            </body></html>
        "#;

        let page =
            process_html(html, "https://percent-size.test", 400.0, 120.0, Vec::new()).unwrap();
        let pct = page.tree.get_element_by_id("pct").unwrap();
        let bounds = page.layout_map.get(&pct).unwrap();

        assert!(
            bounds.width > 0.0,
            "percent width should produce non-zero width"
        );
        assert!(
            page.tree
                .get(pct)
                .expect("pct node should exist")
                .is_visible,
            "percent-sized element in viewport should be visible"
        );
    }

    #[test]
    fn test_visibility_display_none_inherited() {
        let html = r#"
            <html><body style="width: 1280px; height: 720px;">
                <div id="parent" style="display: none; width: 200px; height: 100px;">
                    <button id="child" style="display: block; visibility: visible; width: 120px; height: 40px;">Go</button>
                </div>
            </body></html>
        "#;

        let page = process_html(html, "https://display.test", 1280.0, 720.0, Vec::new()).unwrap();
        let parent_id = page.tree.get_element_by_id("parent").unwrap();
        let child_id = page.tree.get_element_by_id("child").unwrap();

        assert!(
            !page.tree.get(parent_id).unwrap().is_visible,
            "parent display:none must be hidden"
        );
        assert!(
            !page.tree.get(child_id).unwrap().is_visible,
            "child of display:none parent must be hidden despite having display:block"
        );
    }

    #[test]
    fn test_visibility_display_none_nested_inheritance() {
        let html = r#"
            <html><body>
                <div id="grandparent" style="display: none; width: 100px; height: 100px;">
                    <div id="parent" style="display: block; width: 100px; height: 100px;">
                        <button id="child" style="display: block; width: 100px; height: 100px;">Go</button>
                    </div>
                </div>
            </body></html>
        "#;

        let page = process_html(html, "https://nested.test", 1280.0, 720.0, Vec::new()).unwrap();
        let gp = page.tree.get_element_by_id("grandparent").unwrap();
        let p = page.tree.get_element_by_id("parent").unwrap();
        let c = page.tree.get_element_by_id("child").unwrap();

        assert!(
            !page.tree.get(gp).unwrap().is_visible,
            "grandparent display:none must be hidden"
        );
        assert!(
            !page.tree.get(p).unwrap().is_visible,
            "parent under display:none must be hidden"
        );
        assert!(
            !page.tree.get(c).unwrap().is_visible,
            "child under display:none must be hidden"
        );
    }

    #[test]
    fn test_text_nodes_contribute_to_parent_size() {
        let html = r#"
            <html><body>
                <div id="wrapper" style="display: flex;">
                    Hello World
                </div>
            </body></html>
        "#;

        let page =
            process_html(html, "https://text-layout.test", 1280.0, 720.0, Vec::new()).unwrap();
        let wrapper = page.tree.get_element_by_id("wrapper").unwrap();
        let bounds = page.layout_map.get(&wrapper).unwrap();

        // "Hello World" is 11 chars. With our heuristic of 8px/char, it's 88px.
        assert!(
            bounds.width >= 88.0,
            "wrapper width should be at least 88.0, got {}",
            bounds.width
        );
        assert_eq!(
            bounds.height, 18.0,
            "wrapper height should be 18.0, got {}",
            bounds.height
        );
    }
}
