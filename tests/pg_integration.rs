/// Integration test: DCMA with PostgreSQL storage
///
/// This test verifies the full cognitive cycle with PostgreSQL persistence:
/// 1. Create atoms and relations
/// 2. Run cognitive ticks
/// 3. Persist state to PostgreSQL
/// 4. Load state from PostgreSQL
/// 5. Verify data integrity
///
/// Requires: PostgreSQL database `dcma` on llm-ciel-memory-qk6cz9:5432
/// Run with: cargo test --test pg_integration -- --nocapture

use dcma_core::atom::CognitiveAtom;
use dcma_core::types::*;
use dcma_pg::PgStorage;
use dcma_runtime::{Runtime, Stimulus};
use rust_decimal::Decimal;

fn db_url() -> String {
    std::env::var("DCMA_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://dcma:dcma_secure_2026@llm-ciel-memory-qk6cz9:5432/dcma".to_string())
}

#[tokio::test]
async fn test_pg_save_and_load_empty_state() {
    let storage = PgStorage::connect(&db_url()).await.unwrap();
    let config = EngineConfig::default();
    let state = CognitiveState::new(config);

    storage.save_state(&state).await.unwrap();
    let loaded = storage.load_state().await.unwrap();

    assert_eq!(loaded.graph.atom_count(), 0);
    assert_eq!(loaded.graph.relation_count(), 0);
    assert_eq!(loaded.tick, 0);
}

#[tokio::test]
async fn test_pg_save_and_load_with_data() {
    let storage = PgStorage::connect(&db_url()).await.unwrap();
    let config = EngineConfig::default();
    let mut state = CognitiveState::new(config);

    // Create atoms
    let a = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "rust_language".to_string());
    let b = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "systems_programming".to_string());
    let c = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "memory_safety".to_string());
    let id_a = a.id;
    let id_b = b.id;
    let id_c = c.id;

    state.graph.add_atom(a);
    state.graph.add_atom(b);
    state.graph.add_atom(c);

    // Create relations
    state.graph.add_relation(id_a, id_b, RelationType::Causes).unwrap();
    state.graph.add_relation(id_a, id_c, RelationType::Enables).unwrap();

    // Set activation
    state.set_activation(id_a, Activation::new(Decimal::ONE));

    // Save
    storage.save_state(&state).await.unwrap();

    // Load
    let loaded = storage.load_state().await.unwrap();

    assert_eq!(loaded.graph.atom_count(), 3);
    assert_eq!(loaded.graph.relation_count(), 2);
    assert!(loaded.graph.get_atom(id_a).is_some());
    assert!(loaded.graph.get_atom(id_b).is_some());
    assert!(loaded.graph.get_atom(id_c).is_some());

    // Verify atom data
    let atom_a = loaded.graph.get_atom(id_a).unwrap();
    assert_eq!(atom_a.canonical, "rust_language");
    assert_eq!(atom_a.atom_type, AtomType::Concept);

    // Verify search
    let results = storage.search_atoms("rust", 10).await.unwrap();
    assert!(results.contains(&id_a));
}

#[tokio::test]
async fn test_pg_runtime_tick_persistence() {
    let storage = PgStorage::connect(&db_url()).await.unwrap();
    let config = EngineConfig::default();

    // Create initial state with atoms
    let mut state = CognitiveState::new(config.clone());
    let a = CognitiveAgent::new(AtomId::new(), AtomType::Concept, "machine_learning".to_string());
    let b = CognitiveAgent::new(AtomId::new(), AtomType::Concept, "neural_networks".to_string());
    let id_a = a.id;
    let id_b = b.id;
    state.graph.add_atom(a);
    state.graph.add_atom(b);
    state.graph.add_relation(id_a, id_b, RelationType::Causes).unwrap();

    // Create runtime with state
    let mut runtime = Runtime::with_state(config, state);

    // Run a tick
    let stimulus = Stimulus {
        source_nodes: vec![id_a],
        query_context: vec![id_b],
        label: "test".into(),
    };
    let result = runtime.tick(&stimulus);

    // Verify tick executed
    assert_eq!(result.tick, 1);
    assert!(result.convergence.converged);

    // Persist to PostgreSQL
    storage.save_state(runtime.state()).await.unwrap();

    // Load into a new runtime
    let loaded_state = storage.load_state().await.unwrap();
    assert_eq!(loaded_state.graph.atom_count(), 2);
    assert_eq!(loaded_state.graph.relation_count(), 1);
    assert_eq!(loaded_state.tick, 1);

    // Verify the loaded state can continue ticking
    let mut runtime2 = Runtime::with_state(EngineConfig::default(), loaded_state);
    let result2 = runtime2.tick(&stimulus);
    assert_eq!(result2.tick, 2);
}

