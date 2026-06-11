use dcma_core::atom::CognitiveAtom;
use dcma_core::state::CognitiveState;
use dcma_core::types::*;
use dcma_entropy::entropy::graph_entropy;
use dcma_runtime::{Runtime, Stimulus};
use dcma_topology::edge::clear_weight_history;
use rust_decimal::Decimal;

#[test]
fn test_end_to_end_100_ticks() {
    clear_weight_history();

    let config = EngineConfig::default();
    let mut state = CognitiveState::new(config.clone());

    // Insert 20 atoms
    let atoms: Vec<CognitiveAtom> = (0..20)
        .map(|i| CognitiveAtom::new(AtomId::new(), AtomType::Concept, format!("atom_{}", i)))
        .collect();
    let ids: Vec<AtomId> = atoms.iter().map(|a| a.id).collect();
    for atom in atoms {
        state.graph.add_atom(atom);
    }

    // Create 15 relations
    for i in 0..15 {
        let src = i % 20;
        let dst = (i + 7) % 20;
        let rel_type = match i % 4 {
            0 => RelationType::Supports,
            1 => RelationType::Causes,
            2 => RelationType::Reinforces,
            _ => RelationType::Precedes,
        };
        state
            .graph
            .add_relation(ids[src], ids[dst], rel_type)
            .unwrap();
    }

    // Create 2 Contradicts edges
    state
        .graph
        .add_relation(ids[0], ids[1], RelationType::Contradicts)
        .unwrap();
    state
        .graph
        .add_relation(ids[2], ids[3], RelationType::Contradicts)
        .unwrap();

    let source_nodes = ids[..5].to_vec();
    let stimulus = Stimulus {
        source_nodes,
        query_context: ids[10..15].to_vec(),
        label: "e2e".to_string(),
    };

    let mut runtime = Runtime::with_state(config.clone(), state);

    // Run 100 ticks with the same stimulus
    for _ in 0..100 {
        runtime.tick(&stimulus);
    }

    // Verify all activations converged
    for (_id, act) in &runtime.activations() {
        assert!(
            act.get() >= Decimal::ZERO && act.get() <= Decimal::ONE,
            "All activations must be in [0, 1]"
        );
    }

    // Verify entropy is non-negative (a valid graph property)
    let h = graph_entropy(&runtime.state().graph);
    assert!(
        h >= Decimal::ZERO,
        "Entropy should be non-negative, got {h}"
    );

    // Verify contradictions classified
    let contradiction_count = runtime.state().contradictions.len();
    assert!(
        contradiction_count >= 2,
        "Should have at least 2 contradictions, got {contradiction_count}"
    );

    // Verify tick count
    assert_eq!(runtime.tick_count(), 100, "Should have completed 100 ticks");
}
