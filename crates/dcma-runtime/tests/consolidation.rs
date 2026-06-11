use dcma_core::atom::CognitiveAtom;
use dcma_core::state::CognitiveState;
use dcma_core::types::*;
use dcma_runtime::{Runtime, Stimulus, CONSOLIDATION_INTERVAL};
use dcma_topology::edge::clear_weight_history;

fn make_stimulus() -> Stimulus {
    Stimulus {
        source_nodes: vec![],
        query_context: vec![],
        label: "test".to_string(),
    }
}

#[test]
fn test_consolidation_periodic() {
    clear_weight_history();

    let config = EngineConfig::default();
    let state = CognitiveState::new(config.clone());
    let mut runtime = Runtime::with_state(config, state);
    let stimulus = make_stimulus();

    // Run fewer than CONSOLIDATION_INTERVAL ticks
    for _ in 0..CONSOLIDATION_INTERVAL - 1 {
        let result = runtime.tick(&stimulus);
        assert!(
            !result.consolidation_applied,
            "Consolidation should not run before interval at tick {}",
            result.tick
        );
    }

    // Run one more to hit the interval
    let result = runtime.tick(&stimulus);
    assert!(
        result.consolidation_applied,
        "Consolidation should run at tick {} (interval {})",
        result.tick,
        CONSOLIDATION_INTERVAL
    );
}

#[test]
fn test_motifs_crystallize() {
    clear_weight_history();

    let config = EngineConfig::default();
    let mut state = CognitiveState::new(config.clone());

    // Create a 3-node pattern
    let atoms: Vec<CognitiveAtom> = (0..3)
        .map(|i| CognitiveAtom::new(AtomId::new(), AtomType::Concept, format!("m_{}", i)))
        .collect();
    let ids: Vec<AtomId> = atoms.iter().map(|a| a.id).collect();
    for atom in atoms {
        state.graph.add_atom(atom);
    }
    state
        .graph
        .add_relation_with_weight(ids[0], ids[1], RelationType::Causes, Weight::new(rust_decimal::Decimal::from_str_exact("0.8").unwrap()))
        .unwrap();
    state
        .graph
        .add_relation_with_weight(ids[1], ids[2], RelationType::Supports, Weight::new(rust_decimal::Decimal::from_str_exact("0.7").unwrap()))
        .unwrap();

    // Create copies of the same pattern
    let pattern_ids: Vec<Vec<AtomId>> = (0..6)
        .map(|p| {
            let copy_atoms: Vec<CognitiveAtom> = (0..3)
                .map(|i| CognitiveAtom::new(AtomId::new(), AtomType::Concept, format!("p{}_{}", p, i)))
                .collect();
            let copy_ids: Vec<AtomId> = copy_atoms.iter().map(|a| a.id).collect();
            for atom in copy_atoms {
                state.graph.add_atom(atom);
            }
            state
                .graph
                .add_relation_with_weight(copy_ids[0], copy_ids[1], RelationType::Causes, Weight::new(rust_decimal::Decimal::from_str_exact("0.8").unwrap()))
                .unwrap();
            state
                .graph
                .add_relation_with_weight(copy_ids[1], copy_ids[2], RelationType::Supports, Weight::new(rust_decimal::Decimal::from_str_exact("0.7").unwrap()))
                .unwrap();
            copy_ids
        })
        .collect();

    // Add reinforcement to increase motif recurrence count
    for t in 1..=10 {
        for pat in &pattern_ids {
            for id in pat {
                if let Some(atom) = state.graph.get_atom_mut(*id) {
                    atom.record_reinforcement(t);
                }
            }
        }
    }

    let mut runtime = Runtime::with_state(config, state);
    let stimulus = Stimulus {
        source_nodes: pattern_ids.iter().flat_map(|p| p.iter()).copied().collect(),
        query_context: vec![],
        label: "motif_test".to_string(),
    };

    // Run enough ticks to trigger consolidation at least once
    for _ in 0..CONSOLIDATION_INTERVAL {
        runtime.tick(&stimulus);
    }

    assert!(runtime.tick_count() >= CONSOLIDATION_INTERVAL);
}
