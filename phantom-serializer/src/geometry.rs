use std::collections::HashMap;
use indextree::NodeId;
use phantom_core::dom::{DomTree, NodeData};
use phantom_core::layout::bounds::{LayoutEngine, ViewportBounds};

pub struct GeometryMap {
    inner: HashMap<NodeId, ViewportBounds>,
}

impl GeometryMap {
    pub fn new() -> Self {
        Self { inner: HashMap::new() }
    }

    pub fn get(&self, id: NodeId) -> Option<&ViewportBounds> {
        self.inner.get(&id)
    }

    pub fn get_or_zero(&self, id: NodeId) -> ViewportBounds {
        self.inner.get(&id).cloned().unwrap_or_else(ViewportBounds::zero)
    }
}

impl Default for GeometryMap {
    fn default() -> Self {
        Self::new()
    }
}

pub fn extract_geometry(
    tree: &DomTree,
    layout: &LayoutEngine,
    viewport: &ViewportBounds,
) -> GeometryMap {
    let mut map = GeometryMap::new();

    if let Some(root) = tree.document_root {
        process_node_geometry(tree, layout, viewport, root, 0.0, 0.0, &mut map);
    }

    map
}

pub fn is_in_viewport(bounds: &ViewportBounds, viewport: &ViewportBounds) -> bool {
    bounds.intersects(viewport)
}

fn process_node_geometry(
    tree: &DomTree,
    layout: &LayoutEngine,
    viewport: &ViewportBounds,
    node_id: NodeId,
    parent_offset_x: f32,
    parent_offset_y: f32,
    map: &mut GeometryMap,
) {
    let dom_node = tree.get(node_id);
    let mut bounds = layout.get_bounds(node_id);

    // Transform local coordinates to absolute viewport coordinates
    bounds.x += parent_offset_x;
    bounds.y += parent_offset_y;

    if matches!(dom_node.data, NodeData::Element { .. }) {
        if !is_in_viewport(&bounds, viewport) {
            // Early subtree rejection for off-screen content
            map.inner.insert(node_id, ViewportBounds::zero());
            return;
        }

        map.inner.insert(node_id, bounds.clone());

        for child in node_id.children(&tree.arena) {
            process_node_geometry(tree, layout, viewport, child, bounds.x, bounds.y, map);
        }
    } else {
        // Document, Comment, Text have no layout bounds themselves
        map.inner.insert(node_id, ViewportBounds::zero());
        for child in node_id.children(&tree.arena) {
            process_node_geometry(tree, layout, viewport, child, parent_offset_x, parent_offset_y, map);
        }
    }
}