#[tokio::test]
async fn test_pg_state_roundtrip_integrity() {
    let storage = PgStorage::connect(&db_url()).await.unwrap();
    let config = EngineConfig::default();
    let mut state = CognitiveState::new(config);

    // Build a small knowledge graph
    let atoms: Vec<_> = (0..5).map(|i| {
        CognitiveAtom::new(AtomId::new(), AtomType::Concept, format!("concept_{}", i))
    }).collect();
    let ids: Vec<_> = atoms.iter().map(|a| a.id).collect();

    for atom in atoms {
        state.graph.add_atom(atom);
    }

    // Create a chain: 0 -> 1 -> 2 -> 3 -> 4
    for i in 0..4 {
        state.graph.add_relation(ids[i], ids[i + 1], RelationType::Causes).unwrap();
    }

    // Add a contradiction: 0 contradicts 4
    state.graph.add_relation(ids[0], ids[4], RelationType::Contradicts).unwrap();

    // Save and load
    storage.save_state(&state).await.unwrap();
    let loaded = storage.load_state().await.unwrap();

    // Verify structure
    assert_eq!(loaded.graph.atom_count(), 5);
    assert_eq!(loaded.graph.relation_count(), 5); // 4 causal + 1 contradiction

    // Verify contradiction exists
    let contrs = loaded.graph.contradiction_edges();
    assert_eq!(contrs.len(), 1);
    assert_eq!(contrs[0].2, ids[0]); // src
    assert_eq!(contrs[0].3, ids[4]); // dst

    // Verify stats
    let stats = storage.get_stats().await.unwrap();
    assert_eq!(stats.atom_count, 5);
    assert_eq!(stats.relation_count, 5);
}

#[tokio::test]
async fn test_pg_multiple_ticks_persist_correctly() {
    let storage = PgStorage::connect(&db_url()).await.unwrap();
    let config = EngineConfig::default();
    let mut state = CognitiveState::new(config.clone());

    let a = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "activation_test".to_string());
    let id_a = a.id;
    state.graph.add_atom(a);

    let mut runtime = Runtime::with_state(config, state);

    // Run 5 ticks, persisting after each
    for i in 0..5 {
        let stimulus = Stimulus {
            source_nodes: vec![id_a],
            query_context: vec![],
            label: format!("tick_{}", i),
        };
        let result = runtime.tick(&stimulus);
        assert_eq!(result.tick, (i + 1) as u64);

        // Persist
        storage.save_state(runtime.state()).await.unwrap();
    }

    // Load and verify final state
    let loaded = storage.load_state().await.unwrap();
    assert_eq!(loaded.tick, 5);
    assert_eq!(loaded.graph.atom_count(), 1);

    // Verify we can continue from loaded state
    let mut runtime2 = Runtime::with_state(EngineConfig::default(), loaded);
    let stimulus = Stimulus {
        source_nodes: vec![id_a],
        query_context: vec![],
        label: "tick_5".into(),
    };
    let result = runtime2.tick(&stimulus);
    assert_eq!(result.tick, 6);
}

// Helper to fix the typo in the test above
use dcma_core::atom::CognitiveAtom as CognitiveAgent;
