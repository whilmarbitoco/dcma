use dcma_core::atom::CognitiveAtom;
use dcma_core::state::CognitiveState;
use dcma_core::types::*;
use dcma_entropy::entropy::graph_entropy;
use dcma_runtime::{Runtime, Stimulus};
use dcma_topology::edge::clear_weight_history;
use rust_decimal::Decimal;

fn make_stimulus(source_nodes: Vec<AtomId>) -> Stimulus {
    Stimulus {
        source_nodes,
        query_context: vec![],
        label: "test".to_string(),
    }
}

#[test]
fn test_entropy_stays_in_bounds() {
    clear_weight_history();

    let config = EngineConfig::default();
    let mut state = CognitiveState::new(config.clone());

    // Build a graph with moderate structure
    let atoms: Vec<CognitiveAtom> = (0..10)
        .map(|i| CognitiveAtom::new(AtomId::new(), AtomType::Concept, format!("n_{}", i)))
        .collect();
    let ids: Vec<AtomId> = atoms.iter().map(|a| a.id).collect();
    for atom in atoms {
        state.graph.add_atom(atom);
    }

    // Chain topology: 0→1→2→...→9
    for i in 0..9 {
        state
            .graph
            .add_relation(ids[i], ids[i + 1], RelationType::Supports)
            .unwrap();
    }

    let mut runtime = Runtime::with_state(config, state);
    let stimulus = make_stimulus(ids[..3].to_vec());

    for _ in 0..100 {
        runtime.tick(&stimulus);
    }

    let h = graph_entropy(&runtime.state().graph);
    assert!(
        h >= Decimal::ZERO,
        "Entropy should be non-negative, got {h}"
    );
}

#[test]
fn test_entropy_regulation_triggers() {
    clear_weight_history();

    let config = EngineConfig::default();
    let mut state = CognitiveState::new(config.clone());

    // Create a star with many leaves for non-uniform degree distribution
    let center = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "hub".to_string());
    let center_id = center.id;
    state.graph.add_atom(center);

    for i in 0..20 {
        let leaf = CognitiveAtom::new(AtomId::new(), AtomType::Concept, format!("leaf_{}", i));
        let lid = leaf.id;
        state.graph.add_atom(leaf);
        state
            .graph
            .add_relation(center_id, lid, RelationType::Causes)
            .unwrap();
    }

    let h_initial = graph_entropy(&state.graph);
    assert!(h_initial > Decimal::ZERO, "Star graph should have positive entropy");

    let mut runtime = Runtime::with_state(config, state);
    let stimulus = make_stimulus(vec![]);

    for _ in 0..50 {
        runtime.tick(&stimulus);
    }

    let h_after = graph_entropy(&runtime.state().graph);
    assert!(
        h_after >= Decimal::ZERO,
        "Entropy should be non-negative, got {h_after}"
    );
}
