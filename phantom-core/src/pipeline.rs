use crate::css::{ComputedStyle, CssEngine};
use crate::dom::{Display, DomTree, NodeData, SizeValue, Visibility};
use crate::layout::bounds::{LayoutEngine, LayoutError, LayoutMap, ViewportBounds};
use indextree::NodeId;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("layout error: {0}")]
    Layout(#[from] LayoutError),
    #[error("parse error: {0}")]
    Parse(String),
}

#[derive(Clone)]
pub struct ParsedPage {
    pub tree: DomTree,
    pub layout_map: LayoutMap,
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
        apply_css_pass(&mut tree, root, None, false);
    }

    rebuild_page_from_tree(tree, url, viewport_width, viewport_height)
}

/// Rebuild layout + visibility for an existing DOM tree with computed styles.
///
/// This is used when callers persist only the DOM snapshot (which is `Send`)
/// and need to reconstruct a fresh [`ParsedPage`] on demand.
pub fn rebuild_page_from_tree(
    mut tree: DomTree,
    url: &str,
    viewport_width: f32,
    viewport_height: f32,
) -> Result<ParsedPage, CoreError> {
    // Pass 2: Layout computation
    let mut layout = LayoutEngine::new();

    if let Some(doc_root) = tree.document_root {
        build_layout_tree(&mut layout, &tree, doc_root)?;

        // Document is never added to Taffy; locate the <html> element as the real root.
        let html_node = doc_root.children(&tree.arena).find(|&child| {
            tree.get(child)
                .is_some_and(|node| matches!(node.data, NodeData::Element { .. }))
        });
        if let Some(html_id) = html_node {
            if let Some(taffy_root) = layout.get_taffy_id(html_id) {
                layout.compute(taffy_root, viewport_width, viewport_height)?;
            }
        }
    }

    // Pass 3: Final visibility state using CSS + layout bounds.
    if let Some(root) = tree.document_root {
        let viewport = ViewportBounds::new(0.0, 0.0, viewport_width, viewport_height);
        apply_layout_visibility_pass(&mut tree, &layout, &viewport, root, (0.0, 0.0), false);
    }

    // Pass 4: Convert live Taffy tree to static absolute map for Serialization.
    // This allows ParsedPage to be Send + Sync and moved across threads.
    let layout_map = layout.compute_absolute_map(&tree);

    Ok(ParsedPage {
        tree,
        layout_map,
        url: url.to_string(),
    })
}

fn apply_css_pass(
    tree: &mut DomTree,
    node_id: NodeId,
    parent_style: Option<ComputedStyle>,
    ancestor_display_none: bool,
) {
    let inline_style_val = tree.get(node_id).and_then(|node| {
        if let NodeData::Element { ref attributes, .. } = node.data {
            attributes.get("style").cloned()
        } else {
            None
        }
    });

    let computed = CssEngine::compute(inline_style_val.as_deref(), parent_style.as_ref());

    let mut currently_display_none = ancestor_display_none;
    if let Some(node) = tree.get_mut(node_id) {
        if matches!(node.data, NodeData::Element { .. }) {
            node.computed_display = computed.display.clone();
            node.computed_visibility = computed.visibility.clone();
            node.computed_opacity = computed.opacity;
            node.computed_pointer_events = computed.pointer_events.clone();
            node.computed_width = computed.width.clone();
            node.computed_height = computed.height.clone();
            node.z_index = computed.z_index;

            let is_display_none = node.computed_display == Display::None;
            currently_display_none = ancestor_display_none || is_display_none;

            node.is_visible = !currently_display_none
                && node.computed_visibility != Visibility::Hidden
                && node.computed_opacity > 0.0;
        }
    }

    let children: Vec<NodeId> = node_id.children(&tree.arena).collect();
    for child in children {
        apply_css_pass(tree, child, Some(computed.clone()), currently_display_none);
    }
}

fn build_layout_tree(
    layout: &mut LayoutEngine,
    tree: &DomTree,
    node_id: NodeId,
) -> Result<Option<taffy::NodeId>, CoreError> {
    let Some(node) = tree.get(node_id) else {
        return Ok(None);
    };

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

        if let Some(w) = &node.computed_width {
            style.size.width = match w {
                SizeValue::Length(v) => taffy::Dimension::length(*v),
                SizeValue::Percent(v) => taffy::Dimension::percent(*v),
                SizeValue::Auto => taffy::Dimension::auto(),
            };
        }
        if let Some(h) = &node.computed_height {
            style.size.height = match h {
                SizeValue::Length(v) => taffy::Dimension::length(*v),
                SizeValue::Percent(v) => taffy::Dimension::percent(*v),
                SizeValue::Auto => taffy::Dimension::auto(),
            };
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
    } else if let NodeData::Text { ref content } = node.data {
        let trimmed = content.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }

        let char_count = trimmed.chars().count() as f32;
        let style = taffy::Style {
            size: taffy::Size {
                // Heuristic: ~8.5px avg width, 18px line height
                width: taffy::Dimension::length(char_count * 8.5),
                height: taffy::Dimension::length(18.0),
            },
            ..Default::default()
        };

        let taffy_id = layout.add_node(node_id, style)?;
        Ok(Some(taffy_id))
    } else {
        // Document / Comment — not a layout node, but recurse so element
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
    ancestor_hidden: bool,
) {
    let mut next_offset = parent_offset;
    let (element_visibility, currently_hidden) = {
        let Some(node) = tree.get(node_id) else {
            return;
        };
        match &node.data {
            NodeData::Element { .. } => {
                let local_bounds = layout.get_bounds(node_id);
                let abs_x = parent_offset.0 + local_bounds.x;
                let abs_y = parent_offset.1 + local_bounds.y;
                let bounds =
                    ViewportBounds::new(abs_x, abs_y, local_bounds.width, local_bounds.height);
                next_offset = (abs_x, abs_y);

                let is_display_none = node.computed_display == Display::None;
                let is_hidden = ancestor_hidden || is_display_none;

                let c1 = !is_hidden;
                let c2 = node.computed_visibility != Visibility::Hidden;
                let c3 = node.computed_opacity > 0.0;
                let c4 = bounds.width > 0.0;
                let c5 = bounds.height > 0.0;
                let c6 = bounds.intersects(viewport);
                (Some(c1 && c2 && c3 && c4 && c5 && c6), is_hidden)
            }
            _ => (None, ancestor_hidden),
        }
    };
    if let Some(visible) = element_visibility {
        if let Some(node) = tree.get_mut(node_id) {
            node.is_visible = visible;
        }
    }
    let children: Vec<NodeId> = node_id.children(&tree.arena).collect();
    for child in children {
        apply_layout_visibility_pass(tree, layout, viewport, child, next_offset, currently_hidden);
    }
}
