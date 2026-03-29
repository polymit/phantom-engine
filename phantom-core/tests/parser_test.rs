#[cfg(test)]
mod tests {
    use phantom_core::parse_html;

    #[test]
    fn test_parse_basic_html() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head><title>Test Page</title></head>
            <body>
                <div id="main">
                    <p>Hello world</p>
                    <button>Click me</button>
                </div>
            </body>
            </html>
        "#;
        let tree = parse_html(html);
        assert!(tree.document_root.is_some());
        // Title should be findable
        assert_eq!(tree.get_title(), "Test Page");
    }

    #[test]
    fn test_parse_attributes() {
        let html = r#"<div id="main" class="container" style="display:none"></div>"#;
        let tree = parse_html(html);
        assert!(tree.document_root.is_some());
    }

    #[test]
    fn test_parse_malformed_html() {
        // html5ever must handle malformed HTML without panic
        let html = r#"<div><p>unclosed<span>tags"#;
        let tree = parse_html(html);
        assert!(tree.document_root.is_some());
    }

    #[test]
    fn test_parse_empty() {
        let tree = parse_html("");
        assert!(tree.document_root.is_some());
    }
}
