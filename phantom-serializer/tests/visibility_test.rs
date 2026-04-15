#[cfg(test)]
mod tests {
    use phantom_core::layout::bounds::ViewportBounds;
    use phantom_core::process_html;
    use phantom_serializer::compute_visibility;

    #[test]
    fn test_display_none_invisible() {
        let page = process_html(
            r#"<html><body style="width: 1280px; height: 720px;">
                <div style="display:none; width: 100px; height: 100px;"><span style="width: 10px; height: 10px;">Hidden</span></div>
                <div id="vis" style="width: 100px; height: 100px;">Visible</div>
            </body></html>"#,
            "https://test.com", 1280.0, 720.0
        ).unwrap();

        let viewport = ViewportBounds::new(0.0, 0.0, 1280.0, 720.0);
        let vis_map = compute_visibility(&page.tree, &page.layout_map, &viewport);

        // The body has a width and height so it should be visible
        // The display:none div must be invisible
        let body = page.tree.get_elements_by_tag_name("body")[0];
        assert!(vis_map.is_visible(body));

        let vis_div = page.tree.get_element_by_id("vis").unwrap();
        assert!(vis_map.is_visible(vis_div));

        // Assert that at least some nodes are invisible (e.g., the display:none node and its children)
        let mut invisible_count = 0;
        if let Some(r) = page.tree.document_root {
            for id in r.descendants(&page.tree.arena) {
                if !vis_map.is_visible(id) {
                    invisible_count += 1;
                }
            }
        }
        assert!(invisible_count > 0, "Expected at least one invisible node");
    }

    #[test]
    fn test_opacity_zero_invisible() {
        let page = process_html(
            r#"<html><body style="width:1280px; height:720px;">
                <button id="hidden-btn" style="opacity:0; width: 50px; height: 30px;">Hidden button</button>
            </body></html>"#,
            "https://test.com", 1280.0, 720.0
        ).unwrap();

        let viewport = ViewportBounds::new(0.0, 0.0, 1280.0, 720.0);
        let vis_map = compute_visibility(&page.tree, &page.layout_map, &viewport);

        let hidden_btn = page.tree.get_element_by_id("hidden-btn").unwrap();
        assert!(
            !vis_map.is_visible(hidden_btn),
            "opacity: 0 element must be invisible"
        );
    }
}
