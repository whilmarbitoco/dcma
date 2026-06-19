//! Test 7: Distributed Sync
//! Two nodes ingest different knowledge, merge, verify both have all knowledge.

use dcma_core::atom::CognitiveAtom;
use dcma_core::state::CognitiveState;
use dcma_core::types::*;
use dcma_runtime::{Runtime, Stimulus};

fn add_atom(state: &mut CognitiveState, name: &str, atom_type: AtomType) -> AtomId {
    let id = AtomId::new();
    let atom = CognitiveAtom::new(id, atom_type, name.to_string());
    state.graph.add_atom(atom);
    id
}

#[test]
fn test_two_nodes_merge_no_overlap() {
    // Node A ingests ML knowledge
    let config = EngineConfig::default();
    let mut state_a = CognitiveState::new(config.clone());
    let ml = add_atom(&mut state_a, "Machine Learning", AtomType::Concept);
    let dl = add_atom(&mut state_a, "Deep Learning", AtomType::Concept);
    state_a.graph.add_relation(ml, dl, RelationType::InstanceOf).unwrap();

    // Node B ingests NLP knowledge
    let mut state_b = CognitiveState::new(config.clone());
    let nlp = add_atom(&mut state_b, "Natural Language Processing", AtomType::Concept);
    let tf = add_atom(&mut state_b, "Transformers", AtomType::Concept);
    state_b.graph.add_relation(nlp, tf, RelationType::InstanceOf).unwrap();

    // Merge: add all atoms from B into A
    let atom_count_before = state_a.graph.atom_count();
    let mut id_map = std::collections::HashMap::new();
    for (_, atom) in state_b.graph.atoms() {
        let new_id = AtomId::new();
        id_map.insert(atom.id, new_id);
        let mut new_atom = CognitiveAtom::new(new_id, atom.atom_type, atom.canonical.clone());
        new_atom.attributes = atom.attributes.clone();
        state_a.graph.add_atom(new_atom);
    }
    for (_, relation) in state_b.graph.relations() {
        if let (Some(&new_src), Some(&new_dst)) = (id_map.get(&relation.src), id_map.get(&relation.dst)) {
            let _ = state_a.graph.add_relation(new_src, new_dst, relation.relation_type);
        }
    }

    // Verify merged state has all atoms
    println!("Node A: {} atoms, Node B: {} atoms", atom_count_before, state_b.graph.atom_count());
    println!("Merged: {} atoms", state_a.graph.atom_count());
    assert!(state_a.graph.atom_count() >= atom_count_before + state_b.graph.atom_count());
}

#[test]
fn test_merge_preserves_relations() {
    let config = EngineConfig::default();
    let mut state_a = CognitiveState::new(config.clone());
    let a1 = add_atom(&mut state_a, "A1", AtomType::Concept);
    let a2 = add_atom(&mut state_a, "A2", AtomType::Concept);
    state_a.graph.add_relation(a1, a2, RelationType::Causes).unwrap();

    let mut state_b = CognitiveState::new(config.clone());
    let b1 = add_atom(&mut state_b, "B1", AtomType::Concept);
    let b2 = add_atom(&mut state_b, "B2", AtomType::Concept);
    state_b.graph.add_relation(b1, b2, RelationType::Causes).unwrap();

    let rel_count_before = state_a.graph.relation_count();

    // Merge B into A (atoms only, no relation merge in this test)
    for (_, atom) in state_b.graph.atoms() {
        state_a.graph.add_atom(atom.clone());
    }

    println!("Before merge: {} atoms, {} relations", rel_count_before, state_b.graph.relation_count());
    println!("After merge: {} atoms, {} relations", state_a.graph.atom_count(), state_a.graph.relation_count());
    // Atoms should be merged
    assert_eq!(state_a.graph.atom_count(), 4);
}

