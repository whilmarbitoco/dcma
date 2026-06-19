//! Test 6: Persistence Roundtrip
//! Ingest -> save -> restart -> load -> query. Verifies integrity.

use dcma_core::atom::CognitiveAtom;
use dcma_core::state::CognitiveState;
use dcma_core::types::*;
use dcma_runtime::{Runtime, Stimulus};
use dcma_storage::Storage;
use std::time::Instant;

fn make_state_with_knowledge() -> (CognitiveState, Vec<AtomId>) {
    let config = EngineConfig::default();
    let mut state = CognitiveState::new(config);

    let engine = dcma_ingest::IngestEngine::new();
    let text = "Machine learning is a subset of artificial intelligence. \
                Deep learning is a subset of machine learning. \
                Neural networks are the foundation of deep learning. \
                Backpropagation is used to train neural networks. \
                Gradient descent is an optimization algorithm. \
                Convolutional neural networks are used for image recognition. \
                Recurrent neural networks are used for sequence data. \
                Transformers are a type of neural network architecture. \
                Natural language processing uses transformers. \
                Reinforcement learning trains agents through rewards.";

    let entities = engine.extract_entities(text);
    let mut ids = Vec::new();
    for entity in &entities {
        let atom_type = match entity.entity_type.as_str() {
            "Entity" => AtomType::Entity,
            _ => AtomType::Concept,
        };
        let id = AtomId::new();
        let atom = CognitiveAtom::new(id, atom_type, entity.name.clone());
        state.graph.add_atom(atom);
        ids.push(id);
    }

    // Add some relations
    for i in 0..ids.len().saturating_sub(1) {
        let _ = state.graph.add_relation(ids[i], ids[i + 1], RelationType::Supports);
    }

    (state, ids)
}

#[test]
fn test_file_save_load_preserves_atoms() {
    let (state, ids) = make_state_with_knowledge();
    let atom_count = state.graph.atom_count();
    let relation_count = state.graph.relation_count();

    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("state.json");

    // Save
    let start = Instant::now();
    Storage::save_state(&state, &path).unwrap();
    let save_time = start.elapsed();

    // Load
    let start = Instant::now();
    let loaded = Storage::load_state(&path).unwrap();
    let load_time = start.elapsed();

    println!("File roundtrip: {} atoms, {} relations", atom_count, relation_count);
    println!("  Save: {:?}  Load: {:?}", save_time, load_time);

    assert_eq!(loaded.graph.atom_count(), atom_count);
    assert_eq!(loaded.graph.relation_count(), relation_count);

    // Verify specific atoms survived
    for id in &ids {
        assert!(loaded.graph.get_atom(*id).is_some(), "Atom {:?} should survive roundtrip", id);
    }
}

#[test]
fn test_file_roundtrip_preserves_ticks() {
    let (state, ids) = make_state_with_knowledge();
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("state.json");

    let mut runtime = Runtime::with_state(EngineConfig::default(), state);
    let stimulus = Stimulus {
        source_nodes: ids[..3].to_vec(),
        query_context: vec![],
        label: "roundtrip".to_string(),
    };

    // Run 25 ticks
    for _ in 0..25 {
        runtime.tick(&stimulus);
    }

    assert_eq!(runtime.tick_count(), 25);

    // Save
    Storage::save_state(runtime.state(), &path).unwrap();

    // Load into new runtime
    let loaded_state = Storage::load_state(&path).unwrap();
    assert_eq!(loaded_state.tick, 25);

    let mut rt2 = Runtime::with_state(EngineConfig::default(), loaded_state);

    // Continue ticking from 25
    let result = rt2.tick(&stimulus);
    assert_eq!(result.tick, 26);

    println!("Tick continuity: 25 -> save -> load -> tick 26 ✓");
}

#[test]
fn test_file_roundtrip_query_works_after_load() {
    let (state, ids) = make_state_with_knowledge();
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("state.json");

    // Save
    Storage::save_state(&state, &path).unwrap();

    // Load
    let loaded = Storage::load_state(&path).unwrap();
    let mut runtime = Runtime::with_state(EngineConfig::default(), loaded);

    // Query should work on loaded state
    let stimulus = Stimulus {
        source_nodes: ids[..3].to_vec(),
        query_context: vec![],
        label: "query_after_load".to_string(),
    };

    for _ in 0..10 {
        runtime.tick(&stimulus);
    }

    let active = runtime.activations().values()
        .filter(|a| a.get() > rust_decimal::Decimal::from_str_exact("0.001").unwrap())
        .count();

    assert!(active > 0, "Query should activate atoms after load");
    println!("Query after load: {} active atoms", active);
}

#[test]
fn test_large_state_roundtrip() {
    let config = EngineConfig::default();
    let mut state = CognitiveState::new(config);

    // Create 10K atoms
    let mut ids = Vec::new();
    for i in 0..10_000 {
        let atom = CognitiveAtom::new(AtomId::new(), AtomType::Concept, format!("concept_{}", i));
        ids.push(atom.id);
        state.graph.add_atom(atom);
    }

    // Add 5K relations
    for i in 0..5_000 {
        let _ = state.graph.add_relation(ids[i], ids[i + 5_000], RelationType::Causes);
    }

    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("large_state.json");

    let start = Instant::now();
    Storage::save_state(&state, &path).unwrap();
    let save_time = start.elapsed();

    let start = Instant::now();
    let loaded = Storage::load_state(&path).unwrap();
    let load_time = start.elapsed();

    println!("Large state roundtrip (10K atoms, 5K relations):");
    println!("  Save: {:?}  Load: {:?}", save_time, load_time);

    assert_eq!(loaded.graph.atom_count(), 10_000);
    assert_eq!(loaded.graph.relation_count(), 5_000);

    // Verify file size
    let file_size = std::fs::metadata(&path).unwrap().len();
    println!("  File size: {:.1} MB", file_size as f64 / 1_048_576.0);
}
