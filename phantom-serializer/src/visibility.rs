use indextree::NodeId;
use phantom_core::dom::{Display, DomTree, NodeData, Visibility};
use phantom_core::layout::bounds::{LayoutMap, ViewportBounds};
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

pub fn compute_visibility(
    tree: &DomTree,
    layout_map: &LayoutMap,
    viewport: &ViewportBounds,
) -> VisibilityMap {
    let mut map = VisibilityMap::new();

    if let Some(root) = tree.document_root {
        process_node_visibility(tree, layout_map, viewport, root, true, &mut map);
    }

    map
}

fn process_node_visibility(
    tree: &DomTree,
    layout_map: &LayoutMap,
    viewport: &ViewportBounds,
    root_node_id: NodeId,
    initial_parent_visible: bool,
    map: &mut VisibilityMap,
) {
    let mut stack = vec![(root_node_id, initial_parent_visible)];

    while let Some((node_id, parent_visible)) = stack.pop() {
        if !parent_visible {
            map.set(node_id, false);
            let mut children: Vec<_> = node_id.children(&tree.arena).collect();
            children.reverse();
            for child in children {
                stack.push((child, false));
            }
            continue;
        }

        let Some(dom_node) = tree.get(node_id) else {
            map.set(node_id, false);
            let mut children: Vec<_> = node_id.children(&tree.arena).collect();
            children.reverse();
            for child in children {
                stack.push((child, false));
            }
            continue;
        };

        let visible = match &dom_node.data {
            NodeData::Document => true,
            NodeData::Comment { .. } => false,
            NodeData::Text { content } => !content.trim().is_empty(),
            NodeData::Element { .. } => {
                if let Some(bounds) = layout_map.get(&node_id) {
                    let c1 = dom_node.computed_display != Display::None;
                    let c2 = dom_node.computed_visibility != Visibility::Hidden;
                    let c3 = dom_node.computed_opacity > 0.0;
                    let c4 = bounds.width > 0.0;
                    let c5 = bounds.height > 0.0;
                    let c6 = bounds.intersects(viewport);
                    c1 && c2 && c3 && c4 && c5 && c6
                } else {
                    false
                }
            }
        };

        map.set(node_id, visible);

        let mut children: Vec<_> = node_id.children(&tree.arena).collect();
        children.reverse();
        for child in children {
            stack.push((child, visible));
        }
    }
}
