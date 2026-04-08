#[cfg(test)]
mod tests {
    use std::thread::sleep;
    use std::time::Duration;

    use indextree::Arena;
    use phantom_serializer::cct_types::CctDelta;
    use phantom_serializer::delta::DeltaEngine;
    use phantom_serializer::delta::RawMutation;

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

    #[test]
    fn test_parent_remove_dominates_child_remove() {
        let mut arena = Arena::<()>::new();
        let root = arena.new_node(());
        let parent = arena.new_node(());
        let child = arena.new_node(());
        root.append(parent, &mut arena);
        parent.append(child, &mut arena);

        let mut engine = DeltaEngine::new();
        engine.push(RawMutation::NodeRemoved {
            node_id: child,
            parent_id: parent,
        });
        engine.push(RawMutation::NodeRemoved {
            node_id: parent,
            parent_id: root,
        });

        sleep(Duration::from_millis(20));
        let out = engine.coalesce();

        assert_eq!(out.len(), 1, "Only parent remove should remain");
        assert!(
            matches!(out[0], CctDelta::Remove(id) if id == parent),
            "Parent removal must dominate descendant removal"
        );
    }

    #[test]
    fn test_removed_parent_prunes_child_updates() {
        let mut arena = Arena::<()>::new();
        let root = arena.new_node(());
        let parent = arena.new_node(());
        let child = arena.new_node(());
        root.append(parent, &mut arena);
        parent.append(child, &mut arena);

        let mut engine = DeltaEngine::new();
        engine.push(RawMutation::NodeRemoved {
            node_id: parent,
            parent_id: root,
        });
        engine.push(RawMutation::NodeRemoved {
            node_id: child,
            parent_id: parent,
        });
        engine.push(RawMutation::AttrChanged {
            node_id: child,
            attr: "class".to_string(),
            old_val: None,
            new_val: Some("x".to_string()),
        });
        engine.push(RawMutation::TextChanged {
            node_id: child,
            new_text: "hidden".to_string(),
        });

        sleep(Duration::from_millis(20));
        let out = engine.coalesce();

        assert_eq!(
            out.len(),
            1,
            "Child updates must be pruned by parent remove"
        );
        assert!(
            matches!(out[0], CctDelta::Remove(id) if id == parent),
            "Only parent remove should remain in final delta set"
        );
    }

    #[test]
    fn test_pending_queue_is_bounded_when_consumer_stalls() {
        let mut arena = Arena::<()>::new();
        let root = arena.new_node(());
        let mut engine = DeltaEngine::new();

        for _ in 0..5000 {
            let node = arena.new_node(());
            engine.push(RawMutation::NodeInserted {
                node_id: node,
                parent_id: root,
            });
        }

        sleep(Duration::from_millis(20));
        let out = engine.coalesce();

        assert_eq!(
            out.len(),
            4096,
            "pending mutation queue must stay bounded under stalled consumer"
        );
    }

    #[test]
    fn test_attr_round_trip_over_multiple_steps_is_noop() {
        let mut arena = Arena::<()>::new();
        let node = arena.new_node(());
        let mut engine = DeltaEngine::new();

        engine.push(RawMutation::AttrChanged {
            node_id: node,
            attr: "class".to_string(),
            old_val: None,
            new_val: Some("active".to_string()),
        });
        engine.push(RawMutation::AttrChanged {
            node_id: node,
            attr: "class".to_string(),
            old_val: Some("active".to_string()),
            new_val: Some("active loading".to_string()),
        });
        engine.push(RawMutation::AttrChanged {
            node_id: node,
            attr: "class".to_string(),
            old_val: Some("active loading".to_string()),
            new_val: None,
        });

        sleep(Duration::from_millis(20));
        let out = engine.coalesce();

        assert!(
            out.is_empty(),
            "A -> B -> C -> A attribute transitions must coalesce to no-op"
        );
    }
}
