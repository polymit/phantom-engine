use crate::cct_types::CctDelta;
use indextree::NodeId;
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Instant;

#[derive(Debug, Clone)]
pub enum RawMutation {
    NodeInserted {
        node_id: NodeId,
        parent_id: NodeId,
    },
    NodeRemoved {
        node_id: NodeId,
        parent_id: NodeId,
    },
    AttrChanged {
        node_id: NodeId,
        attr: String,
        old_val: Option<String>,
        new_val: Option<String>,
    },
    TextChanged {
        node_id: NodeId,
        new_text: String,
    },
    ScrollChanged {
        x: f32,
        y: f32,
    },
}

/// Batches raw DOM mutations and coalesces them into minimal CCT deltas.
/// Implements four rules:
/// no-op cancellation, last-attr-wins, parent-removal dominance, and
/// rapid insert-remove cancellation.
pub struct DeltaEngine {
    pending: VecDeque<RawMutation>,
    batch_start: Option<Instant>,
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
            self.batch_start = Some(Instant::now());
        }
        self.pending.push_back(mutation);
    }

    pub fn coalesce(&mut self) -> Vec<CctDelta> {
        if self.pending.is_empty() {
            return Vec::new();
        }
        if let Some(start) = self.batch_start {
            if (start.elapsed().as_millis() as u64) < self.window_ms {
                return Vec::new();
            }
        }
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
    let mut parent_of: HashMap<NodeId, NodeId> = HashMap::new();
    let mut attr_map: HashMap<(NodeId, String), RawMutation> = HashMap::new();
    let mut text_map: HashMap<NodeId, String> = HashMap::new();
    let mut last_scroll = None;

    for m in mutations {
        match m {
            RawMutation::NodeInserted { node_id, parent_id } => {
                parent_of.insert(node_id, parent_id);
                if has_removed_ancestor(node_id, &removed, &parent_of) {
                    continue;
                }
                if removed.contains(&node_id) {
                    removed.remove(&node_id);
                } else {
                    inserted.insert(node_id);
                }
            }
            RawMutation::NodeRemoved { node_id, parent_id } => {
                parent_of.insert(node_id, parent_id);

                if inserted.contains(&node_id) {
                    inserted.remove(&node_id);
                } else {
                    // Parent-removal dominance: skip child remove if an ancestor
                    // is already marked removed in this coalescing window.
                    if has_removed_ancestor(node_id, &removed, &parent_of) {
                        continue;
                    }
                    removed.retain(|id| !is_descendant(*id, node_id, &parent_of));
                    removed.insert(node_id);
                }

                // A remove dominates all descendant mutability and inserts.
                attr_map.retain(|(id, _), _| !is_descendant_or_self(*id, node_id, &parent_of));
                text_map.retain(|id, _| !is_descendant_or_self(*id, node_id, &parent_of));
                inserted.retain(|id| !is_descendant_or_self(*id, node_id, &parent_of));
            }
            RawMutation::AttrChanged {
                node_id,
                attr,
                old_val,
                new_val,
            } => {
                if removed.contains(&node_id) || has_removed_ancestor(node_id, &removed, &parent_of)
                {
                    continue;
                }
                let key = (node_id, attr.clone());

                // Rule 1: A -> B -> A
                if let Some(RawMutation::AttrChanged {
                    old_val: orig_old, ..
                }) = attr_map.get(&key)
                {
                    if *orig_old == new_val {
                        attr_map.remove(&key);
                        continue;
                    }
                }
                // Rule 2: Keep last value
                attr_map.insert(
                    key,
                    RawMutation::AttrChanged {
                        node_id,
                        attr,
                        old_val,
                        new_val,
                    },
                );
            }
            RawMutation::TextChanged { node_id, new_text } => {
                if removed.contains(&node_id) || has_removed_ancestor(node_id, &removed, &parent_of)
                {
                    continue;
                }
                text_map.insert(node_id, new_text);
            }
            RawMutation::ScrollChanged { x, y } => {
                last_scroll = Some((x, y));
            }
        }
    }

    // Process Removes (deterministic order)
    let mut removed_ids: Vec<_> = removed.into_iter().collect();
    removed_ids.sort_by_key(|id| id.to_string());
    for id in removed_ids {
        final_muts.push(CctDelta::Remove(id));
    }

    // Process Inserts (deterministic order)
    let mut inserted_ids: Vec<_> = inserted.into_iter().collect();
    inserted_ids.sort_by_key(|id| id.to_string());
    for id in inserted_ids {
        final_muts.push(CctDelta::Add(id));
    }

    // Process Updates
    let mut updated_nodes = HashSet::new();
    for &(node_id, _) in attr_map.keys() {
        updated_nodes.insert(node_id);
    }
    for &node_id in text_map.keys() {
        updated_nodes.insert(node_id);
    }

    let mut updated_ids: Vec<_> = updated_nodes.into_iter().collect();
    updated_ids.sort_by_key(|id| id.to_string());
    for id in updated_ids {
        final_muts.push(CctDelta::Update {
            node_id: id,
            display: None,
            bounds: None,
        });
    }

    if let Some((x, y)) = last_scroll {
        final_muts.push(CctDelta::Scroll { x, y });
    }

    final_muts
}

fn has_removed_ancestor(
    node_id: NodeId,
    removed: &HashSet<NodeId>,
    parent_of: &HashMap<NodeId, NodeId>,
) -> bool {
    let mut cur = node_id;
    let mut depth = 0usize;
    while let Some(parent) = parent_of.get(&cur).copied() {
        if removed.contains(&parent) {
            return true;
        }
        if parent == cur {
            break;
        }
        cur = parent;
        depth += 1;
        if depth > 2048 {
            break;
        }
    }
    false
}

fn is_descendant(
    node_id: NodeId,
    ancestor_id: NodeId,
    parent_of: &HashMap<NodeId, NodeId>,
) -> bool {
    let mut cur = node_id;
    let mut depth = 0usize;
    while let Some(parent) = parent_of.get(&cur).copied() {
        if parent == ancestor_id {
            return true;
        }
        if parent == cur {
            break;
        }
        cur = parent;
        depth += 1;
        if depth > 2048 {
            break;
        }
    }
    false
}

fn is_descendant_or_self(
    node_id: NodeId,
    ancestor_id: NodeId,
    parent_of: &HashMap<NodeId, NodeId>,
) -> bool {
    node_id == ancestor_id || is_descendant(node_id, ancestor_id, parent_of)
}
