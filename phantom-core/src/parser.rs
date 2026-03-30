use html5ever::tendril::TendrilSink;
use html5ever::parse_document;

use crate::dom::sink::DomSink;
use crate::dom::DomTree;

/// Parses a full HTML document string into a [`DomTree`].
/// Uses html5ever's tree-building algorithm, which handles malformed markup
/// the same way a browser would.
pub fn parse_html(html: &str) -> DomTree {
    let sink = DomSink::new();
    let parser = parse_document(sink, Default::default());
    parser.from_utf8().one(html.as_bytes())
}

