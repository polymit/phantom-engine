use indextree::NodeId;
use phantom_core::dom::{DomTree, NodeData};
use phantom_core::layout::bounds::{LayoutEngine, ViewportBounds};
use std::collections::HashMap;

pub struct GeometryMap {
    inner: HashMap<NodeId, ViewportBounds>,
}

impl GeometryMap {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn get(&self, id: NodeId) -> Option<&ViewportBounds> {
        self.inner.get(&id)
    }

    pub fn get_or_zero(&self, id: NodeId) -> ViewportBounds {
        self.inner
            .get(&id)
            .cloned()
            .unwrap_or_else(ViewportBounds::zero)
    }
}

impl Default for GeometryMap {
    fn default() -> Self {
        Self::new()
    }
}

pub fn extract_geometry(
    tree: &DomTree,
    layout: &LayoutEngine,
    viewport: &ViewportBounds,
) -> GeometryMap {
    let mut map = GeometryMap::new();

    if let Some(root) = tree.document_root {
        process_node_geometry(tree, layout, viewport, root, 0.0, 0.0, &mut map);
    }

    map
}

pub fn is_in_viewport(bounds: &ViewportBounds, viewport: &ViewportBounds) -> bool {
    bounds.intersects(viewport)
}

fn process_node_geometry(
    tree: &DomTree,
    layout: &LayoutEngine,
    viewport: &ViewportBounds,
    node_id: NodeId,
    parent_offset_x: f32,
    parent_offset_y: f32,
    map: &mut GeometryMap,
) {
    let Some(dom_node) = tree.get(node_id) else {
        return;
    };
    let mut abs_bounds = layout.get_bounds(node_id);

    // Transform local coordinates to absolute viewport coordinates
    abs_bounds.x += parent_offset_x;
    abs_bounds.y += parent_offset_y;

    if matches!(dom_node.data, NodeData::Element { .. }) {
        let node_bounds = if is_in_viewport(&abs_bounds, viewport) {
            abs_bounds.clone()
        } else {
            ViewportBounds::zero()
        };
        map.inner.insert(node_id, node_bounds);

        // Keep walking with the element's absolute coordinates even when this
        // element itself is outside the viewport and stored as zero.
        for child in node_id.children(&tree.arena) {
            process_node_geometry(
                tree,
                layout,
                viewport,
                child,
                abs_bounds.x,
                abs_bounds.y,
                map,
            );
        }
    } else {
        // Document, Comment, Text have no layout bounds themselves
        map.inner.insert(node_id, ViewportBounds::zero());
        for child in node_id.children(&tree.arena) {
            process_node_geometry(
                tree,
                layout,
                viewport,
                child,
                parent_offset_x,
                parent_offset_y,
                map,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::extract_geometry;
    use phantom_core::layout::bounds::ViewportBounds;
    use phantom_core::process_html;

    #[test]
    fn child_geometry_uses_absolute_parent_offset_even_when_parent_is_zeroed() {
        let html = r#"
            <html><body style="width: 1280px; height: 3000px;">
                <div style="width: 1280px; height: 1000px;"></div>
                <div id="parent" style="width: 1280px; height: 100px;">
                    <div id="child" style="width: 1280px; height: 300px;"></div>
                </div>
            </body></html>
        "#;
        let page = process_html(html, "https://geometry.test", 1280.0, 720.0).unwrap();
        let viewport = ViewportBounds::new(0.0, 1100.0, 1280.0, 200.0);
        let map = extract_geometry(&page.tree, &page.layout, &viewport);

        let parent_id = page.tree.get_element_by_id("parent").unwrap();
        let child_id = page.tree.get_element_by_id("child").unwrap();

        let parent_bounds = map.get(parent_id).expect("parent bounds must exist");
        let child_bounds = map.get(child_id).expect("child bounds must exist");

        assert_eq!(*parent_bounds, ViewportBounds::zero());
        assert!(
            child_bounds.height > 0.0,
            "child must retain non-zero bounds when it intersects the viewport"
        );
        assert!(
            child_bounds.intersects(&viewport),
            "child should intersect viewport using absolute parent offsets"
        );
    }
}
