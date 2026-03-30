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
        let mut arena = indextree::Arena::<()>::new();
        let node_id = arena.new_node(());
        let delta = CctDelta::Add(node_id);
        assert_eq!(delta.to_string(), format!("+ {}", node_id));
    }
}
