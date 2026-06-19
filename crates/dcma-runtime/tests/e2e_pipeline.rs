//! Test 2: End-to-End Pipeline
//! Full pipeline: raw text -> ingest -> atoms -> tick -> retrieval -> verify

use dcma_core::atom::CognitiveAtom;
use dcma_core::state::CognitiveState;
use dcma_core::types::*;
use dcma_runtime::{Runtime, Stimulus};

const TEST_CORPUS: &str = include_str!("../../../test_data/ml_knowledge.txt");

#[test]
fn test_full_pipeline_text_to_retrieval() {
    // Step 1: Ingest raw text
    let engine = dcma_ingest::IngestEngine::new();
    let entities = engine.extract_entities(TEST_CORPUS);
    let relations = engine.extract_relations(&entities, TEST_CORPUS);
    assert!(entities.len() >= 100, "Need >= 100 entities for e2e test");

    // Step 2: Build runtime from ingested knowledge
    let config = EngineConfig::default();
    let mut state = CognitiveState::new(config.clone());

    let mut atom_map: std::collections::HashMap<String, AtomId> = std::collections::HashMap::new();
    for entity in &entities {
        let atom_type = match entity.entity_type.as_str() {
            "Entity" => AtomType::Entity,
            _ => AtomType::Concept,
        };
        let id = AtomId::new();
        let atom = CognitiveAtom::new(id, atom_type, entity.name.clone());
        state.graph.add_atom(atom);
        atom_map.insert(entity.name.to_lowercase(), id);
    }

    // Step 3: Run cognitive ticks
    let mut runtime = Runtime::with_state(config, state);
    let seed_ids: Vec<AtomId> = atom_map.values().take(10).copied().collect();
    let stimulus = Stimulus {
        source_nodes: seed_ids,
        query_context: vec![],
        label: "e2e".into(),
    };
    for _ in 0..20 {
        runtime.tick(&stimulus);
    }

    // Step 4: Retrieve using a seed node
    let query_seed = atom_map.iter()
        .find(|(k, _)| k.contains("deep learning"))
        .map(|(_, v)| *v)
        .expect("Should have a deep learning atom");

    let query_stimulus = Stimulus {
        source_nodes: vec![query_seed],
        query_context: vec![],
        label: "query".into(),
    };
    runtime.tick(&query_stimulus);

    // Step 5: Verify retrieval activated related concepts
    let activations = runtime.activations();
    let active_count = activations.values()
        .filter(|a| a.get() > rust_decimal::Decimal::from_str_exact("0.001").unwrap())
        .count();

    assert!(active_count >= 5, "Retrieval should activate >= 5 atoms, got {}", active_count);

    println!("E2E pipeline complete:");
    println!("  Entities: {}  Relations: {}  Atoms: {}", entities.len(), relations.len(), atom_map.len());
    println!("  Ticks: {}  Active atoms: {}", runtime.tick_count(), active_count);
}

#[test]
fn test_pipeline_stability_on_real_data() {
    let engine = dcma_ingest::IngestEngine::new();
    let entities = engine.extract_entities(TEST_CORPUS);

    let config = EngineConfig::default();
    let mut state = CognitiveState::new(config.clone());

    for entity in &entities[..std::cmp::min(50, entities.len())] {
        let atom_type = match entity.entity_type.as_str() {
            "Entity" => AtomType::Entity,
            _ => AtomType::Concept,
        };
        let atom = CognitiveAtom::new(AtomId::new(), atom_type, entity.name.clone());
        state.graph.add_atom(atom);
    }

    let mut runtime = Runtime::with_state(config, state);
    let all_ids: Vec<_> = runtime.state().graph.atoms().map(|(_, a)| a.id).collect();

    // Use 5 seed nodes
    let stimulus = Stimulus {
        source_nodes: all_ids.into_iter().take(5).collect(),
        query_context: vec![],
        label: "stability".into(),
    };

    // Run 50 ticks - all activations must stay bounded
    for i in 0..50 {
        let _ = runtime.tick(&stimulus);
        for (id, act) in runtime.activations() {
            assert!(
                act.get() >= rust_decimal::Decimal::ZERO && act.get() <= rust_decimal::Decimal::ONE,
                "Tick {}: activation for {:?} = {} out of range", i, id, act.get()
            );
        }
    }

    println!("Stability test passed: 50 ticks on {}-atom graph", runtime.state().graph.atom_count());
}