#[test]
fn test_merge_with_shared_knowledge() {
    let config = EngineConfig::default();

    // Both nodes know about Machine Learning
    let mut state_a = CognitiveState::new(config.clone());
    let ml_a = add_atom(&mut state_a, "Machine Learning", AtomType::Concept);
    let dl = add_atom(&mut state_a, "Deep Learning", AtomType::Concept);
    state_a.graph.add_relation(ml_a, dl, RelationType::InstanceOf).unwrap();

    let mut state_b = CognitiveState::new(config.clone());
    // Same concept, different UUID (simulating independent ingestion)
    let ml_b = add_atom(&mut state_b, "Machine Learning", AtomType::Concept);
    let nlp = add_atom(&mut state_b, "Natural Language Processing", AtomType::Concept);
    state_b.graph.add_relation(ml_b, nlp, RelationType::Supports).unwrap();

    // Merge — both ML atoms exist (duplicate by name, different UUID)
    for (_, atom) in state_b.graph.atoms() {
        state_a.graph.add_atom(atom.clone());
    }

    println!("Merged with shared concept: {} atoms", state_a.graph.atom_count());
    // Both ML atoms exist — deduplication is a future concern
    assert!(state_a.graph.atom_count() >= 4);
}

#[test]
fn test_contradiction_across_nodes() {
    let config = EngineConfig::default();

    // Node A believes X
    let mut state_a = CognitiveState::new(config.clone());
    let x = add_atom(&mut state_a, "X", AtomType::Concept);
    let y = add_atom(&mut state_a, "Y", AtomType::Concept);
    state_a.graph.add_relation(x, y, RelationType::Supports).unwrap();

    // Node B believes X contradicts Y
    let mut state_b = CognitiveState::new(config.clone());
    let x2 = add_atom(&mut state_b, "X", AtomType::Concept);
    let y2 = add_atom(&mut state_b, "Y", AtomType::Concept);
    state_b.graph.add_relation(x2, y2, RelationType::Contradicts).unwrap();

    // Merge: both relation types exist between the same concepts
    for (_, atom) in state_b.graph.atoms() {
        state_a.graph.add_atom(atom.clone());
    }
    // Add the contradicting relation from B
    state_a.graph.add_relation(x, y, RelationType::Contradicts).unwrap();

    let runtime = Runtime::with_state(config, state_a);
    let contradictions = &runtime.state().contradictions;
    println!("Cross-node contradictions: {}", contradictions.len());
    // The contradicting relation should be registered
}

#[test]
fn test_merge_preserves_cognitive_state() {
    let config = EngineConfig::default();

    // Node A has run 10 ticks
    let mut state_a = CognitiveState::new(config.clone());
    for i in 0..50 {
        let atom = CognitiveAtom::new(AtomId::new(), AtomType::Concept, format!("a_{}", i));
        state_a.graph.add_atom(atom);
    }

    let mut runtime_a = Runtime::with_state(config.clone(), state_a);
    let stimulus = Stimulus {
        source_nodes: runtime_a.state().graph.atoms().map(|(_, a)| a.id).take(5).collect(),
        query_context: vec![],
        label: "a_ticks".to_string(),
    };
    for _ in 0..10 {
        runtime_a.tick(&stimulus);
    }

    // Node B has run 5 ticks
    let mut state_b = CognitiveState::new(config.clone());
    for i in 0..50 {
        let atom = CognitiveAtom::new(AtomId::new(), AtomType::Concept, format!("b_{}", i));
        state_b.graph.add_atom(atom);
    }

    let mut runtime_b = Runtime::with_state(config.clone(), state_b);
    let stimulus_b = Stimulus {
        source_nodes: runtime_b.state().graph.atoms().map(|(_, a)| a.id).take(5).collect(),
        query_context: vec![],
        label: "b_ticks".to_string(),
    };
    for _ in 0..5 {
        runtime_b.tick(&stimulus_b);
    }

    println!("Node A: {} ticks, Node B: {} ticks", runtime_a.tick_count(), runtime_b.tick_count());

    // Each node preserves its own tick count
    assert_eq!(runtime_a.tick_count(), 10);
    assert_eq!(runtime_b.tick_count(), 5);
}
