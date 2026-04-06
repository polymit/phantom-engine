use crate::css::{ComputedStyle, CssEngine};
use crate::dom::{Display, DomTree, NodeData, Visibility};
use crate::layout::bounds::{LayoutEngine, LayoutError, ViewportBounds};
use indextree::NodeId;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("layout error: {0}")]
    Layout(#[from] LayoutError),
    #[error("parse error: {0}")]
    Parse(String),
}

pub struct ParsedPage {
    pub tree: DomTree,
    pub layout: LayoutEngine,
    pub url: String,
}

/// Parses `html` into a DOM tree, applies CSS cascade, computes layout, and
/// returns a [`ParsedPage`] ready for CCT serialisation. Returns an error if
/// the layout pass fails.
pub fn process_html(
    html: &str,
    url: &str,
    viewport_width: f32,
    viewport_height: f32,
) -> Result<ParsedPage, CoreError> {
    let mut tree = crate::parser::parse_html(html);

    // Pass 1: CSS parsing and initial visibility
    if let Some(root) = tree.document_root {
        apply_css_pass(&mut tree, root, None);
    }

    // Pass 2: Layout computation
    let mut layout = LayoutEngine::new();

    if let Some(doc_root) = tree.document_root {
        build_layout_tree(&mut layout, &tree, doc_root)?;

        // Document is never added to Taffy; locate the <html> element as the real root.
        let html_node = doc_root
            .children(&tree.arena)
            .find(|&child| matches!(tree.get(child).data, NodeData::Element { .. }));
        if let Some(html_id) = html_node {
            if let Some(taffy_root) = layout.get_taffy_id(html_id) {
                layout.compute(taffy_root, viewport_width, viewport_height)?;
            }
        }
    }

    // Pass 3: Final visibility state using CSS + layout bounds.
    if let Some(root) = tree.document_root {
        let viewport = ViewportBounds::new(0.0, 0.0, viewport_width, viewport_height);
        apply_layout_visibility_pass(&mut tree, &layout, &viewport, root, (0.0, 0.0));
    }

    Ok(ParsedPage {
        tree,
        layout,
        url: url.to_string(),
    })
}

fn apply_css_pass(tree: &mut DomTree, node_id: NodeId, parent_style: Option<ComputedStyle>) {
    let inline_style_val = {
        let node = tree.get(node_id);
        if let NodeData::Element { ref attributes, .. } = node.data {
            attributes.get("style").cloned()
        } else {
            None
        }
    };

    let computed = CssEngine::compute(inline_style_val.as_deref(), parent_style.as_ref());

    {
        let node = tree.get_mut(node_id);
        if matches!(node.data, NodeData::Element { .. }) {
            node.computed_display = computed.display.clone();
            node.computed_visibility = computed.visibility.clone();
            node.computed_opacity = computed.opacity;
            node.computed_pointer_events = computed.pointer_events.clone();
            node.computed_width = computed.width;
            node.computed_height = computed.height;
            node.z_index = computed.z_index;

            node.is_visible = node.computed_display != Display::None
                && node.computed_visibility != Visibility::Hidden
                && node.computed_opacity > 0.0;
        }
    }

    let children: Vec<NodeId> = node_id.children(&tree.arena).collect();
    for child in children {
        apply_css_pass(tree, child, Some(computed.clone()));
    }
}

fn build_layout_tree(
    layout: &mut LayoutEngine,
    tree: &DomTree,
    node_id: NodeId,
) -> Result<Option<taffy::NodeId>, CoreError> {
    let node = tree.get(node_id);

    if matches!(node.data, NodeData::Element { .. }) {
        let mut style = taffy::Style {
            display: match node.computed_display {
                crate::dom::node::Display::None => taffy::Display::None,
                crate::dom::node::Display::Flex => taffy::Display::Flex,
                crate::dom::node::Display::Grid => taffy::Display::Grid,
                _ => taffy::Display::Block,
            },
            ..Default::default()
        };

        if let Some(w) = node.computed_width {
            style.size.width = taffy::Dimension::length(w);
        }
        if let Some(h) = node.computed_height {
            style.size.height = taffy::Dimension::length(h);
        }

        let taffy_id = layout.add_node(node_id, style)?;

        let mut child_taffy_ids = Vec::new();
        let children: Vec<NodeId> = node_id.children(&tree.arena).collect();
        for child in children {
            if let Some(child_taffy_id) = build_layout_tree(layout, tree, child)? {
                child_taffy_ids.push(child_taffy_id);
            }
        }

        layout.set_children(taffy_id, &child_taffy_ids)?;
        Ok(Some(taffy_id))
    } else {
        // Document / Text / Comment — not a layout node, but recurse so element
        // descendants still get registered in the Taffy tree.
        let children: Vec<NodeId> = node_id.children(&tree.arena).collect();
        for child in children {
            build_layout_tree(layout, tree, child)?;
        }
        Ok(None)
    }
}

fn apply_layout_visibility_pass(
    tree: &mut DomTree,
    layout: &LayoutEngine,
    viewport: &ViewportBounds,
    node_id: NodeId,
    parent_offset: (f32, f32),
) {
    let mut next_offset = parent_offset;
    let element_visibility = {
        let node = tree.get(node_id);
        match &node.data {
            NodeData::Element { .. } => {
                let mut bounds = layout.get_bounds(node_id);
                bounds.x += parent_offset.0;
                bounds.y += parent_offset.1;
                next_offset = (bounds.x, bounds.y);

                let c1 = node.computed_display != Display::None;
                let c2 = node.computed_visibility != Visibility::Hidden;
                let c3 = node.computed_opacity > 0.0;
                let c4 = bounds.width > 0.0;
                let c5 = bounds.height > 0.0;
                let c6 = bounds.intersects(viewport);
                Some(c1 && c2 && c3 && c4 && c5 && c6)
            }
            _ => None,
        }
    };
    if let Some(visible) = element_visibility {
        tree.get_mut(node_id).is_visible = visible;
    }
    let children: Vec<NodeId> = node_id.children(&tree.arena).collect();
    for child in children {
        apply_layout_visibility_pass(tree, layout, viewport, child, next_offset);
    }
}
