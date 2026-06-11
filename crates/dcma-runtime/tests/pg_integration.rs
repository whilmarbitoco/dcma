use dcma_core::atom::CognitiveAtom;
use dcma_core::state::CognitiveState;
use dcma_core::types::*;
use dcma_pg::PgStorage;
use dcma_runtime::{Runtime, Stimulus};
use serial_test::serial;

fn db_url() -> String {
    std::env::var("DCMA_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://dcma:dcma_secure_2026@llm-ciel-memory-qk6cz9:5432/dcma".to_string())
}

async fn fresh_storage() -> PgStorage {
    let storage = PgStorage::connect(&db_url()).await.unwrap();
    storage.clear().await.unwrap();
    storage
}

#[serial]
#[tokio::test]
async fn test_pg_save_and_load_empty_state() {
    let storage = fresh_storage().await;
    let config = EngineConfig::default();
    let state = CognitiveState::new(config);
    storage.save_state(&state).await.unwrap();
    let loaded = storage.load_state().await.unwrap();
    assert_eq!(loaded.graph.atom_count(), 0);
    assert_eq!(loaded.graph.relation_count(), 0);
}

#[serial]
#[tokio::test]
async fn test_pg_save_and_load_with_data() {
    let storage = fresh_storage().await;
    let config = EngineConfig::default();
    let mut state = CognitiveState::new(config);

    let a = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "rust_lang".to_string());
    let b = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "systems".to_string());
    let id_a = a.id;
    let id_b = b.id;
    state.graph.add_atom(a);
    state.graph.add_atom(b);
    state.graph.add_relation(id_a, id_b, RelationType::Causes).unwrap();

    storage.save_state(&state).await.unwrap();
    let loaded = storage.load_state().await.unwrap();

    assert_eq!(loaded.graph.atom_count(), 2);
    assert_eq!(loaded.graph.relation_count(), 1);
    assert!(loaded.graph.get_atom(id_a).is_some());
    assert!(loaded.graph.get_atom(id_b).is_some());

    let results = storage.search_atoms("rust", 10).await.unwrap();
    assert!(results.contains(&id_a));
}

#[serial]
#[tokio::test]
async fn test_pg_runtime_tick_persistence() {
    let storage = fresh_storage().await;
    let config = EngineConfig::default();
    let mut state = CognitiveState::new(config.clone());

    let a = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "ml".to_string());
    let b = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "nn".to_string());
    let id_a = a.id;
    let id_b = b.id;
    state.graph.add_atom(a);
    state.graph.add_atom(b);
    state.graph.add_relation(id_a, id_b, RelationType::Causes).unwrap();

    let mut runtime = Runtime::with_state(config, state);
    let stimulus = Stimulus {
        source_nodes: vec![id_a],
        query_context: vec![id_b],
        label: "test".into(),
    };
    let result = runtime.tick(&stimulus);
    assert_eq!(result.tick, 1);

    storage.save_state(runtime.state()).await.unwrap();
    let loaded = storage.load_state().await.unwrap();
    assert_eq!(loaded.graph.atom_count(), 2);
    assert_eq!(loaded.tick, 1);

    let mut rt2 = Runtime::with_state(EngineConfig::default(), loaded);
    let r2 = rt2.tick(&stimulus);
    assert_eq!(r2.tick, 2);
}

#[serial]
#[tokio::test]
async fn test_pg_roundtrip_integrity() {
    let storage = fresh_storage().await;
    let config = EngineConfig::default();
    let mut state = CognitiveState::new(config);

    let atoms: Vec<_> = (0..5).map(|i| {
        CognitiveAtom::new(AtomId::new(), AtomType::Concept, format!("c_{}", i))
    }).collect();
    let ids: Vec<_> = atoms.iter().map(|a| a.id).collect();
    for atom in atoms { state.graph.add_atom(atom); }

    for i in 0..4 {
        state.graph.add_relation(ids[i], ids[i + 1], RelationType::Causes).unwrap();
    }
    state.graph.add_relation(ids[0], ids[4], RelationType::Contradicts).unwrap();

    storage.save_state(&state).await.unwrap();
    let loaded = storage.load_state().await.unwrap();

    assert_eq!(loaded.graph.atom_count(), 5);
    assert_eq!(loaded.graph.relation_count(), 5);

    let contrs = loaded.graph.contradiction_edges();
    assert_eq!(contrs.len(), 1);

    let stats = storage.get_stats().await.unwrap();
    assert!(stats.atom_count >= 5);
    assert!(stats.relation_count >= 5);
}

#[serial]
#[tokio::test]
async fn test_pg_multiple_ticks_persist() {
    let storage = fresh_storage().await;
    let config = EngineConfig::default();
    let mut state = CognitiveState::new(config.clone());

    let a = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "tick_test".to_string());
    let id_a = a.id;
    state.graph.add_atom(a);

    let mut runtime = Runtime::with_state(config, state);
    let stimulus = Stimulus {
        source_nodes: vec![id_a],
        query_context: vec![],
        label: "tick".into(),
    };

    for i in 0..5 {
        let result = runtime.tick(&stimulus);
        assert_eq!(result.tick, (i + 1) as u64);
        storage.save_state(runtime.state()).await.unwrap();
    }

    let loaded = storage.load_state().await.unwrap();
    assert_eq!(loaded.tick, 5);
    assert_eq!(loaded.graph.atom_count(), 1);

    let mut rt2 = Runtime::with_state(EngineConfig::default(), loaded);
    let r2 = rt2.tick(&stimulus);
    assert_eq!(r2.tick, 6);
}
