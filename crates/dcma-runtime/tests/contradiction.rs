use dcma_core::atom::CognitiveAtom;
use dcma_core::state::CognitiveState;
use dcma_core::types::*;
use dcma_runtime::{Runtime, Stimulus};
use dcma_topology::edge::clear_weight_history;

fn make_stimulus(source_nodes: Vec<AtomId>) -> Stimulus {
    Stimulus {
        source_nodes,
        query_context: vec![],
        label: "test".to_string(),
    }
}

#[test]
fn test_contradiction_pressure_updates() {
    clear_weight_history();

    let config = EngineConfig::default();
    let mut state = CognitiveState::new(config.clone());

    let a = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "belief_a".to_string());
    let b = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "belief_b".to_string());
    let id_a = a.id;
    let id_b = b.id;
    state.graph.add_atom(a);
    state.graph.add_atom(b);
    state
        .graph
        .add_relation(id_a, id_b, RelationType::Contradicts)
        .unwrap();

    let mut runtime = Runtime::with_state(config, state);
    let stimulus = make_stimulus(vec![id_a]);
    runtime.tick(&stimulus);

    let pressure = runtime.state().get_contradiction(id_a, id_b);
    assert!(
        pressure.is_some(),
        "Contradiction pressure should exist after tick"
    );
    if let Some(p) = pressure {
        assert!(
            p.get() >= rust_decimal::Decimal::ZERO,
            "Pressure should be non-negative"
        );
    }
}

#[test]
fn test_contradiction_equilibrium() {
    clear_weight_history();

    let config = EngineConfig::default();
    let mut state = CognitiveState::new(config.clone());

    let a = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "a".to_string());
    let b = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "b".to_string());
    let id_a = a.id;
    let id_b = b.id;
    state.graph.add_atom(a);
    state.graph.add_atom(b);
    state
        .graph
        .add_relation(id_a, id_b, RelationType::Contradicts)
        .unwrap();

    let mut runtime = Runtime::with_state(config, state);
    // Stimulate only node a repeatedly so it dominates
    let stimulus = make_stimulus(vec![id_a]);
    for _ in 0..10 {
        runtime.tick(&stimulus);
    }

    let pressure = runtime.state().get_contradiction(id_a, id_b);
    assert!(
        pressure.is_some(),
        "Contradiction should be tracked after multiple ticks"
    );
}
