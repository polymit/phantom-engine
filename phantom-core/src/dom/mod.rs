pub mod node;
pub mod sink;
pub mod query;

use indextree::{Arena, NodeId};
pub use self::node::{AriaRole, Display, DomNode, EventListenerType, NodeData, PointerEvents, Visibility};
pub use self::sink::DomSink;

#[derive(Debug, Clone)]
pub struct DomTree {
    pub arena: Arena<DomNode>,
    pub document_root: Option<NodeId>,
}

impl Default for DomTree {
    fn default() -> Self {
        Self::new()
    }
}

impl DomTree {
    pub fn new() -> Self {
        Self {
            arena: Arena::new(),
            document_root: None,
        }
    }

    pub fn get(&self, id: NodeId) -> &DomNode {
        self.arena.get(id).expect("NodeId not found in arena").get()
    }

    pub fn get_mut(&mut self, id: NodeId) -> &mut DomNode {
        self.arena.get_mut(id).expect("NodeId not found in arena").get_mut()
    }

    pub fn get_tag_name(&self, id: NodeId) -> String {
        let node = self.get(id);
        match &node.data {
            NodeData::Element { tag_name, .. } => tag_name.clone(),
            _ => String::new(),
        }
    }

    pub fn get_text_content(&self, id: NodeId) -> String {
        let mut text = String::new();
        for descendant_id in id.descendants(&self.arena) {
            let node = self.get(descendant_id);
            if let NodeData::Text { content } = &node.data {
                text.push_str(content);
            }
        }
        text
    }

    pub fn get_title(&self) -> String {
        if let Some(root) = self.document_root {
            for descendant_id in root.descendants(&self.arena) {
                let node = self.get(descendant_id);
                if let NodeData::Element { tag_name, .. } = &node.data {
                    if tag_name == "title" {
                        return self.get_text_content(descendant_id);
                    }
                }
            }
        }
        String::new()
    }

    pub fn query_selector(&self, selector: &str) -> Option<NodeId> {
        if let Some(root) = self.document_root {
            self.query_selector_from(selector, root)
        } else {
            None
        }
    }

    pub fn query_selector_all(&self, selector: &str) -> Vec<NodeId> {
        if let Some(root) = self.document_root {
            crate::dom::query::query_node_with_selectors(root, &self.arena, selector, false)
        } else {
            Vec::new()
        }
    }

    pub fn query_selector_from(&self, selector: &str, context_node: NodeId) -> Option<NodeId> {
        let results = crate::dom::query::query_node_with_selectors(context_node, &self.arena, selector, true);
        results.into_iter().next()
    }

    pub fn get_element_by_id(&self, id: &str) -> Option<NodeId> {
        if let Some(root) = self.document_root {
            for descendant_id in root.descendants(&self.arena) {
                let node = self.get(descendant_id);
                if let NodeData::Element { attributes, .. } = &node.data {
                    if let Some(val) = attributes.get("id") {
                        if val == id {
                            return Some(descendant_id);
                        }
                    }
                }
            }
        }
        None
    }

    pub fn get_elements_by_tag_name(&self, tag: &str) -> Vec<NodeId> {
        let mut results = Vec::new();
        if let Some(root) = self.document_root {
            for descendant_id in root.descendants(&self.arena) {
                let node = self.get(descendant_id);
                if let NodeData::Element { tag_name, .. } = &node.data {
                    if tag_name.eq_ignore_ascii_case(tag) {
                        results.push(descendant_id);
                    }
                }
            }
        }
        results
    }
}
