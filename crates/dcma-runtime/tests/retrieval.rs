use dcma_core::atom::CognitiveAtom;
use dcma_core::state::CognitiveState;
use dcma_core::types::*;
use dcma_retrieval::retrieve;
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
fn test_retrieval_after_tick() {
    clear_weight_history();

    let config = EngineConfig::default();
    let mut state = CognitiveState::new(config.clone());

    let atoms: Vec<CognitiveAtom> = (0..8)
        .map(|i| CognitiveAtom::new(AtomId::new(), AtomType::Concept, format!("r_{}", i)))
        .collect();
    let ids: Vec<AtomId> = atoms.iter().map(|a| a.id).collect();
    for atom in atoms {
        state.graph.add_atom(atom);
    }

    for i in 0..4 {
        state
            .graph
            .add_relation(ids[i], ids[i + 4], RelationType::Supports)
            .unwrap();
    }

    let mut runtime = Runtime::with_state(config, state);
    let stimulus = make_stimulus(ids[..2].to_vec());
    runtime.tick(&stimulus);

    let field = retrieve(
        &runtime.state().graph,
        &[ids[4]],
        &runtime.activations(),
        runtime.tick_count(),
        &runtime.state().config,
    );

    assert!(
        field.nodes.len() <= runtime.state().graph.atom_count(),
        "Retrieval should not return more nodes than exist"
    );
}

#[test]
fn test_retrieval_empty_state() {
    clear_weight_history();

    let config = EngineConfig::default();
    let state = CognitiveState::new(config.clone());
    let mut runtime = Runtime::with_state(config, state);

    let stimulus = make_stimulus(vec![]);
    runtime.tick(&stimulus);

    let field = retrieve(
        &runtime.state().graph,
        &[],
        &runtime.activations(),
        runtime.tick_count(),
        &runtime.state().config,
    );

    assert!(
        field.nodes.is_empty(),
        "Retrieval on empty graph should return empty field"
    );
}
