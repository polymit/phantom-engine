use cssparser_selectors::{Parser as CssParser, ParserInput};
use indextree::{Arena, NodeId};
use selectors::{
    attr::{AttrSelectorOperation, AttrSelectorOperator, CaseSensitivity, NamespaceConstraint},
    context::{
        IgnoreNthChildForInvalidation, MatchingContext, MatchingMode, NeedsSelectorFlags,
        QuirksMode,
    },
    matching::{matches_selector_list, ElementSelectorFlags},
    parser::{ParseRelative, SelectorParseErrorKind},
    NthIndexCache, SelectorList,
};

use super::{DomNode, NodeData};

// --------------------------------------------------------------------------
// CssString — a String wrapper that satisfies cssparser_selectors 0.31's ToCss bound.
// selectors 0.25 depends on cssparser_selectors ^0.31; our crate re-exports it as
// `cssparser_selectors` to keep it distinct from our cssparser_selectors 0.37.
// --------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct CssString(pub String);

impl cssparser_selectors::ToCss for CssString {
    fn to_css<W: std::fmt::Write>(&self, dest: &mut W) -> std::fmt::Result {
        cssparser_selectors::serialize_identifier(&self.0, dest)
    }
}

impl From<&str> for CssString {
    fn from(s: &str) -> Self {
        CssString(s.to_string())
    }
}

// --------------------------------------------------------------------------
// PhantomSelectorImpl — the SelectorImpl glue type
// --------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PhantomSelectorImpl;

impl selectors::parser::SelectorImpl for PhantomSelectorImpl {
    type ExtraMatchingData<'a> = ();
    type AttrValue = CssString;
    type Identifier = CssString;
    type LocalName = CssString;
    type NamespaceUrl = CssString;
    type NamespacePrefix = CssString;
    type BorrowedNamespaceUrl = CssString;
    type BorrowedLocalName = CssString;
    type NonTSPseudoClass = NonTSPseudoClass;
    type PseudoElement = PseudoElement;
}

// --------------------------------------------------------------------------
// NonTSPseudoClass — empty; we do not support any custom pseudo-classes
// --------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NonTSPseudoClass {}

impl cssparser_selectors::ToCss for NonTSPseudoClass {
    fn to_css<W: std::fmt::Write>(&self, _dest: &mut W) -> std::fmt::Result {
        Ok(())
    }
}

impl selectors::parser::NonTSPseudoClass for NonTSPseudoClass {
    type Impl = PhantomSelectorImpl;
    fn is_active_or_hover(&self) -> bool {
        false
    }
    fn is_user_action_state(&self) -> bool {
        false
    }
}

// --------------------------------------------------------------------------
// PseudoElement — empty; we do not support pseudo-elements
// --------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PseudoElement {}

impl cssparser_selectors::ToCss for PseudoElement {
    fn to_css<W: std::fmt::Write>(&self, _dest: &mut W) -> std::fmt::Result {
        Ok(())
    }
}

impl selectors::parser::PseudoElement for PseudoElement {
    type Impl = PhantomSelectorImpl;
}

// --------------------------------------------------------------------------
// PhantomParser — selector text → SelectorList
// --------------------------------------------------------------------------

pub struct PhantomParser;

impl<'i> selectors::Parser<'i> for PhantomParser {
    type Impl = PhantomSelectorImpl;
    type Error = SelectorParseErrorKind<'i>;
}

// --------------------------------------------------------------------------
// DomElement — borrows an arena node for the lifetime of a match operation
// --------------------------------------------------------------------------

pub struct DomElement<'a> {
    pub node_id: NodeId,
    pub arena: &'a Arena<DomNode>,
}

impl<'a> Clone for DomElement<'a> {
    fn clone(&self) -> Self {
        Self {
            node_id: self.node_id,
            arena: self.arena,
        }
    }
}

impl<'a> std::fmt::Debug for DomElement<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DomElement")
            .field("node_id", &self.node_id)
            .finish()
    }
}

// --------------------------------------------------------------------------
// selectors::Element for DomElement
// --------------------------------------------------------------------------

impl<'a> selectors::Element for DomElement<'a> {
    type Impl = PhantomSelectorImpl;

    fn opaque(&self) -> selectors::OpaqueElement {
        selectors::OpaqueElement::new(&self.node_id)
    }

    fn parent_element(&self) -> Option<Self> {
        let mut curr = self.arena.get(self.node_id)?.parent()?;
        loop {
            let node = self.arena.get(curr)?;
            if matches!(node.get().data, NodeData::Element { .. }) {
                return Some(DomElement {
                    node_id: curr,
                    arena: self.arena,
                });
            }
            curr = node.parent()?;
        }
    }

