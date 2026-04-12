use indextree::NodeId;
use phantom_core::dom::{Display, DomTree, NodeData, Visibility};
use phantom_core::layout::bounds::{LayoutEngine, ViewportBounds};
use std::collections::HashMap;

pub struct VisibilityMap {
    inner: HashMap<NodeId, bool>,
}

impl VisibilityMap {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn is_visible(&self, id: NodeId) -> bool {
        self.inner.get(&id).copied().unwrap_or(false)
    }

    pub fn set(&mut self, id: NodeId, visible: bool) {
        self.inner.insert(id, visible);
    }
}

impl Default for VisibilityMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Walks the DOM tree and builds a [`VisibilityMap`] flagging each node as
/// visible or hidden. Hides a node if `display:none`, `visibility:hidden`,
/// `opacity:0`, or its bounds are negative. Hidden ancestors propagate
/// invisibility to all descendants.
pub fn compute_visibility(
    tree: &DomTree,
    layout: &LayoutEngine,
    viewport: &ViewportBounds,
) -> VisibilityMap {
    let mut map = VisibilityMap::new();

    if let Some(root) = tree.document_root {
        process_node_visibility(tree, layout, viewport, root, true, (0.0, 0.0), &mut map);
    }

    map
}

fn process_node_visibility(
    tree: &DomTree,
    layout: &LayoutEngine,
    viewport: &ViewportBounds,
    node_id: NodeId,
    parent_visible: bool,
    parent_offset: (f32, f32),
    map: &mut VisibilityMap,
) {
    if !parent_visible {
        map.set(node_id, false);
        for child in node_id.children(&tree.arena) {
            process_node_visibility(tree, layout, viewport, child, false, parent_offset, map);
        }
        return;
    }

    let Some(dom_node) = tree.get(node_id) else {
        map.set(node_id, false);
        for child in node_id.children(&tree.arena) {
            process_node_visibility(tree, layout, viewport, child, false, parent_offset, map);
        }
        return;
    };
    let (visible, next_offset) = match &dom_node.data {
        NodeData::Document => (true, parent_offset),
        NodeData::Comment { .. } => (false, parent_offset),
        NodeData::Text { content } => (!content.trim().is_empty(), parent_offset),
        NodeData::Element { .. } => {
            let mut bounds = layout.get_bounds(node_id);
            bounds.x += parent_offset.0;
            bounds.y += parent_offset.1;

            let c1 = dom_node.computed_display != Display::None;
            let c2 = dom_node.computed_visibility != Visibility::Hidden;
            let c3 = dom_node.computed_opacity > 0.0;
            let c4 = bounds.width > 0.0;
            let c5 = bounds.height > 0.0;
            let c6 = bounds.intersects(viewport);
            (c1 && c2 && c3 && c4 && c5 && c6, (bounds.x, bounds.y))
        }
    };

    map.set(node_id, visible);

    for child in node_id.children(&tree.arena) {
        process_node_visibility(tree, layout, viewport, child, visible, next_offset, map);
    }
}
