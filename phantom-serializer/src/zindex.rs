use crate::geometry::GeometryMap;
use indextree::NodeId;
use phantom_core::dom::{Display, DomTree, NodeData, Visibility};
use phantom_core::layout::bounds::ViewportBounds;
use std::collections::HashMap;

pub struct ZIndexMap {
    inner: HashMap<NodeId, bool>,
}

impl ZIndexMap {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn is_occluded(&self, id: NodeId) -> bool {
        self.inner.get(&id).copied().unwrap_or(false)
    }
}

impl Default for ZIndexMap {
    fn default() -> Self {
        Self::new()
    }
}

fn intersection_area(a: &ViewportBounds, b: &ViewportBounds) -> f32 {
    let x_overlap = (a.x + a.width).min(b.x + b.width) - a.x.max(b.x);
    let y_overlap = (a.y + a.height).min(b.y + b.height) - a.y.max(b.y);
    if x_overlap <= 0.0 || y_overlap <= 0.0 {
        0.0
    } else {
        x_overlap * y_overlap
    }
}

pub fn resolve_zindex(tree: &DomTree, geometry: &GeometryMap) -> ZIndexMap {
    let mut map = ZIndexMap::new();
    const MIN_OCCLUSION_AREA: f32 = 100.0;

    if let Some(root) = tree.document_root {
        let mut elements_with_bounds = Vec::new();

        // Flatten the tree for O(n^2) intersection tests
        for node_id in root.descendants(&tree.arena) {
            let dom_node = tree.get(node_id);
            if matches!(dom_node.data, NodeData::Element { .. }) {
                if let Some(bounds) = geometry.get(node_id) {
                    let z = dom_node.z_index.unwrap_or(0);
                    let has_explicit_z = dom_node.z_index.is_some();
                    let can_occlude = dom_node.computed_display != Display::None
                        && dom_node.computed_visibility != Visibility::Hidden
                        && dom_node.computed_opacity > 0.0;
                    elements_with_bounds.push((
                        node_id,
                        z,
                        has_explicit_z,
                        can_occlude,
                        bounds.clone(),
                    ));
                }
            }
        }

        for &(node_id, node_z, node_explicit_z, _, ref bounds) in &elements_with_bounds {
            let mut is_occluded = false;

            // For Phase 1, any element sharing the viewport with a strictly higher Z-index
            // is considered to occlude it if bounding boxes intersect.
            // TODO: fully conform to CSS stacking context algorithms
            for &(other_id, other_z, other_explicit_z, other_can_occlude, ref other_bounds) in
                &elements_with_bounds
            {
                if other_id == node_id {
                    continue;
                }
                if !node_explicit_z || !other_explicit_z {
                    continue;
                }
                if !other_can_occlude {
                    continue;
                }
                if other_z <= node_z {
                    continue;
                }
                if intersection_area(bounds, other_bounds) < MIN_OCCLUSION_AREA {
                    continue;
                }
                if bounds.intersects(other_bounds) {
                    is_occluded = true;
                    break;
                }
            }
            map.inner.insert(node_id, is_occluded);
        }
    }

    map
}
