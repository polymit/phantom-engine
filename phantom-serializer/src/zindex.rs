use std::collections::HashMap;
use indextree::NodeId;
use phantom_core::dom::{DomTree, NodeData};
use crate::geometry::GeometryMap;

pub struct ZIndexMap {
    inner: HashMap<NodeId, bool>,
}

impl ZIndexMap {
    pub fn new() -> Self {
        Self { inner: HashMap::new() }
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

pub fn resolve_zindex(
    tree: &DomTree,
    geometry: &GeometryMap,
) -> ZIndexMap {
    let mut map = ZIndexMap::new();

    if let Some(root) = tree.document_root {
        let mut elements_with_bounds = Vec::new();
        
        // Flatten the tree for O(n^2) intersection tests
        for node_id in root.descendants(&tree.arena) {
            let dom_node = tree.get(node_id);
            if matches!(dom_node.data, NodeData::Element { .. }) {
                if let Some(bounds) = geometry.get(node_id) {
                    let z = dom_node.z_index.unwrap_or(0);
                    elements_with_bounds.push((node_id, z, bounds.clone()));
                }
            }
        }
        
        for &(node_id, node_z, ref bounds) in &elements_with_bounds {
            let mut is_occluded = false;
            
            // For Phase 1, any element sharing the viewport with a strictly higher Z-index
            // is considered to occlude it if bounding boxes intersect.
            // TODO: fully conform to CSS stacking context algorithms
            for &(other_id, other_z, ref other_bounds) in &elements_with_bounds {
                if other_id == node_id { continue; }
                if other_z > node_z && bounds.intersects(other_bounds) {
                    is_occluded = true;
                    break;
                }
            }
            map.inner.insert(node_id, is_occluded);
        }
    }

    map
}
