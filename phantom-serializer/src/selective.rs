use phantom_core::dom::{DomNode, NodeData};
use crate::semantic::SemanticInfo;

pub fn compute_relevance(
    node: &DomNode,
    semantic: &SemanticInfo,
    task_hint: &str,
) -> f32 {
    let mut score: f32 = 0.0;
    let hint_lower = task_hint.to_lowercase();
    
    if !hint_lower.is_empty() {
        let text_lower = semantic.visible_text.to_lowercase();
        if hint_lower.split_whitespace().any(|word| text_lower.contains(word) && word.len() > 2) {
            score += 0.3;
        }

        let label_lower = semantic.accessible_name.to_lowercase();
        if hint_lower.split_whitespace().any(|word| label_lower.contains(word) && word.len() > 2) {
            score += 0.3;
        }
    }

    let is_login_task = hint_lower.contains("login") || hint_lower.contains("sign in") || hint_lower.contains("password");
    if is_login_task {
        if let NodeData::Element { tag_name, .. } = &node.data {
            let tag = tag_name.to_lowercase();
            if tag == "input" || tag == "button" || tag == "form" || tag == "a" {
                score += 0.2;
            }
        }
    }
    
    score.min(1.0)
}

pub fn should_include_in_selective(
    _node: &DomNode,
    relevance: f32,
    is_interactive: bool,
    is_landmark: bool,
) -> bool {
    if is_interactive || is_landmark {
        return true;
    }
    relevance > 0.4
}
