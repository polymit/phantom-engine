use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct ViewportBounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl ViewportBounds {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    pub fn intersects(&self, other: &ViewportBounds) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }

    pub fn contains_point(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.x + self.width && py >= self.y && py <= self.y + self.height
    }

    pub fn is_empty(&self) -> bool {
        self.width == 0.0 || self.height == 0.0
    }

    pub fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }
}

pub struct LayoutEngine {
    taffy: taffy::TaffyTree,
    node_map: HashMap<indextree::NodeId, taffy::NodeId>,
}

#[derive(Debug, thiserror::Error)]
pub enum LayoutError {
    #[error("taffy error: {0}")]
    Taffy(#[from] taffy::TaffyError),
    #[error("node not found in layout tree")]
    NodeNotFound,
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutEngine {
    pub fn new() -> Self {
        Self {
            taffy: taffy::TaffyTree::new(),
            node_map: HashMap::new(),
        }
    }

    pub fn add_node(
        &mut self,
        dom_id: indextree::NodeId,
        style: taffy::Style,
    ) -> Result<taffy::NodeId, LayoutError> {
        let taffy_id = self.taffy.new_leaf(style)?;
        self.node_map.insert(dom_id, taffy_id);
        Ok(taffy_id)
    }

    pub fn set_children(
        &mut self,
        parent: taffy::NodeId,
        children: &[taffy::NodeId],
    ) -> Result<(), LayoutError> {
        self.taffy.set_children(parent, children)?;
        Ok(())
    }

    pub fn compute(
        &mut self,
        root: taffy::NodeId,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Result<(), LayoutError> {
        let available_space = taffy::Size {
            width: taffy::AvailableSpace::Definite(viewport_width),
            height: taffy::AvailableSpace::Definite(viewport_height),
        };
        self.taffy.compute_layout(root, available_space)?;
        Ok(())
    }

    pub fn get_bounds(&self, dom_id: indextree::NodeId) -> ViewportBounds {
        if let Some(taffy_id) = self.node_map.get(&dom_id) {
            if let Ok(layout) = self.taffy.layout(*taffy_id) {
                // IMPORTANT: bounds logic confidence flag
                let bounds = ViewportBounds::new(
                    layout.location.x,
                    layout.location.y,
                    layout.size.width,
                    layout.size.height,
                );
                return bounds;
            }
        }
        ViewportBounds::zero()
    }

    pub fn get_taffy_id(&self, dom_id: indextree::NodeId) -> Option<taffy::NodeId> {
        self.node_map.get(&dom_id).copied()
    }
}
