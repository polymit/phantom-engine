#[cfg(test)]
mod tests {
    use phantom_core::parse_html;

    #[test]
    fn test_query_selector_by_id() {
        let tree = parse_html(
            r#"
            <html><body>
                <div id="main"><p id="para">Hello</p></div>
            </body></html>
        "#,
        );
        let node = tree.get_element_by_id("main");
        assert!(node.is_some());
        let node = tree.get_element_by_id("para");
        assert!(node.is_some());
        let node = tree.get_element_by_id("nonexistent");
        assert!(node.is_none());
    }

    #[test]
    fn test_query_selector_tag() {
        let tree = parse_html(
            r#"
            <html><body>
                <div><p>One</p><p>Two</p><p>Three</p></div>
            </body></html>
        "#,
        );
        let first = tree.query_selector("p");
        assert!(first.is_some());
        let all = tree.query_selector_all("p");
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_query_selector_class() {
        let tree = parse_html(
            r#"
            <html><body>
                <div class="container">
                    <button class="btn primary">Click</button>
                    <button class="btn secondary">Cancel</button>
                </div>
            </body></html>
        "#,
        );
        let all_btns = tree.query_selector_all(".btn");
        assert_eq!(all_btns.len(), 2);
    }

    #[test]
    fn test_query_selector_none() {
        let tree = parse_html("<html><body><p>Hello</p></body></html>");
        let result = tree.query_selector("nonexistent-tag");
        assert!(result.is_none());
    }

    #[test]
    fn test_get_elements_by_tag_name() {
        let tree = parse_html(
            r#"
            <html><body>
                <input type="text"/>
                <input type="password"/>
                <input type="submit"/>
            </body></html>
        "#,
        );
        let inputs = tree.get_elements_by_tag_name("input");
        assert_eq!(inputs.len(), 3);
    }

    #[test]
    fn test_attr_prefix_utf8_does_not_panic() {
        let tree = parse_html(
            r#"
            <html><body>
                <div data-name="éclair"></div>
            </body></html>
        "#,
        );

        let node = tree.query_selector(r#"[data-name^="é"]"#);
        assert!(node.is_some(), "UTF-8 prefix selector must match safely");
    }

    #[test]
    fn test_attr_suffix_utf8_does_not_panic() {
        let tree = parse_html(
            r#"
            <html><body>
                <div data-name="café"></div>
            </body></html>
        "#,
        );

        let node = tree.query_selector(r#"[data-name$="é"]"#);
        assert!(node.is_some(), "UTF-8 suffix selector must match safely");
    }

    #[test]
    fn test_attr_dashmatch_case_insensitive_matches() {
        let tree = parse_html(
            r#"
            <html><body>
                <div lang="EN-US"></div>
            </body></html>
        "#,
        );

        let node = tree.query_selector(r#"[lang|="en" i]"#);
        assert!(
            node.is_some(),
            "dash-match should honor ASCII-insensitive flag"
        );
    }
}
