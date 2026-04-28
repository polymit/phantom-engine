use phantom_core::dom::NodeData;
use phantom_core::{DomTree, NodeId};

use crate::error::{FuzzerError, Result};

pub(crate) fn render_doc(tree: &DomTree) -> Result<String> {
    let Some(root) = tree.document_root else {
        return Err(FuzzerError::Serialize("document root missing".to_string()));
    };
    let mut out = String::new();
    for child in root.children(&tree.arena) {
        render_node(tree, child, &mut out)?;
    }
    Ok(out)
}

fn render_node(tree: &DomTree, node_id: NodeId, out: &mut String) -> Result<()> {
    let Some(node) = tree.get(node_id) else {
        return Err(FuzzerError::Serialize(
            "stale node during render".to_string(),
        ));
    };
    match &node.data {
        NodeData::Document => {
            for child in node_id.children(&tree.arena) {
                render_node(tree, child, out)?;
            }
        }
        NodeData::Element {
            tag_name,
            attributes,
        } => {
            out.push('<');
            out.push_str(tag_name);

            // Keep this stable so case diffs stay reviewable.
            let mut attrs: Vec<_> = attributes.iter().collect();
            attrs.sort_by(|lhs, rhs| lhs.0.cmp(rhs.0));
            for (key, val) in attrs {
                out.push(' ');
                out.push_str(key);
                out.push_str("=\"");
                push_attr(out, val);
                out.push('"');
            }

            if is_void(tag_name) {
                out.push('>');
                return Ok(());
            }

            out.push('>');
            for child in node_id.children(&tree.arena) {
                render_node(tree, child, out)?;
            }
            out.push_str("</");
            out.push_str(tag_name);
            out.push('>');
        }
        NodeData::Text { content } => push_text(out, content),
        NodeData::Comment { content } => {
            out.push_str("<!--");
            out.push_str(content);
            out.push_str("-->");
        }
    }
    Ok(())
}

fn push_text(out: &mut String, text: &str) {
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(ch),
        }
    }
}

fn push_attr(out: &mut String, text: &str) {
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
}

fn is_void(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}