    fn parent_node_is_shadow_root(&self) -> bool {
        false
    }
    fn containing_shadow_host(&self) -> Option<Self> {
        None
    }
    fn is_pseudo_element(&self) -> bool {
        false
    }

    fn prev_sibling_element(&self) -> Option<Self> {
        let mut curr = self.arena.get(self.node_id)?.previous_sibling()?;
        loop {
            let node = self.arena.get(curr)?;
            if matches!(node.get().data, NodeData::Element { .. }) {
                return Some(DomElement {
                    node_id: curr,
                    arena: self.arena,
                });
            }
            match node.previous_sibling() {
                Some(prev) => curr = prev,
                None => return None,
            }
        }
    }

    fn next_sibling_element(&self) -> Option<Self> {
        let mut curr = self.arena.get(self.node_id)?.next_sibling()?;
        loop {
            let node = self.arena.get(curr)?;
            if matches!(node.get().data, NodeData::Element { .. }) {
                return Some(DomElement {
                    node_id: curr,
                    arena: self.arena,
                });
            }
            match node.next_sibling() {
                Some(next) => curr = next,
                None => return None,
            }
        }
    }

    fn first_element_child(&self) -> Option<Self> {
        for child_id in self.node_id.children(self.arena) {
            let node = self.arena.get(child_id)?;
            if matches!(node.get().data, NodeData::Element { .. }) {
                return Some(DomElement {
                    node_id: child_id,
                    arena: self.arena,
                });
            }
        }
        None
    }

    fn is_html_element_in_html_document(&self) -> bool {
        true
    }

    fn has_local_name(&self, local_name: &CssString) -> bool {
        match &self.arena.get(self.node_id).unwrap().get().data {
            NodeData::Element { tag_name, .. } => tag_name.eq_ignore_ascii_case(&local_name.0),
            _ => false,
        }
    }

    fn has_namespace(&self, _ns: &CssString) -> bool {
        true
    }

    fn is_same_type(&self, other: &Self) -> bool {
        // Two elements are the same type if they have the same tag name.
        let a = self.arena.get(self.node_id).unwrap().get();
        let b = other.arena.get(other.node_id).unwrap().get();
        match (&a.data, &b.data) {
            (NodeData::Element { tag_name: t1, .. }, NodeData::Element { tag_name: t2, .. }) => {
                t1.eq_ignore_ascii_case(t2)
            }
            _ => false,
        }
    }

    fn attr_matches(
        &self,
        _ns: &NamespaceConstraint<&CssString>,
        local_name: &CssString,
        operation: &AttrSelectorOperation<&CssString>,
    ) -> bool {
        let node = self.arena.get(self.node_id).unwrap().get();
        let NodeData::Element { attributes, .. } = &node.data else {
            return false;
        };
        let Some(val) = attributes.get(&local_name.0) else {
            return false;
        };

        match operation {
            AttrSelectorOperation::Exists => true,
            AttrSelectorOperation::WithValue {
                operator,
                case_sensitivity,
                value,
            } => {
                let expected = value.0.as_str();
                match operator {
                    AttrSelectorOperator::Equal => case_cmp(val, expected, *case_sensitivity),
                    AttrSelectorOperator::Includes => val
                        .split_whitespace()
                        .any(|token| case_cmp(token, expected, *case_sensitivity)),
                    AttrSelectorOperator::DashMatch => {
                        case_cmp(val, expected, *case_sensitivity)
                            || val.starts_with(&format!("{}-", expected))
                    }
                    AttrSelectorOperator::Prefix => {
                        !expected.is_empty() && prefix_cmp(val, expected, *case_sensitivity)
                    }
                    AttrSelectorOperator::Suffix => {
                        !expected.is_empty() && suffix_cmp(val, expected, *case_sensitivity)
                    }
                    AttrSelectorOperator::Substring => {
                        !expected.is_empty() && substring_cmp(val, expected, *case_sensitivity)
                    }
                }
            }
        }
    }

    fn match_non_ts_pseudo_class(
        &self,
        _pc: &NonTSPseudoClass,
        _ctx: &mut MatchingContext<'_, Self::Impl>,
    ) -> bool {
        false
    }

    fn match_pseudo_element(
        &self,
        _pe: &PseudoElement,
        _ctx: &mut MatchingContext<'_, Self::Impl>,
    ) -> bool {
        false
    }

    fn apply_selector_flags(&self, _flags: ElementSelectorFlags) {}

