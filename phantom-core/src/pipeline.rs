use crate::dom::{DomTree, NodeData, Display, Visibility};
use crate::layout::bounds::{LayoutEngine, LayoutError};
use crate::css::{CssEngine, ComputedStyle};
use indextree::NodeId;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("layout error: {0}")]
    Layout(#[from] LayoutError),
    #[error("parse error: {0}")]
    Parse(String),
}

pub struct ParsedPage {
    pub tree: DomTree,
    pub layout: LayoutEngine,
    pub url: String,
}

pub fn process_html(html: &str, url: &str, viewport_width: f32, viewport_height: f32) -> Result<ParsedPage, CoreError> {
    let mut tree = crate::parser::parse_html(html);
    
    // Pass 1: CSS parsing and initial visibility
    if let Some(root) = tree.document_root {
        apply_css_pass(&mut tree, root, None);
    }
    
    // Pass 2: Layout computation
    let mut layout = LayoutEngine::new();
    
    if let Some(root) = tree.document_root {
        build_layout_tree(&mut layout, &tree, root)?;
        if let Some(taffy_root) = layout.get_taffy_id(root) {
            layout.compute(taffy_root, viewport_width, viewport_height)?;
        }
    }
    
    // Pass 3: Layout-dependent visibility
    if let Some(root) = tree.document_root {
        apply_layout_visibility_pass(&mut tree, &layout, root, viewport_width, viewport_height);
    }
    
    Ok(ParsedPage {
        tree,
        layout,
        url: url.to_string(),
    })
}

fn apply_css_pass(tree: &mut DomTree, node_id: NodeId, parent_style: Option<ComputedStyle>) {
    let inline_style_val = {
        let node = tree.get(node_id);
        if let NodeData::Element { ref attributes, .. } = node.data {
            attributes.get("style").cloned()
        } else {
            None
        }
    };
    
    let computed = CssEngine::compute(inline_style_val.as_deref(), parent_style.as_ref());
    
    {
        let node = tree.get_mut(node_id);
        if matches!(node.data, NodeData::Element { .. }) {
            node.computed_display = computed.display.clone();
            node.computed_visibility = computed.visibility.clone();
            node.computed_opacity = computed.opacity;
            node.computed_pointer_events = computed.pointer_events.clone();
            node.z_index = computed.z_index;
            
            node.is_visible = node.computed_display != Display::None
                && node.computed_visibility != Visibility::Hidden
                && node.computed_opacity > 0.0;
        }
    }
        
    let children: Vec<NodeId> = node_id.children(&tree.arena).collect();
    for child in children {
        apply_css_pass(tree, child, Some(computed.clone()));
    }
}

fn build_layout_tree(layout: &mut LayoutEngine, tree: &DomTree, node_id: NodeId) -> Result<Option<taffy::NodeId>, CoreError> {
    let node = tree.get(node_id);
    
    if matches!(node.data, NodeData::Element { .. }) {
        let style = taffy::Style::default();
        let taffy_id = layout.add_node(node_id, style)?;
        
        let mut child_taffy_ids = Vec::new();
        let children: Vec<NodeId> = node_id.children(&tree.arena).collect();
        for child in children {
            if let Some(child_taffy_id) = build_layout_tree(layout, tree, child)? {
                child_taffy_ids.push(child_taffy_id);
            }
        }
        
        layout.set_children(taffy_id, &child_taffy_ids)?;
        Ok(Some(taffy_id))
    } else {
        Ok(None)
    }
}

fn apply_layout_visibility_pass(tree: &mut DomTree, layout: &LayoutEngine, node_id: NodeId, viewport_width: f32, viewport_height: f32) {
    {
        let node = tree.get_mut(node_id);
        if matches!(node.data, NodeData::Element { .. }) {
            let bounds = layout.get_bounds(node_id);
            let viewport = crate::layout::bounds::ViewportBounds::new(0.0, 0.0, viewport_width, viewport_height);
            
            if node.is_visible {
                node.is_visible = bounds.width > 0.0 
                    && bounds.height > 0.0 
                    && bounds.intersects(&viewport);
            }
        }
    }
    
    let children: Vec<NodeId> = node_id.children(&tree.arena).collect();
    for child in children {
        apply_layout_visibility_pass(tree, layout, child, viewport_width, viewport_height);
    }
}
