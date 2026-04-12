pub mod node;
pub mod query;
pub mod sink;

pub use self::node::{
    AriaRole, Display, DomNode, EventListenerType, NodeData, PointerEvents, SizeValue, Visibility,
};
pub use self::sink::DomSink;
use indextree::{Arena, NodeId};
use std::num::NonZeroUsize;

/// The live DOM tree produced by the HTML parser.
/// Nodes are stored in an arena indexed by [`NodeId`]; the tree is navigable
/// through indextree's parent/child/sibling relationships.
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
        self.arena
            .get_mut(id)
            .expect("NodeId not found in arena")
            .get_mut()
    }

    pub fn get_text_content(&self, id: NodeId) -> String {
        let mut text = String::new();
        for descendant_id in id.descendants(&self.arena) {
            if descendant_id == id {
                continue;
            }
            let node = self.get(descendant_id);
            if let NodeData::Text { content } = &node.data {
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(content);
            }
        }
        text.trim().to_string()
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
        let results =
            crate::dom::query::query_node_with_selectors(context_node, &self.arena, selector, true);
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

    /// Convert a raw arena index (as stored in JS as a u64) back to a
    /// live [`NodeId`].
    ///
    /// Uses [`Arena::get_node_id_at`] which validates the stamp — so
    /// a stale or garbage index safely returns `None` rather than
    /// indexing into a removed slot.
    pub fn node_id_from_raw(&self, arena_id: u64) -> Option<NodeId> {
        let nz = NonZeroUsize::new(arena_id as usize)?;
        self.arena.get_node_id_at(nz)
    }
}
