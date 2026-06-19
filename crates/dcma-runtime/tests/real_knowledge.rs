//! Test 1: Real Knowledge Ingestion
//! Ingests a real ML knowledge corpus and verifies extraction quality.

use dcma_core::atom::CognitiveAtom;
use dcma_core::types::*;
use dcma_runtime::{Runtime, Stimulus};

const ML_KNOWLEDGE: &str = include_str!("../../../test_data/ml_knowledge.txt");

#[test]
fn test_real_knowledge_ingestion_extracts_entities() {
    let engine = dcma_ingest::IngestEngine::new();
    let entities = engine.extract_entities(ML_KNOWLEDGE);

    // Should extract at least 200 distinct entities from the corpus
    assert!(
        entities.len() >= 200,
        "Expected >= 200 entities from ML corpus, got {}",
        entities.len()
    );

    // Print extracted entities for visibility
    println!("Extracted {} entities:", entities.len());
    for e in entities.iter().take(20) {
        println!("  '{}'  type={}  conf={:.2}", e.name, e.entity_type, e.confidence);
    }
    if entities.len() > 20 {
        println!("  ... and {} more", entities.len() - 20);
    }

    // Verify key ML concepts are present (with flexible matching)
    let names: Vec<String> = entities.iter().map(|e| e.name.to_lowercase()).collect();

    let has_machine_learning = names.iter().any(|n| n.contains("machine learning"));
    let has_neural_networks = names.iter().any(|n| n.contains("neural network"));
    let has_deep_learning = names.iter().any(|n| n.contains("deep learning"));
    let has_transformer = names.iter().any(|n| n.contains("transformer"));
    let has_gradient_descent = names.iter().any(|n| n.contains("gradient descent"));

    assert!(has_machine_learning, "Should extract 'Machine Learning' (found: {:?})",
        names.iter().filter(|n| n.contains("machine")).collect::<Vec<_>>());
    assert!(has_neural_networks, "Should extract 'Neural Networks'");
    assert!(has_deep_learning, "Should extract 'Deep Learning'");
    assert!(has_transformer, "Should extract 'Transformer'");
    assert!(has_gradient_descent, "Should extract 'Gradient Descent'");
}

#[test]
fn test_real_knowledge_ingestion_extracts_relations() {
    let engine = dcma_ingest::IngestEngine::new();
    let entities = engine.extract_entities(ML_KNOWLEDGE);
    let relations = engine.extract_relations(&entities, ML_KNOWLEDGE);

    assert!(
        relations.len() >= 100,
        "Expected >= 100 relations, got {}",
        relations.len()
    );

    println!("Extracted {} relations", relations.len());

    // Print sample relations
    for rel in relations.iter().take(10) {
        let src_name = entities.get(rel.src_idx).map(|e| e.name.as_str()).unwrap_or("?");
        let dst_name = entities.get(rel.dst_idx).map(|e| e.name.as_str()).unwrap_or("?");
        println!("  {} --{}--> {}", src_name, rel.relation_type, dst_name);
    }

    // Verify key relation types are present
    let has_instance_of = relations.iter().any(|r| r.relation_type == "InstanceOf");
    let has_cooccurrence = relations.iter().any(|r| r.relation_type == "CoOccursWith");
    assert!(has_instance_of, "Should extract InstanceOf relations");
    assert!(has_cooccurrence, "Should extract CoOccursWith relations");
}

#[test]
fn test_real_knowledge_into_runtime() {
    let engine = dcma_ingest::IngestEngine::new();
    let entities = engine.extract_entities(ML_KNOWLEDGE);

    let config = EngineConfig::default();
    let mut state = dcma_core::state::CognitiveState::new(config.clone());

    let mut atom_ids = Vec::new();
    for entity in &entities {
        let atom_type = match entity.entity_type.as_str() {
            "Entity" => AtomType::Entity,
            _ => AtomType::Concept,
        };
        let id = AtomId::new();
        let atom = CognitiveAtom::new(id, atom_type, entity.name.clone());
        state.graph.add_atom(atom);
        atom_ids.push(id);
    }

    println!("Ingested {} atoms into runtime", state.graph.atom_count());
    assert!(state.graph.atom_count() >= 200, "Graph should have >= 200 atoms");

    let mut runtime = Runtime::with_state(config, state);

    // Run 10 cognitive ticks
    let seed: Vec<_> = atom_ids.iter().take(5).copied().collect();
    let stimulus = Stimulus {
        source_nodes: seed,
        query_context: vec![],
        label: "real_knowledge_test".into(),
    };

    for _ in 0..10 {
        let result = runtime.tick(&stimulus);
        assert!(result.tick > 0);
    }

    assert_eq!(runtime.tick_count(), 10);

    let active = runtime.activations();
    let active_count = active.values()
        .filter(|a| a.get() > rust_decimal::Decimal::from_str_exact("0.001").unwrap())
        .count();

    println!("Successfully ran 10 ticks on {}-atom graph", runtime.state().graph.atom_count());
    println!("Active atoms: {}", active_count);
    assert!(active_count > 0, "Should have some active atoms after ticks");
}
