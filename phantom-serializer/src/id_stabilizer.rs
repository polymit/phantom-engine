use std::collections::{HashMap, HashSet};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use indextree::NodeId;
use phantom_core::dom::{DomTree, NodeData};
use crate::cct_types::{CctAriaRole, IdConfidence};
use crate::visibility::VisibilityMap;

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
        self.inner.get(&node_id).map(|(_, conf)| conf.clone()).unwrap_or(IdConfidence::Low)
    }
}

impl Default for StableIdMap {
    fn default() -> Self {
        Self::new()
    }
}

pub fn stabilise_ids(
    tree: &DomTree,
    _visible_nodes: &VisibilityMap,
) -> StableIdMap {
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
        NodeData::Element { tag_name, attributes, .. } => {
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
                let mut hasher = DefaultHasher::new();
                label.hash(&mut hasher);
                role_code.hash(&mut hasher);
                (format!("n_{:x}", hasher.finish()), IdConfidence::High)
            }
            // Priority 4
            else if let Some(id) = attributes.get("id").filter(|s| !is_framework_auto_id(s)) {
                (id.clone(), IdConfidence::Medium)
            }
            // Priority 5
            else if let Some(text) = get_text_content(tree, node_id).filter(|s| !s.is_empty()) {
                let mut hasher = DefaultHasher::new();
                text.hash(&mut hasher);
                role_code.hash(&mut hasher);
                (format!("n_{:x}", hasher.finish()), IdConfidence::Medium)
            }
            // Priority 6 (Position hash is omitted if layout is unavailable, so we use structural hash as priority 6/7)
            else {
                let mut hasher = DefaultHasher::new();
                path.hash(&mut hasher);
                (format!("n_{:x}", hasher.finish()), IdConfidence::Low)
            }
        }
        NodeData::Text { content } => {
            let mut hasher = DefaultHasher::new();
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

fn get_text_content(tree: &DomTree, node_id: NodeId) -> Option<String> {
    let mut text = String::new();
    for descendant in node_id.descendants(&tree.arena) {
        if descendant == node_id { continue; }
        if let NodeData::Text { content } = &tree.get(descendant).data {
            text.push_str(content);
            text.push(' ');
        }
    }
    let trimmed = text.trim();
    if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
}
