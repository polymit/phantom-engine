use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;

use html5ever::tendril::StrTendril;
use html5ever::tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink};
use markup5ever::{Attribute, ExpandedName, QualName};

use crate::css::CssEngine;
use crate::dom::node::{AriaRole, DomNode, NodeData};
use crate::dom::DomTree;

pub struct DomSink {
    pub tree: RefCell<DomTree>,
    errors: RefCell<Vec<String>>,
    names: RefCell<HashMap<indextree::NodeId, QualName>>,
}

impl DomSink {
    pub fn new() -> Self {
        let mut tree = DomTree::new();
        let root_id = tree.arena.new_node(DomNode::new(NodeData::Document));
        tree.document_root = Some(root_id);

        Self {
            tree: RefCell::new(tree),
            errors: RefCell::new(Vec::new()),
            names: RefCell::new(HashMap::new()),
        }
    }
}

impl Default for DomSink {
    fn default() -> Self {
        Self::new()
    }
}

impl TreeSink for DomSink {
    // ElemName is required as per the html5ever 0.38 API change
    type Output = DomTree;
    type Handle = indextree::NodeId;
    type ElemName<'a> = ExpandedName<'a>;

    fn finish(self) -> DomTree {
        self.tree.into_inner()
    }

    fn get_document(&self) -> Self::Handle {
        self.tree.borrow().document_root.unwrap()
    }

    fn get_template_contents(&self, target: &Self::Handle) -> Self::Handle {
        *target
    }

    fn same_node(&self, x: &Self::Handle, y: &Self::Handle) -> bool {
        x == y
    }

    fn elem_name<'a>(&'a self, target: &'a Self::Handle) -> Self::ElemName<'a> {
        // SAFETY: `self.names` is only mutated during `create_element`, which cannot
        // overlap with `elem_name` because html5ever calls them sequentially on `&self`.
        // The returned `ExpandedName` borrows from the `QualName` stored in the map,
        // so we need a reference with lifetime `'a` that the `RefCell::borrow()` guard
        // cannot provide (guard would be dropped at end of this function). The raw
        // pointer dereference is safe because no mutable borrow is active.
        let names_ref = unsafe { &*self.names.as_ptr() };
        names_ref.get(target).expect("Node not found").expanded()
    }

    fn create_element(
        &self,
        name: QualName,
        attrs: Vec<Attribute>,
        _flags: ElementFlags,
    ) -> Self::Handle {
        let tag_name = name.local.to_string();
        let mut attributes = HashMap::new();
        let mut aria_role = None;
        let mut aria_label = None;
        let mut computed_style = crate::css::properties::ComputedStyle::default();

        for attr in attrs {
            let key = attr.name.local.to_string();
            let val = attr.value.to_string();

            if key == "role" {
                aria_role = match val.to_lowercase().as_str() {
                    "button" => Some(AriaRole::Button),
                    "link" => Some(AriaRole::Link),
                    "main" => Some(AriaRole::Main),
                    "header" => Some(AriaRole::Header),
                    "footer" => Some(AriaRole::Footer),
                    "aside" => Some(AriaRole::Aside),
                    "form" => Some(AriaRole::Form),
                    "dialog" => Some(AriaRole::Dialog),
                    "search" => Some(AriaRole::Search),
                    "list" => Some(AriaRole::List),
                    "table" => Some(AriaRole::Table),
                    "input" => Some(AriaRole::Input),
                    "navigation" => Some(AriaRole::Navigation),
                    _ => Some(AriaRole::None),
                };
            } else if key == "aria-label" {
                aria_label = Some(val.clone());
            } else if key == "style" {
                computed_style = CssEngine::parse_inline_style(&val);
            }

            attributes.insert(key, val);
        }

        let node_data = NodeData::Element {
            tag_name,
            attributes,
        };

        let mut dom_node = DomNode::new(node_data);
        dom_node.aria_role = aria_role;
        dom_node.aria_label = aria_label;
        dom_node.computed_display = computed_style.display;
        dom_node.computed_visibility = computed_style.visibility;
        dom_node.computed_opacity = computed_style.opacity;
        dom_node.computed_pointer_events = computed_style.pointer_events;
        dom_node.z_index = computed_style.z_index;

        let node_id = self.tree.borrow_mut().arena.new_node(dom_node);
        self.names.borrow_mut().insert(node_id, name);
        node_id
    }

