use indextree::NodeId;
use phantom_core::dom::{DomTree, NodeData};
use phantom_core::layout::bounds::{LayoutMap, ViewportBounds};
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
    layout_map: &LayoutMap,
    viewport: &ViewportBounds,
) -> GeometryMap {
    let mut map = GeometryMap::new();

    if let Some(root) = tree.document_root {
        for node_id in root.descendants(&tree.arena) {
            let Some(dom_node) = tree.get(node_id) else {
                continue;
            };

            if matches!(dom_node.data, NodeData::Element { .. }) {
                if let Some(abs_bounds) = layout_map.get(&node_id) {
                    let node_bounds = if abs_bounds.intersects(viewport) {
                        abs_bounds.clone()
                    } else {
                        ViewportBounds::zero()
                    };
                    map.inner.insert(node_id, node_bounds);
                }
            } else {
                map.inner.insert(node_id, ViewportBounds::zero());
            }
        }
    }

    map
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
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
        let map = extract_geometry(&page.tree, &page.layout_map, &viewport);

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
