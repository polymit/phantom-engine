use std::collections::HashMap;
use indextree::NodeId;
use phantom_core::dom::{DomTree, NodeData, Display, Visibility};
use phantom_core::layout::bounds::{LayoutEngine, ViewportBounds};

pub struct VisibilityMap {
    inner: HashMap<NodeId, bool>,
}

impl VisibilityMap {
    pub fn new() -> Self {
        Self { inner: HashMap::new() }
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

pub fn compute_visibility(
    tree: &DomTree,
    layout: &LayoutEngine,
    viewport: &ViewportBounds,
) -> VisibilityMap {
    let mut map = VisibilityMap::new();

    if let Some(root) = tree.document_root {
        process_node_visibility(tree, layout, viewport, root, &mut map);
    }

    map
}

fn process_node_visibility(
    tree: &DomTree,
    layout: &LayoutEngine,
    viewport: &ViewportBounds,
    node_id: NodeId,
    map: &mut VisibilityMap,
) {
    let children: Vec<NodeId> = node_id.children(&tree.arena).collect();
    for &child in &children {
        process_node_visibility(tree, layout, viewport, child, map);
    }

    let dom_node = tree.get(node_id);
    let visible = match &dom_node.data {
        NodeData::Document | NodeData::Comment { .. } => false,
        NodeData::Text { content } => {
            !content.trim().is_empty()
        }
        NodeData::Element { .. } => {
            let bounds = layout.get_bounds(node_id);
            let c1 = dom_node.computed_display != Display::None;
            let c2 = dom_node.computed_visibility != Visibility::Hidden;
            let c3 = dom_node.computed_opacity > 0.0;
            let c4 = bounds.width > 0.0;
            let c5 = bounds.height > 0.0;
            let c6 = bounds.intersects(viewport);
            c1 && c2 && c3 && c4 && c5 && c6
        }
    };

    map.set(node_id, visible);

    if !visible {
        propagate_invisible(tree, node_id, map);
    }
}

fn propagate_invisible(tree: &DomTree, node_id: NodeId, map: &mut VisibilityMap) {
    for child in node_id.children(&tree.arena) {
        if map.is_visible(child) {
            map.set(child, false);
            propagate_invisible(tree, child, map);
        }
    }
}
