use phantom_core::ParsedPage;
use phantom_core::dom::NodeData;
use phantom_core::layout::bounds::ViewportBounds;
use crate::cct_types::{
    CctNode, CctPageHeader, SerialiserMode, ElementType, CctDisplay, CctVisibility,
    CctPointerEvents, BoundsConfidence, CctAriaRole, LandmarkType
};
use crate::visibility::compute_visibility;
use crate::geometry::extract_geometry;
use crate::zindex::resolve_zindex;
use crate::semantic::extract_semantics;
use crate::id_stabilizer::stabilise_ids;
use crate::selective::{compute_relevance, should_include_in_selective};
use crate::buffer_pool::BUFFER_POOL;

#[derive(Clone)]
pub struct SerialiserConfig {
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub url: String,
    pub scroll_x: f32,
    pub scroll_y: f32,
    pub total_height: f32,
    pub mode: SerialiserMode,
    pub task_hint: Option<String>,
}

impl Default for SerialiserConfig {
    fn default() -> Self {
        Self {
            viewport_width: 1280.0,
            viewport_height: 720.0,
            url: String::new(),
            scroll_x: 0.0,
            scroll_y: 0.0,
            total_height: 720.0,
            mode: SerialiserMode::Full,
            task_hint: None,
        }
    }
}

pub struct HeadlessSerializer;

impl HeadlessSerializer {
    /// Runs the full 8-stage CCT pipeline over `page` and returns the complete
    /// CCT v0.2 string. Acquires a pre-allocated buffer from the global pool,
    /// then clones and releases it — the caller receives an owned `String`.
    pub fn serialise(page: &ParsedPage, config: &SerialiserConfig) -> String {
        let mut buffer = BUFFER_POOL.acquire();

        let viewport = ViewportBounds::new(
            config.scroll_x,
            config.scroll_y,
            config.viewport_width,
            config.viewport_height,
        );

        let vis_map = compute_visibility(&page.tree, &page.layout, &viewport);
        let geo_map = extract_geometry(&page.tree, &page.layout, &viewport);
        let z_map = resolve_zindex(&page.tree, &geo_map);

        let mut visible_ids = Vec::new();
        if let Some(root) = page.tree.document_root {
            for node_id in root.descendants(&page.tree.arena) {
                if vis_map.is_visible(node_id) {
                    visible_ids.push(node_id);
                }
            }
        }

        let mut actual_mode = config.mode.clone();
        if visible_ids.len() > 500 && actual_mode == SerialiserMode::Full {
            actual_mode = SerialiserMode::Selective;
            tracing::debug!("Switched to Selective mode due to visible node count > 500");
        }

        let sem_map = extract_semantics(&page.tree, &vis_map, &visible_ids);
        let id_map = stabilise_ids(&page.tree, &vis_map);

        let header = CctPageHeader {
            url: config.url.clone(),
            scroll_x: config.scroll_x,
            scroll_y: config.scroll_y,
            viewport_width: config.viewport_width,
            viewport_height: config.viewport_height,
            total_width: config.viewport_width,
            total_height: config.total_height,
            node_count: visible_ids.len(),
            mode: actual_mode.clone(),
        };

        buffer.push_str(&header.to_string());
        buffer.push('\n');

        let hint = config.task_hint.as_deref().unwrap_or("");

        if let Some(root) = page.tree.document_root {
            for node_id in root.descendants(&page.tree.arena) {
                if !visible_ids.contains(&node_id) {
                    continue;
                }
                
                let dom_node = page.tree.get(node_id);
                if !matches!(dom_node.data, NodeData::Element { .. }) {
                    continue;
                }

                let semantic = sem_map.get(node_id).unwrap();
                let is_interactive = semantic.events.click || semantic.events.input || semantic.events.focus || 
                                     matches!(dom_node.data, NodeData::Element{ ref tag_name, .. } if is_interactive_tag(tag_name));
                                     
                let cct_role = CctAriaRole::from_aria_role(&dom_node.aria_role);
                let mut is_landmark = false;
                if let NodeData::Element { tag_name, .. } = &dom_node.data {
                    if LandmarkType::from_tag(tag_name.as_str()).is_some() || LandmarkType::from_aria_role(cct_role.to_cct_code()).is_some() {
                        is_landmark = true;
                    }
                }

                let mut relevance_score = 1.0;
                if actual_mode == SerialiserMode::Selective {
                    relevance_score = compute_relevance(dom_node, semantic, hint);
                    if !should_include_in_selective(dom_node, relevance_score, is_interactive, is_landmark) {
                        continue;
                    }
                }

                if is_landmark {
                    let landmark_type = if let NodeData::Element { tag_name, .. } = &dom_node.data {
                        LandmarkType::from_tag(tag_name.as_str()).or_else(|| LandmarkType::from_aria_role(cct_role.to_cct_code())).unwrap_or(LandmarkType::Main)
                    } else { LandmarkType::Main };
                    
                    if let Some(id_str) = id_map.get_id(node_id) {
                        buffer.push_str(&landmark_type.to_marker(id_str));
                        buffer.push('\n');
                    }
                }

                let bounds = geo_map.get_or_zero(node_id);
                
                let el_type = if let NodeData::Element { tag_name, .. } = &dom_node.data {
                    ElementType::from_tag(tag_name.as_str())
                } else {
                    ElementType::Div
                };

                let parent_id = node_id.ancestors(&page.tree.arena)
                    .nth(1)
                    .and_then(|p| id_map.get_id(p))
                    .unwrap_or("root")
                    .to_string();

                let mut flags: u8 = 0;
                if is_interactive { flags |= 1; }

                let cct_node = CctNode {
                    node_id: id_map.get_id(node_id).unwrap_or("").to_string(),
                    element_type: el_type,
                    aria_role: cct_role,
                    x: bounds.x,
                    y: bounds.y,
                    w: bounds.width,
                    h: bounds.height,
                    bounds_confidence: BoundsConfidence::Reliable,
                    display: CctDisplay::from_display(&dom_node.computed_display),
                    visibility: if z_map.is_occluded(node_id) { CctVisibility::H } else { CctVisibility::from_visibility(&dom_node.computed_visibility) },
                    opacity: dom_node.computed_opacity,
                    pointer_events: CctPointerEvents::from_pointer_events(&dom_node.computed_pointer_events),
                    accessible_name: semantic.accessible_name.clone(),
                    visible_text: semantic.visible_text.clone(),
                    events: semantic.events.clone(),
                    parent_id,
                    flags,
                    state: semantic.state.clone(),
                    id_confidence: id_map.get_confidence(node_id),
                    relevance: if actual_mode == SerialiserMode::Selective { Some(relevance_score) } else { None },
                };

                cct_node.serialise_into(&mut buffer);
                buffer.push('\n');
            }
        }

        let final_string = buffer.clone();
        BUFFER_POOL.release(buffer);
        final_string
    }

    pub fn node_count_from_output(cct: &str) -> usize {
        cct.lines().filter(|l| l.starts_with("n_")).count()
    }
}

fn is_interactive_tag(tag: &str) -> bool {
    let t = tag.to_lowercase();
    matches!(t.as_str(), "button" | "input" | "a" | "select" | "textarea" | "form")
}