    fn is_link(&self) -> bool {
        match &self.arena.get(self.node_id).unwrap().get().data {
            NodeData::Element {
                tag_name,
                attributes,
                ..
            } => tag_name.eq_ignore_ascii_case("a") && attributes.contains_key("href"),
            _ => false,
        }
    }

    fn is_html_slot_element(&self) -> bool {
        false
    }

    fn has_id(&self, id: &CssString, case_sensitivity: CaseSensitivity) -> bool {
        let node = self.arena.get(self.node_id).unwrap().get();
        let NodeData::Element { attributes, .. } = &node.data else {
            return false;
        };
        attributes
            .get("id")
            .is_some_and(|val| case_cmp(val, &id.0, case_sensitivity))
    }

    fn has_class(&self, name: &CssString, case_sensitivity: CaseSensitivity) -> bool {
        let node = self.arena.get(self.node_id).unwrap().get();
        let NodeData::Element { attributes, .. } = &node.data else {
            return false;
        };
        let Some(class_attr) = attributes.get("class") else {
            return false;
        };
        class_attr
            .split_whitespace()
            .any(|token| case_cmp(token, &name.0, case_sensitivity))
    }

    fn imported_part(&self, _name: &CssString) -> Option<CssString> {
        None
    }
    fn is_part(&self, _name: &CssString) -> bool {
        false
    }

    fn is_empty(&self) -> bool {
        !self.node_id.children(self.arena).any(|child_id| {
            matches!(
                self.arena.get(child_id).unwrap().get().data,
                NodeData::Element { .. } | NodeData::Text { .. }
            )
        })
    }

    fn is_root(&self) -> bool {
        match &self.arena.get(self.node_id).unwrap().get().data {
            NodeData::Element { tag_name, .. } => tag_name.eq_ignore_ascii_case("html"),
            _ => false,
        }
    }
}

// --------------------------------------------------------------------------
// Case-comparison helpers
// --------------------------------------------------------------------------

fn case_cmp(val: &str, expected: &str, cs: CaseSensitivity) -> bool {
    match cs {
        CaseSensitivity::CaseSensitive => val == expected,
        CaseSensitivity::AsciiCaseInsensitive => val.eq_ignore_ascii_case(expected),
    }
}

fn prefix_cmp(val: &str, prefix: &str, cs: CaseSensitivity) -> bool {
    if val.len() < prefix.len() {
        return false;
    }
    case_cmp(&val[..prefix.len()], prefix, cs)
}

fn suffix_cmp(val: &str, suffix: &str, cs: CaseSensitivity) -> bool {
    if val.len() < suffix.len() {
        return false;
    }
    case_cmp(&val[val.len() - suffix.len()..], suffix, cs)
}

fn substring_cmp(val: &str, needle: &str, cs: CaseSensitivity) -> bool {
    match cs {
        CaseSensitivity::CaseSensitive => val.contains(needle),
        CaseSensitivity::AsciiCaseInsensitive => val
            .to_ascii_lowercase()
            .contains(&needle.to_ascii_lowercase()),
    }
}

// --------------------------------------------------------------------------
// Public API
// --------------------------------------------------------------------------

pub fn parse_selector_list(selector: &str) -> Option<SelectorList<PhantomSelectorImpl>> {
    let mut input = ParserInput::new(selector);
    let mut parser = CssParser::new(&mut input);
    SelectorList::parse(&PhantomParser, &mut parser, ParseRelative::No).ok()
}

/// Walk every element-descendant of `context_node` and collect those that
/// match `selector_str`. Stops after the first hit when `first_only` is true.
pub fn query_node_with_selectors(
    context_node: NodeId,
    arena: &Arena<DomNode>,
    selector_str: &str,
    first_only: bool,
) -> Vec<NodeId> {
    let mut results = Vec::new();

    let selectors = match parse_selector_list(selector_str) {
        Some(s) => s,
        None => return results,
    };

    let mut cache = NthIndexCache::default();

    for descendant_id in context_node.descendants(arena) {
        if descendant_id == context_node {
            // querySelector/querySelectorAll search within descendants only.
            continue;
        }

        let node = arena.get(descendant_id).unwrap().get();
        if !matches!(node.data, NodeData::Element { .. }) {
            continue;
        }

        let el = DomElement {
            node_id: descendant_id,
            arena,
        };
        let mut ctx = MatchingContext::new(
            MatchingMode::Normal,
            None,
            &mut cache,
            QuirksMode::NoQuirks,
            NeedsSelectorFlags::No,
            IgnoreNthChildForInvalidation::No,
        );

        if matches_selector_list(&selectors, &el, &mut ctx) {
            results.push(descendant_id);
            if first_only {
                break;
            }
        }
    }

    results
}
