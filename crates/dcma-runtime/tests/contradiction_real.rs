//! Test 4: Contradiction Detection
//! Feeds deliberately contradictory knowledge and verifies detection.

use dcma_core::atom::CognitiveAtom;
use dcma_core::state::CognitiveState;
use dcma_core::types::*;
use dcma_runtime::{Runtime, Stimulus};

fn add_concept(state: &mut CognitiveState, name: &str) -> AtomId {
    let id = AtomId::new();
    let atom = CognitiveAtom::new(id, AtomType::Concept, name.to_string());
    state.graph.add_atom(atom);
    id
}

#[test]
fn test_explicit_contradiction_detected() {
    let config = EngineConfig::default();
    let mut state = CognitiveState::new(config.clone());

    let a = add_concept(&mut state, "Theory A");
    let b = add_concept(&mut state, "Theory B");
    state.graph.add_relation(a, b, RelationType::Contradicts).unwrap();

    let mut runtime = Runtime::with_state(config, state);
    let stimulus = Stimulus {
        source_nodes: vec![a, b],
        query_context: vec![],
        label: "explicit".to_string(),
    };

    for _ in 0..20 {
        runtime.tick(&stimulus);
    }

    let contradictions = &runtime.state().contradictions;
    assert!(!contradictions.is_empty(), "Explicit Contradicts edge must be detected");
    println!("Explicit contradiction: {} found", contradictions.len());
}

#[test]
fn test_no_false_positives_consistent_knowledge() {
    let config = EngineConfig::default();
    let mut state = CognitiveState::new(config.clone());

    let mammals = add_concept(&mut state, "Mammals");
    let dogs = add_concept(&mut state, "Dogs");
    let cats = add_concept(&mut state, "Cats");
    let animals = add_concept(&mut state, "Animals");

    state.graph.add_relation(dogs, mammals, RelationType::InstanceOf).unwrap();
    state.graph.add_relation(cats, mammals, RelationType::InstanceOf).unwrap();
    state.graph.add_relation(mammals, animals, RelationType::InstanceOf).unwrap();

    let mut runtime = Runtime::with_state(config, state);
    let stimulus = Stimulus {
        source_nodes: vec![animals],
        query_context: vec![],
        label: "no_false_pos".to_string(),
    };

    for _ in 0..50 {
        runtime.tick(&stimulus);
    }

    let contradictions = &runtime.state().contradictions;
    assert!(contradictions.len() <= 1, "Consistent hierarchy should produce 0 or minimal contradictions, got {}", contradictions.len());
    println!("False positive check: {} contradictions", contradictions.len());
}

#[test]
fn test_multiple_contradictions() {
    let config = EngineConfig::default();
    let mut state = CognitiveState::new(config.clone());

    let a = add_concept(&mut state, "Theory A");
    let b = add_concept(&mut state, "Theory B");
    let c = add_concept(&mut state, "Theory C");
    let d = add_concept(&mut state, "Theory D");

    state.graph.add_relation(a, b, RelationType::Contradicts).unwrap();
    state.graph.add_relation(c, d, RelationType::Contradicts).unwrap();
    state.graph.add_relation(a, c, RelationType::Contradicts).unwrap();

    let mut runtime = Runtime::with_state(config, state);
    let stimulus = Stimulus {
        source_nodes: vec![a, b, c, d],
        query_context: vec![],
        label: "multi".to_string(),
    };

    for _ in 0..30 {
        runtime.tick(&stimulus);
    }

    let contradictions = &runtime.state().contradictions;
    assert!(contradictions.len() >= 3, "Should detect >= 3 contradictions, got {}", contradictions.len());
    println!("Multiple contradictions: {} found", contradictions.len());
}

#[test]
fn test_contradiction_resolution_creates_asymmetry() {
    let config = EngineConfig::default();
    let mut state = CognitiveState::new(config.clone());

    let thesis = add_concept(&mut state, "Thesis X");
    let antithesis = add_concept(&mut state, "Anti-Thesis X");
    state.graph.add_relation(thesis, antithesis, RelationType::Contradicts).unwrap();

    // Both start with equal activation
    state.set_activation(thesis, Activation::new(rust_decimal::Decimal::from_str_exact("0.8").unwrap()));
    state.set_activation(antithesis, Activation::new(rust_decimal::Decimal::from_str_exact("0.8").unwrap()));

    let mut runtime = Runtime::with_state(config, state);
    let stimulus = Stimulus {
        source_nodes: vec![thesis],
        query_context: vec![],
        label: "resolution".to_string(),
    };

    for _ in 0..100 {
        runtime.tick(&stimulus);
    }

    let act_thesis = runtime.state().get_activation(thesis).unwrap();
    let act_antithesis = runtime.state().get_activation(antithesis).unwrap();
    let diff = (act_thesis.get() - act_antithesis.get()).abs();

    println!("After resolution: thesis={}, antithesis={}, diff={}", act_thesis.get(), act_antithesis.get(), diff);
    assert!(diff > rust_decimal::Decimal::ZERO, "Resolution should create asymmetry");
}

#[test]
fn test_stable_complementary_knowledge() {
    let config = EngineConfig::default();
    let mut state = CognitiveState::new(config.clone());

    let physics = add_concept(&mut state, "Physics");
    let mechanics = add_concept(&mut state, "Classical Mechanics");
    let quantum = add_concept(&mut state, "Quantum Mechanics");
    let relativity = add_concept(&mut state, "Relativity");

    state.graph.add_relation(mechanics, physics, RelationType::InstanceOf).unwrap();
    state.graph.add_relation(quantum, physics, RelationType::InstanceOf).unwrap();
    state.graph.add_relation(relativity, physics, RelationType::InstanceOf).unwrap();
    state.graph.add_relation(mechanics, quantum, RelationType::Supports).unwrap();
    state.graph.add_relation(quantum, relativity, RelationType::Supports).unwrap();

    let mut runtime = Runtime::with_state(config, state);
    let stimulus = Stimulus {
        source_nodes: vec![physics],
        query_context: vec![],
        label: "stable".to_string(),
    };

    for _ in 0..100 {
        runtime.tick(&stimulus);
    }

    let contradictions = &runtime.state().contradictions;
    assert!(contradictions.is_empty(), "Complementary knowledge should produce 0 contradictions, got {}", contradictions.len());
    println!("Stable knowledge: {} contradictions", contradictions.len());
}

#[test]
fn test_supervised_unsupervised_mutually_exclusive() {
    let config = EngineConfig::default();
    let mut state = CognitiveState::new(config.clone());

    let supervised = add_concept(&mut state, "Supervised Learning");
    let unsupervised = add_concept(&mut state, "Unsupervised Learning");

    state.graph.add_relation(supervised, unsupervised, RelationType::Contradicts).unwrap();

    let mut runtime = Runtime::with_state(config, state);
    let stimulus = Stimulus {
        source_nodes: vec![supervised, unsupervised],
        query_context: vec![],
        label: "exclusive".to_string(),
    };

    for _ in 0..50 {
        runtime.tick(&stimulus);
    }

    let contradictions = &runtime.state().contradictions;
    assert!(!contradictions.is_empty(), "Supervised vs Unsupervised should be flagged as contradictory");
    println!("Mutually exclusive detection: {} contradictions", contradictions.len());
}
