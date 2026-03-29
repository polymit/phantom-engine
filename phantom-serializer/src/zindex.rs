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
        process_node_zindex(tree, geometry, root, &mut map);
    }

    map
}

fn process_node_zindex(
    tree: &DomTree,
    geometry: &GeometryMap,
    node_id: NodeId,
    map: &mut ZIndexMap,
) {
    let dom_node = tree.get(node_id);
    let mut is_occluded = false;

    if matches!(dom_node.data, NodeData::Element { .. }) {
        if let Some(bounds) = geometry.get(node_id) {
            let node_z = dom_node.z_index.unwrap_or(0);

            let siblings = node_id.preceding_siblings(&tree.arena).chain(node_id.following_siblings(&tree.arena));
            
            for sibling_id in siblings {
                if sibling_id == node_id { continue; }
                let sibling_node = tree.get(sibling_id);
                
                if matches!(sibling_node.data, NodeData::Element { .. }) {
                    let sib_z = sibling_node.z_index.unwrap_or(0);
                    if sib_z > node_z {
                        if let Some(sib_bounds) = geometry.get(sibling_id) {
                            if bounds.intersects(sib_bounds) {
                                is_occluded = true;
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    map.inner.insert(node_id, is_occluded);

    for child in node_id.children(&tree.arena) {
        process_node_zindex(tree, geometry, child, map);
    }
}
