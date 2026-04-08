pub mod click;
pub mod cookies;
pub mod evaluate;
pub mod navigate;
pub mod press_key;
pub mod scene_graph;
pub mod snapshot;
pub mod subscribe;
pub mod tabs;
pub mod type_text;

pub(crate) fn escape_js_single_quoted(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '\'' => escaped.push_str("\\'"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\u{2028}' => escaped.push_str("\\u2028"),
            '\u{2029}' => escaped.push_str("\\u2029"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::escape_js_single_quoted;

    #[test]
    fn escapes_single_quote_and_backslash() {
        let escaped = escape_js_single_quoted(r"foo\'bar\baz");
        assert_eq!(escaped, r"foo\\\'bar\\baz");
    }

    #[test]
    fn escapes_control_and_line_separator_chars() {
        let input = "a\nb\rc\td\u{2028}e\u{2029}f";
        let escaped = escape_js_single_quoted(input);
        assert_eq!(escaped, "a\\nb\\rc\\td\\u2028e\\u2029f");
    }
}
