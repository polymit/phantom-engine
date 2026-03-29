use html5ever::tendril::TendrilSink;
use html5ever::{parse_document, parse_fragment};
use markup5ever::{ns, QualName};

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

/// Parses an HTML fragment in the context of the given tag name.
/// Reserved for future partial-update injection; not yet used by the pipeline.
pub fn parse_html_fragment(html: &str, context_tag: &str) -> DomTree {
    let sink = DomSink::new();
    let context_name = QualName::new(None, ns!(html), markup5ever::LocalName::from(context_tag));
    let parser = parse_fragment(sink, Default::default(), context_name, Vec::new(), false);
    parser.from_utf8().one(html.as_bytes())
}
