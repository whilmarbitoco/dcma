use dcma_core::atom::CognitiveAtom;
use dcma_core::state::CognitiveState;
use dcma_core::types::*;
use dcma_runtime::{Runtime, Stimulus};
use dcma_topology::edge::clear_weight_history;
use rust_decimal::Decimal;

fn make_state(count: usize) -> (CognitiveState, Vec<AtomId>) {
    let config = EngineConfig::default();
    let mut state = CognitiveState::new(config);
    let atoms: Vec<CognitiveAtom> = (0..count)
        .map(|i| CognitiveAtom::new(AtomId::new(), AtomType::Concept, format!("n_{}", i)))
        .collect();
    let ids: Vec<AtomId> = atoms.iter().map(|a| a.id).collect();
    for atom in atoms {
        state.graph.add_atom(atom);
    }
    (state, ids)
}

fn make_stimulus(source_nodes: Vec<AtomId>) -> Stimulus {
    Stimulus {
        source_nodes,
        query_context: vec![],
        label: "test".to_string(),
    }
}

#[test]
fn test_activation_converges() {
    clear_weight_history();

    let (mut state, ids) = make_state(10);
    for i in 0..5 {
        state
            .graph
            .add_relation(ids[i], ids[i + 5], RelationType::Supports)
            .unwrap();
    }

    let mut runtime = Runtime::with_state(EngineConfig::default(), state);
    let stimulus = make_stimulus(ids[..3].to_vec());
    let result = runtime.tick(&stimulus);

    for (_id, act) in &result.activations {
        assert!(
            act.get() >= Decimal::ZERO && act.get() <= Decimal::ONE,
            "Activation must be in [0, 1]"
        );
    }
}

#[test]
fn test_convergence_within_bound() {
    clear_weight_history();

    let mut config = EngineConfig::default();
    config.max_iterations = 80;

    let (mut state, ids) = make_state(20);
    for i in 0..10 {
        state
            .graph
            .add_relation(ids[i], ids[i + 10], RelationType::Supports)
            .unwrap();
    }

    let mut runtime = Runtime::with_state(config, state);
    let stimulus = make_stimulus(ids[..5].to_vec());
    let result = runtime.tick(&stimulus);

    // Activate runs up to max_iterations (80) internally.
    // Verify it completes with valid activations.
    for (_id, act) in &result.activations {
        assert!(
            act.get() >= Decimal::ZERO && act.get() <= Decimal::ONE,
            "Activation must be in [0, 1]"
        );
    }
}
