use crate::cct_types::{CctAriaRole, IdConfidence};
use crate::visibility::VisibilityMap;
use indextree::NodeId;
use phantom_core::dom::{DomTree, NodeData};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

pub struct StableIdMap {
    inner: HashMap<NodeId, (String, IdConfidence)>,
    used_ids: HashSet<String>,
    counter: usize,
}

impl StableIdMap {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
            used_ids: HashSet::new(),
            counter: 0,
        }
    }

    pub fn get_id(&self, node_id: NodeId) -> Option<&str> {
        self.inner.get(&node_id).map(|(id, _)| id.as_str())
    }

    pub fn get_confidence(&self, node_id: NodeId) -> IdConfidence {
        self.inner
            .get(&node_id)
            .map(|(_, conf)| conf.clone())
            .unwrap_or(IdConfidence::Low)
    }
}

impl Default for StableIdMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Assigns a stable, human-readable CCT ID to every node in the tree.
/// Priority: `data-agent-id` → `data-testid` → aria-label hash → DOM id
/// → visible-text hash → structural-path hash. Deduplicates collisions with
/// a suffix counter. Returns a [`StableIdMap`] covering the full arena.
pub fn stabilise_ids(tree: &DomTree, _visible_nodes: &VisibilityMap) -> StableIdMap {
    let mut map = StableIdMap::new();
    if let Some(root) = tree.document_root {
        process_node_ids(tree, root, "", 0, &mut map);
    }
    map
}

fn process_node_ids(
    tree: &DomTree,
    node_id: NodeId,
    parent_path: &str,
    child_idx: usize,
    map: &mut StableIdMap,
) {
    let dom_node = tree.get(node_id);
    let mut path = format!("{}/{}", parent_path, child_idx);

    let (cct_id, conf) = match &dom_node.data {
        NodeData::Element {
            tag_name,
            attributes,
            ..
        } => {
            path = format!("{}/{}[{}]", parent_path, tag_name, child_idx);

            let cct_role = CctAriaRole::from_aria_role(&dom_node.aria_role);
            let role_code = cct_role.to_cct_code();

            // Priority 1
            if let Some(id) = attributes.get("data-agent-id") {
                (id.clone(), IdConfidence::High)
            }
            // Priority 2
            else if let Some(id) = attributes.get("data-testid") {
                (id.clone(), IdConfidence::High)
            }
            // Priority 3
            else if let Some(label) = attributes.get("aria-label").or(attributes.get("alt")) {
                let mut hasher = rustc_hash::FxHasher::default();
                label.hash(&mut hasher);
                role_code.hash(&mut hasher);
                (format!("n_{:x}", hasher.finish()), IdConfidence::High)
            }
            // Priority 4
            else if let Some(id) = attributes.get("id").filter(|s| !is_framework_auto_id(s)) {
                (id.clone(), IdConfidence::Medium)
            } else {
                let text = tree.get_text_content(node_id);
                if !text.is_empty() {
                    let mut hasher = rustc_hash::FxHasher::default();
                    text.hash(&mut hasher);
                    role_code.hash(&mut hasher);
                    (format!("n_{:x}", hasher.finish()), IdConfidence::Medium)
                }
                // Priority 6: structural path hash
                else {
                    let mut hasher = rustc_hash::FxHasher::default();
                    path.hash(&mut hasher);
                    (format!("n_{:x}", hasher.finish()), IdConfidence::Low)
                }
            }
        }
        NodeData::Text { content } => {
            let mut hasher = rustc_hash::FxHasher::default();
            path.hash(&mut hasher);
            content.hash(&mut hasher);
            (format!("n_{:x}", hasher.finish()), IdConfidence::Low)
        }
        NodeData::Document | NodeData::Comment { .. } => {
            (format!("n_{}", map.counter), IdConfidence::Low)
        }
    };

    map.counter += 1;
    let mut final_id = cct_id.clone();
    let mut suffix = 1;

    while map.used_ids.contains(&final_id) {
        final_id = format!("{}_{}", cct_id, suffix);
        suffix += 1;
    }

    map.used_ids.insert(final_id.clone());
    map.inner.insert(node_id, (final_id, conf));

    for (idx, child) in node_id.children(&tree.arena).enumerate() {
        process_node_ids(tree, child, &path, idx, map);
    }
}

fn is_framework_auto_id(id: &str) -> bool {
    id.starts_with("yui_3_")
        || id.starts_with(":r")
        || id.starts_with("__next")
        || id.contains("ember")
}

#[cfg(test)]
mod tests {
    use super::stabilise_ids;
    use crate::visibility::VisibilityMap;
    use phantom_core::dom::{DomNode, DomTree, NodeData};
    use std::collections::HashMap;
    use std::hash::{Hash, Hasher};

    #[test]
    fn text_node_id_uses_fxhasher_directly() {
        let mut tree = DomTree::new();
        let root = tree.arena.new_node(DomNode::new(NodeData::Document));
        tree.document_root = Some(root);

        let div = tree.arena.new_node(DomNode::new(NodeData::Element {
            tag_name: "div".to_string(),
            attributes: HashMap::new(),
        }));
        root.append(div, &mut tree.arena);

        let text_content = "hello";
        let text = tree.arena.new_node(DomNode::new(NodeData::Text {
            content: text_content.to_string(),
        }));
        div.append(text, &mut tree.arena);

        let ids = stabilise_ids(&tree, &VisibilityMap::new());
        let actual = ids.get_id(text).expect("text node id should exist");

        let mut hasher = rustc_hash::FxHasher::default();
        "/0/div[0]/0".hash(&mut hasher);
        text_content.hash(&mut hasher);
        let expected = format!("n_{:x}", hasher.finish());

        assert_eq!(actual, expected);
    }
}
