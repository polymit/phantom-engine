use crate::geometry::GeometryMap;
use indextree::NodeId;
use phantom_core::dom::{Display, DomTree, NodeData, Visibility};
use phantom_core::layout::bounds::ViewportBounds;
use std::collections::HashMap;

pub struct ZIndexMap {
    inner: HashMap<NodeId, bool>,
}

impl ZIndexMap {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn is_occluded(&self, id: NodeId) -> bool {
        self.inner.get(&id).copied().unwrap_or(false)
    }
}

impl Default for ZIndexMap {
    fn default() -> Self {
        Self::new()
    }
}

fn intersection_area(a: &ViewportBounds, b: &ViewportBounds) -> f32 {
    let x_overlap = (a.x + a.width).min(b.x + b.width) - a.x.max(b.x);
    let y_overlap = (a.y + a.height).min(b.y + b.height) - a.y.max(b.y);
    if x_overlap <= 0.0 || y_overlap <= 0.0 {
        0.0
    } else {
        x_overlap * y_overlap
    }
}

#[derive(Clone)]
struct ZElem {
    id: NodeId,
    z: i32,
    order: usize,
    can_occlude: bool,
    bounds: ViewportBounds,
}

fn has_positive_area(bounds: &ViewportBounds) -> bool {
    bounds.width > 0.0 && bounds.height > 0.0
}
fn layer_is_above(other: &ZElem, node: &ZElem) -> bool {
    other.z > node.z || (other.z == node.z && other.order > node.order)
}
fn bucket_range(bounds: &ViewportBounds, cell_size: f32) -> Option<(i32, i32, i32, i32)> {
    if !has_positive_area(bounds) {
        return None;
    }
    let x0 = (bounds.x / cell_size).floor() as i32;
    let y0 = (bounds.y / cell_size).floor() as i32;
    let x1 = ((bounds.x + bounds.width - f32::EPSILON) / cell_size).floor() as i32;
    let y1 = ((bounds.y + bounds.height - f32::EPSILON) / cell_size).floor() as i32;
    Some((x0, y0, x1, y1))
}

pub fn resolve_zindex(tree: &DomTree, geometry: &GeometryMap) -> ZIndexMap {
    let mut map = ZIndexMap::new();
    const MIN_OCCLUSION_AREA: f32 = 100.0;
    const BUCKET_SIZE: f32 = 128.0;

    if let Some(root) = tree.document_root {
        let mut elems = Vec::new();

        for (order, node_id) in root.descendants(&tree.arena).enumerate() {
            let Some(dom_node) = tree.get(node_id) else {
                continue;
            };
            if !matches!(dom_node.data, NodeData::Element { .. }) {
                continue;
            }
            let Some(bounds) = geometry.get(node_id) else {
                continue;
            };
            let can_occlude = dom_node.computed_display != Display::None
                && dom_node.computed_visibility != Visibility::Hidden
                && dom_node.computed_opacity > 0.0;
            elems.push(ZElem {
                id: node_id,
                z: dom_node.z_index.unwrap_or(0),
                order,
                can_occlude,
                bounds: bounds.clone(),
            });
        }

        let mut buckets: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
        for (idx, elem) in elems.iter().enumerate() {
            if !elem.can_occlude {
                continue;
            }
            let Some((x0, y0, x1, y1)) = bucket_range(&elem.bounds, BUCKET_SIZE) else {
                continue;
            };
            for x in x0..=x1 {
                for y in y0..=y1 {
                    buckets.entry((x, y)).or_default().push(idx);
                }
            }
        }

        let mut seen = vec![0u32; elems.len()];
        let mut stamp: u32 = 0;
        for (idx, node) in elems.iter().enumerate() {
            let mut is_occluded = false;
            stamp = stamp.wrapping_add(1);
            if stamp == 0 {
                seen.fill(0);
                stamp = 1;
            }

            let Some((x0, y0, x1, y1)) = bucket_range(&node.bounds, BUCKET_SIZE) else {
                map.inner.insert(node.id, false);
                continue;
            };

            'outer: for x in x0..=x1 {
                for y in y0..=y1 {
                    let Some(candidates) = buckets.get(&(x, y)) else {
                        continue;
                    };
                    for &other_idx in candidates {
                        if other_idx == idx {
                            continue;
                        }
                        if seen[other_idx] == stamp {
                            continue;
                        }
                        seen[other_idx] = stamp;

                        let other = &elems[other_idx];
                        if !layer_is_above(other, node) {
                            continue;
                        }
                        if !node.bounds.intersects(&other.bounds) {
                            continue;
                        }
                        if intersection_area(&node.bounds, &other.bounds) < MIN_OCCLUSION_AREA {
                            continue;
                        }
                        is_occluded = true;
                        break 'outer;
                    }
                }
            }

            map.inner.insert(node.id, is_occluded);
        }

        // Any element missing a geometry entry is treated as not occluded.
        for node_id in root.descendants(&tree.arena) {
            let Some(dom_node) = tree.get(node_id) else {
                continue;
            };
            if !matches!(dom_node.data, NodeData::Element { .. }) {
                continue;
            }
            map.inner.entry(node_id).or_insert(false);
        }
    }

    map
}

#[cfg(test)]
mod tests {
    use super::resolve_zindex;
    use phantom_core::layout::bounds::ViewportBounds;
    use phantom_core::process_html;

    #[test]
    fn explicit_overlay_occludes_implicit_parent() {
        let html = r#"
            <html><body style="width: 1280px; height: 720px;">
                <div id="target" style="width: 300px; height: 300px;">
                    <div id="overlay" style="width: 300px; height: 300px; z-index: 100;">Modal</div>
                </div>
            </body></html>
        "#;
        let page = process_html(html, "https://z.test", 1280.0, 720.0).unwrap();
        let viewport = ViewportBounds::new(0.0, 0.0, 1280.0, 720.0);
        let geo = crate::geometry::extract_geometry(&page.tree, &page.layout, &viewport);
        let z = resolve_zindex(&page.tree, &geo);

        let target = page.tree.get_element_by_id("target").unwrap();
        let overlay = page.tree.get_element_by_id("overlay").unwrap();
        assert!(
            z.is_occluded(target),
            "higher z-index child should occlude parent even when parent has implicit z-index"
        );
        assert!(
            !z.is_occluded(overlay),
            "top overlay should not be occluded"
        );
    }
}