    fn create_comment(&self, text: StrTendril) -> Self::Handle {
        let node_data = NodeData::Comment {
            content: text.to_string(),
        };
        self.tree.borrow_mut().arena.new_node(DomNode::new(node_data))
    }

    fn create_pi(&self, _target: StrTendril, data: StrTendril) -> Self::Handle {
        let node_data = NodeData::Comment {
            content: data.to_string(),
        };
        self.tree.borrow_mut().arena.new_node(DomNode::new(node_data))
    }

    fn append(&self, parent: &Self::Handle, child: NodeOrText<Self::Handle>) {
        let child_id = match child {
            NodeOrText::AppendNode(node_id) => node_id,
            NodeOrText::AppendText(text) => {
                let node_data = NodeData::Text {
                    content: text.to_string(),
                };
                self.tree.borrow_mut().arena.new_node(DomNode::new(node_data))
            }
        };
        parent.append(child_id, &mut self.tree.borrow_mut().arena);
    }

    fn append_before_sibling(
        &self,
        sibling: &Self::Handle,
        new_node: NodeOrText<Self::Handle>,
    ) {
        let child_id = match new_node {
            NodeOrText::AppendNode(node_id) => node_id,
            NodeOrText::AppendText(text) => {
                let node_data = NodeData::Text {
                    content: text.to_string(),
                };
                self.tree.borrow_mut().arena.new_node(DomNode::new(node_data))
            }
        };
        sibling.insert_before(child_id, &mut self.tree.borrow_mut().arena);
    }

    fn append_based_on_parent_node(
        &self,
        element: &Self::Handle,
        prev_element: &Self::Handle,
        child: NodeOrText<Self::Handle>,
    ) {
        let has_parent = self.tree.borrow().arena.get(*element).unwrap().parent().is_some();
        if has_parent {
            self.append_before_sibling(element, child);
        } else {
            self.append(prev_element, child);
        }
    }

    fn append_doctype_to_document(
        &self,
        _name: StrTendril,
        _public_id: StrTendril,
        _system_id: StrTendril,
    ) {
        // No-op per spec
    }

    fn add_attrs_if_missing(&self, target: &Self::Handle, attrs: Vec<Attribute>) {
        let mut tree = self.tree.borrow_mut();
        if let Some(node) = tree.arena.get_mut(*target) {
            let inner = node.get_mut();
            if let NodeData::Element { attributes, .. } = &mut inner.data {
                for attr in attrs {
                    let key = attr.name.local.to_string();
                    attributes.entry(key).or_insert_with(|| attr.value.to_string());
                }
            }
        }
    }

    fn remove_from_parent(&self, target: &Self::Handle) {
        target.detach(&mut self.tree.borrow_mut().arena);
    }

    fn reparent_children(&self, node: &Self::Handle, new_parent: &Self::Handle) {
        let mut children = Vec::new();
        let tree = self.tree.borrow();
        for child in node.children(&tree.arena) {
            children.push(child);
        }
        drop(tree);
        for child in children {
            new_parent.append(child, &mut self.tree.borrow_mut().arena);
        }
    }

    fn mark_script_already_started(&self, _node: &Self::Handle) {
        // No-op
    }

    fn set_quirks_mode(&self, _mode: QuirksMode) {
        // No-op
    }

    fn parse_error(&self, msg: Cow<'static, str>) {
        self.errors.borrow_mut().push(msg.into_owned());
    }
}
