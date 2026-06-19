//! Test 3: Scale Test — 100K+ atoms
//! Measures ingestion speed, tick duration, retrieval latency.

use dcma_core::atom::CognitiveAtom;
use dcma_core::state::CognitiveState;
use dcma_core::types::*;
use dcma_runtime::{Runtime, Stimulus};
use std::time::Instant;

fn atom_name(i: usize) -> String {
    format!("atom_{}", i)
}

#[test]
fn test_ingest_100k_atoms() {
    let target = 100_000;
    let config = EngineConfig::default();
    let mut state = CognitiveState::new(config);

    let start = Instant::now();
    for i in 0..target {
        let atom = CognitiveAtom::new(AtomId::new(), AtomType::Concept, atom_name(i));
        state.graph.add_atom(atom);
    }
    let elapsed = start.elapsed();
    let rate = target as f64 / elapsed.as_secs_f64();

    println!("Ingested {} atoms in {:?} ({:.0} atoms/sec)", target, elapsed, rate);
    assert_eq!(state.graph.atom_count(), target);
    assert!(elapsed.as_secs() < 60, "Should complete in < 60s");
}

#[test]
fn test_ingest_100k_with_relations() {
    let target = 100_000;
    let config = EngineConfig::default();
    let mut state = CognitiveState::new(config);

    let mut ids = Vec::with_capacity(target);
    let start = Instant::now();
    for i in 0..target {
        let atom = CognitiveAtom::new(AtomId::new(), AtomType::Concept, atom_name(i));
        ids.push(atom.id);
        state.graph.add_atom(atom);
    }

    for i in 0..50_000 {
        let _ = state.graph.add_relation(ids[i], ids[(i + 1) % target], RelationType::Causes);
    }
    let elapsed = start.elapsed();

    println!("Ingested {} atoms + 50K relations in {:?}", target, elapsed);
    assert_eq!(state.graph.atom_count(), target);
    assert_eq!(state.graph.relation_count(), 50_000);
}

#[test]
fn test_tick_10k_graph() {
    let target = 10_000;
    let config = EngineConfig::default();
    let mut state = CognitiveState::new(config.clone());

    let mut ids = Vec::with_capacity(target);
    for i in 0..target {
        let atom = CognitiveAtom::new(AtomId::new(), AtomType::Concept, atom_name(i));
        ids.push(atom.id);
        state.graph.add_atom(atom);
    }

    for i in 0..5_000 {
        let _ = state.graph.add_relation(ids[i], ids[i + 5_000], RelationType::Causes);
    }

    let mut runtime = Runtime::with_state(config, state);
    let stimulus = Stimulus {
        source_nodes: ids[..10].to_vec(),
        query_context: vec![],
        label: "scale_tick".to_string(),
    };

    let start = Instant::now();
    let result = runtime.tick(&stimulus);
    let tick_time = start.elapsed();

    println!("Tick on 10K-atom, 5K-relation graph: {:?}", tick_time);
    println!("  Converged: {}  Iterations: {}", result.convergence.converged, result.convergence.iterations);
    assert!(tick_time.as_secs() < 5, "Tick should complete in < 5s");
}

#[test]
fn test_50_ticks_stability_10k() {
    let target = 10_000;
    let config = EngineConfig::default();
    let mut state = CognitiveState::new(config.clone());

    let mut ids = Vec::with_capacity(target);
    for i in 0..target {
        let atom = CognitiveAtom::new(AtomId::new(), AtomType::Concept, atom_name(i));
        ids.push(atom.id);
        state.graph.add_atom(atom);
    }

    for i in 0..2_500 {
        let _ = state.graph.add_relation(ids[i], ids[i + 2_500], RelationType::Causes);
    }

    let mut runtime = Runtime::with_state(config, state);
    let stimulus = Stimulus {
        source_nodes: ids[..10].to_vec(),
        query_context: vec![],
        label: "stability".to_string(),
    };

    let start = Instant::now();
    for tick in 0..50 {
        let _ = runtime.tick(&stimulus);
        for (_, act) in runtime.activations() {
            assert!(
                act.get() >= rust_decimal::Decimal::ZERO && act.get() <= rust_decimal::Decimal::ONE,
                "Tick {}: activation {} out of range", tick, act.get()
            );
        }
    }

    let total = start.elapsed();
    println!("50 ticks on 10K-atom graph: {:?} total, {:?} avg", total, total / 50);
    assert_eq!(runtime.tick_count(), 50);
}

#[test]
fn test_retrieval_from_10k() {
    let target = 10_000;
    let config = EngineConfig::default();
    let mut state = CognitiveState::new(config.clone());

    let mut ids = Vec::with_capacity(target);
    for i in 0..target {
        let atom = CognitiveAtom::new(AtomId::new(), AtomType::Concept, atom_name(i));
        ids.push(atom.id);
        state.graph.add_atom(atom);
    }

    // Create clusters
    for cluster in 0..10 {
        let base = cluster * 1_000;
        for i in 0..100 {
            let src = ids[base + i * 10];
            let dst = ids[base + i * 10 + 5];
            let _ = state.graph.add_relation(src, dst, RelationType::Causes);
        }
        if cluster < 9 {
            let _ = state.graph.add_relation(ids[base + 500], ids[base + 1_000 + 500], RelationType::Supports);
        }
    }

    let mut runtime = Runtime::with_state(config, state);
    let warmup = Stimulus { source_nodes: vec![ids[0]], query_context: vec![], label: "warmup".to_string() };
    runtime.tick(&warmup);

    let seeds = [ids[0], ids[2_500], ids[5_000], ids[7_500], ids[9_999]];
    for (i, seed) in seeds.iter().enumerate() {
        let start = Instant::now();
        let query = Stimulus { source_nodes: vec![*seed], query_context: vec![], label: "query".to_string() };
        runtime.tick(&query);
        let elapsed = start.elapsed();

        let active = runtime.activations().values()
            .filter(|a| a.get() > rust_decimal::Decimal::from_str_exact("0.01").unwrap())
            .count();

        println!("  Retrieval from cluster {}: {:?}, {} active atoms", i, elapsed, active);
    }
}
