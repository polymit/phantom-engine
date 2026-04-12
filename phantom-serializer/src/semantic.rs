use crate::cct_types::{CctEvents, CctState};
use crate::visibility::VisibilityMap;
use indextree::NodeId;
use phantom_core::dom::{AriaRole, DomTree, NodeData};
use std::collections::HashMap;

pub struct SemanticInfo {
    pub accessible_name: String,
    pub visible_text: String,
    pub events: CctEvents,
    pub state: CctState,
}

pub struct SemanticMap {
    inner: HashMap<NodeId, SemanticInfo>,
}

impl SemanticMap {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn get(&self, id: NodeId) -> Option<&SemanticInfo> {
        self.inner.get(&id)
    }
}

impl Default for SemanticMap {
    fn default() -> Self {
        Self::new()
    }
}

pub fn extract_semantics(
    tree: &DomTree,
    visible_nodes: &VisibilityMap,
    visible_node_ids: &[NodeId],
) -> SemanticMap {
    let mut map = SemanticMap::new();

    let results: Vec<(NodeId, SemanticInfo)> = visible_node_ids
        .iter()
        .filter_map(|&node_id| {
            let dom_node = tree.get(node_id);
            if !matches!(dom_node.data, NodeData::Element { .. }) {
                return None;
            }

            let name = get_accessible_name(tree, visible_nodes, node_id);
            let text = get_visible_text(tree, visible_nodes, node_id);
            let events = CctEvents::from_event_listeners(&dom_node.event_listeners);

            let state = if let NodeData::Element { attributes, .. } = &dom_node.data {
                CctState::from_attributes(attributes)
            } else {
                CctState::empty()
            };

            Some((
                node_id,
                SemanticInfo {
                    accessible_name: name,
                    visible_text: text,
                    events,
                    state,
                },
            ))
        })
        .collect();

    for (node_id, info) in results {
        map.inner.insert(node_id, info);
    }

    map
}

fn truncate_to_100(mut s: String) -> String {
    if s.is_empty() {
        return "-".to_string();
    }
    if s.chars().count() > 100 {
        let end = s.char_indices().nth(100).map(|(i, _)| i).unwrap_or(s.len());
        s.truncate(end);
    }
    s
}

fn get_accessible_name(tree: &DomTree, visible_nodes: &VisibilityMap, node_id: NodeId) -> String {
    let dom_node = tree.get(node_id);
    if let NodeData::Element {
        tag_name,
        attributes,
        ..
    } = &dom_node.data
    {
        if let Some(label) = attributes
            .get("aria-label")
            .filter(|v| !v.trim().is_empty())
        {
            return truncate_to_100(label.trim().to_string());
        }
        if tag_name == "img" {
            if let Some(alt) = attributes.get("alt").filter(|v| !v.trim().is_empty()) {
                return truncate_to_100(alt.trim().to_string());
            }
        }
        if let Some(title) = attributes.get("title").filter(|v| !v.trim().is_empty()) {
            return truncate_to_100(title.trim().to_string());
        }
        if tag_name == "input" || tag_name == "textarea" {
            if let Some(placeholder) = attributes
                .get("placeholder")
                .filter(|v| !v.trim().is_empty())
            {
                return truncate_to_100(placeholder.trim().to_string());
            }
        }
        if tag_name == "button"
            || tag_name == "a"
            || matches!(
                dom_node.aria_role,
                Some(AriaRole::Button) | Some(AriaRole::Link)
            )
        {
            let text = get_visible_text(tree, visible_nodes, node_id);
            if text != "-" {
                return text;
            }
        }
    }
    "-".to_string()
}

fn get_visible_text(tree: &DomTree, visible_nodes: &VisibilityMap, node_id: NodeId) -> String {
    let mut text = String::new();
    for descendant in node_id.descendants(&tree.arena) {
        if descendant == node_id {
            continue;
        }
        if !visible_nodes.is_visible(descendant) {
            continue;
        }
        if let NodeData::Text { content } = &tree.get(descendant).data {
            text.push_str(content);
            text.push(' ');
        }
    }

    let collapsed: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    truncate_to_100(collapsed)
}
