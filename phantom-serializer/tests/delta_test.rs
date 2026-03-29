#[cfg(test)]
mod tests {
    use phantom_serializer::delta::DeltaEngine;

    #[test]
    fn test_coalesce_empty() {
        let mut engine = DeltaEngine::new();
        let result = engine.coalesce();
        assert!(result.is_empty());
    }

    #[test]
    fn test_scroll_delta_format() {
        use phantom_serializer::cct_types::CctDelta;
        let delta = CctDelta::Scroll { x: 0.0, y: 840.0 };
        let s = delta.to_string();
        assert_eq!(s, "##SCROLL 0,840");
    }

    #[test]
    fn test_add_delta_format() {
        use phantom_serializer::cct_types::CctDelta;
        use phantom_serializer::cct_types::*;
        let node = CctNode {
            node_id: "n_50".to_string(),
            element_type: ElementType::Btn,
            aria_role: CctAriaRole::Btn,
            x: 10.0, y: 10.0, w: 100.0, h: 30.0,
            bounds_confidence: BoundsConfidence::Reliable,
            display: CctDisplay::B,
            visibility: CctVisibility::V,
            opacity: 1.0,
            pointer_events: CctPointerEvents::A,
            accessible_name: "-".to_string(),
            visible_text: "-".to_string(),
            events: CctEvents::empty(),
            parent_id: "n_root".to_string(),
            flags: 0,
            state: CctState::empty(),
            id_confidence: IdConfidence::High,
            relevance: None,
        };
        let delta = CctDelta::Add(node);
        assert!(delta.to_string().starts_with("+ n_50|btn|btn|"));
    }
}
