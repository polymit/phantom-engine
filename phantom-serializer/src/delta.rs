use std::collections::{VecDeque, HashMap, HashSet};
use indextree::NodeId;
use crate::cct_types::{
    CctDelta, CctNode, ElementType, CctAriaRole, CctDisplay, CctVisibility, 
    CctPointerEvents, BoundsConfidence, CctEvents, CctState, IdConfidence
};

#[derive(Debug, Clone)]
pub enum RawMutation {
    NodeInserted { node_id: NodeId, parent_id: NodeId },
    NodeRemoved  { node_id: NodeId, parent_id: NodeId },
    AttrChanged  { node_id: NodeId, attr: String, old_val: Option<String>, new_val: Option<String> },
    TextChanged  { node_id: NodeId, new_text: String },
    ScrollChanged { x: f32, y: f32 },
}

/// Batches raw DOM mutations and coalesces them into minimal CCT deltas.
/// Accumulates events for `window_ms` (default 16 ms, matching one 60fps frame)
/// before exposing them via [`DeltaEngine::coalesce`]. Implements four rules:
/// no-op cancellation, last-attr-wins, parent-removal dominance, and
/// rapid insert-remove cancellation.
pub struct DeltaEngine {
    pending: VecDeque<RawMutation>,
    batch_start: Option<std::time::Instant>,
    window_ms: u64,
}

impl DeltaEngine {
    pub fn new() -> Self {
        Self {
            pending: VecDeque::new(),
            batch_start: None,
            window_ms: 16,
        }
    }

    pub fn push(&mut self, mutation: RawMutation) {
        if self.pending.is_empty() {
            self.batch_start = Some(std::time::Instant::now());
        }
        self.pending.push_back(mutation);
    }

    pub fn is_ready(&self) -> bool {
        if self.pending.is_empty() {
            return false;
        }
        if let Some(start) = self.batch_start {
            start.elapsed().as_millis() as u64 >= self.window_ms
        } else {
            false
        }
    }

    pub fn coalesce(&mut self) -> Vec<CctDelta> {
        let mutations: Vec<RawMutation> = self.pending.drain(..).collect();
        self.batch_start = None;
        apply_coalescing(mutations)
    }
}

impl Default for DeltaEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn apply_coalescing(mutations: Vec<RawMutation>) -> Vec<CctDelta> {
    let mut final_muts = Vec::new();

    let mut inserted = HashSet::new();
    let mut removed = HashSet::new();
    let mut attr_map: HashMap<(NodeId, String), RawMutation> = HashMap::new();
    let mut text_map: HashMap<NodeId, String> = HashMap::new();
    let mut last_scroll = None;

    for m in mutations {
        match m {
            RawMutation::NodeInserted { node_id, .. } => {
                if removed.contains(&node_id) {
                    removed.remove(&node_id);
                } else {
                    inserted.insert(node_id);
                }
            }
            RawMutation::NodeRemoved { node_id, .. } => {
                if inserted.contains(&node_id) {
                    inserted.remove(&node_id);
                } else {
                    removed.insert(node_id);
                }
                
                // Parent removing implicitly cleans up target nodes attributes
                attr_map.retain(|&(id, _), _| id != node_id);
                text_map.remove(&node_id);
            }
            RawMutation::AttrChanged { node_id, attr, old_val, new_val } => {
                if removed.contains(&node_id) { continue; }
                let key = (node_id, attr.clone());
                
                // Rule 1: A -> B -> A
                if let Some(RawMutation::AttrChanged { old_val: orig_old, .. }) = attr_map.get(&key) {
                    if *orig_old == new_val {
                        attr_map.remove(&key);
                        continue;
                    }
                }
                // Rule 2: Keep last value 
                attr_map.insert(key, RawMutation::AttrChanged { node_id, attr, old_val, new_val });
            }
            RawMutation::TextChanged { node_id, new_text } => {
                if removed.contains(&node_id) { continue; }
                text_map.insert(node_id, new_text);
            }
            RawMutation::ScrollChanged { x, y } => {
                last_scroll = Some((x, y));
            }
        }
    }

    // Process Removes
    for id in removed {
        final_muts.push(CctDelta::Remove(format!("n_{}", id)));
    }

    // Process Inserts
    for id in inserted {
        let dummy = CctNode {
            node_id: format!("n_{}", id),
            element_type: ElementType::Div,
            aria_role: CctAriaRole::None,
            x: 0.0, y: 0.0, w: 0.0, h: 0.0,
            bounds_confidence: BoundsConfidence::Reliable,
            display: CctDisplay::B,
            visibility: CctVisibility::V,
            opacity: 1.0,
            pointer_events: CctPointerEvents::A,
            accessible_name: "-".to_string(),
            visible_text: text_map.remove(&id).unwrap_or_else(|| "-".to_string()),
            events: CctEvents::empty(),
            parent_id: "root".to_string(),
            flags: 0,
            state: CctState::empty(),
            id_confidence: IdConfidence::Low,
            relevance: None,
        };
        final_muts.push(CctDelta::Add(dummy));
    }

    // Process Updates
    let mut updated_nodes = HashSet::new();
    for &(node_id, _) in attr_map.keys() {
        updated_nodes.insert(node_id);
    }
    for &node_id in text_map.keys() {
        updated_nodes.insert(node_id);
    }
    
    for id in updated_nodes {
        final_muts.push(CctDelta::Update {
            node_id: format!("n_{}", id),
            display: None,
            bounds: None,
        });
    }

    if let Some((x, y)) = last_scroll {
        final_muts.push(CctDelta::Scroll { x, y });
    }

    final_muts
}
