use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use dcma_core::atom::CognitiveAtom;
use dcma_core::state::CognitiveState;
use dcma_core::types::*;
use dcma_runtime::{Runtime, Stimulus};
use std::time::Duration;

fn atom_name(i: usize) -> String {
    format!("atom_{}", i)
}

fn build_graph(size: usize, relation_ratio: f64) -> (CognitiveState, Vec<AtomId>) {
    let config = EngineConfig::default();
    let mut state = CognitiveState::new(config);
    let mut ids = Vec::with_capacity(size);
    for i in 0..size {
        let atom_type = if i % 3 == 0 { AtomType::Entity } else { AtomType::Concept };
        let atom = CognitiveAtom::new(AtomId::new(), atom_type, atom_name(i));
        ids.push(atom.id);
        state.graph.add_atom(atom);
    }
    let rel_count = (size as f64 * relation_ratio) as usize;
    for i in 0..rel_count {
        let _ = state.graph.add_relation(ids[i % size], ids[(i + 1) % size], RelationType::Causes);
    }
    (state, ids)
}

fn bench_ingestion(c: &mut Criterion) {
    let mut group = c.benchmark_group("ingestion");
    for &size in &[1_000usize, 10_000, 50_000, 100_000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_function(BenchmarkId::new("atoms_only", size), |b| {
            b.iter(|| {
                let config = EngineConfig::default();
                let mut state = CognitiveState::new(config);
                for i in 0..size {
                    let atom = CognitiveAtom::new(AtomId::new(), AtomType::Concept, atom_name(i));
                    state.graph.add_atom(atom);
                }
                state.graph.atom_count()
            });
        });
    }
    group.finish();
}

fn bench_ingestion_with_relations(c: &mut Criterion) {
    let mut group = c.benchmark_group("ingestion_relations");
    for &size in &[1_000usize, 10_000, 50_000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_function(BenchmarkId::new("atoms_and_relations", size), |b| {
            b.iter(|| {
                let (state, _) = build_graph(size, 0.5);
                state.graph.atom_count() + state.graph.relation_count()
            });
        });
    }
    group.finish();
}

fn bench_tick(c: &mut Criterion) {
    let mut group = c.benchmark_group("tick");
    group.measurement_time(Duration::from_secs(30));
    for &size in &[100usize, 1_000, 5_000, 10_000] {
        group.bench_function(BenchmarkId::new("cognitive_tick", size), |b| {
            let (state, ids) = build_graph(size, 0.5);
            let config = EngineConfig::default();
            let mut runtime = Runtime::with_state(config, state);
            let stimulus = Stimulus {
                source_nodes: ids[..10.min(ids.len())].to_vec(),
                query_context: vec![],
                label: "bench".to_string(),
            };
            b.iter(|| runtime.tick(&stimulus));
        });
    }
    group.finish();
}

fn bench_retrieval(c: &mut Criterion) {
    let mut group = c.benchmark_group("retrieval");
    for &size in &[1_000usize, 5_000, 10_000] {
        group.bench_function(BenchmarkId::new("query", size), |b| {
            let (state, ids) = build_graph(size, 0.5);
            let config = EngineConfig::default();
            let mut runtime = Runtime::with_state(config, state);
            let warmup = Stimulus { source_nodes: vec![ids[0]], query_context: vec![], label: "warmup".to_string() };
            runtime.tick(&warmup);
            let query = Stimulus { source_nodes: vec![ids[0]], query_context: vec![], label: "query".to_string() };
            b.iter(|| runtime.tick(&query));
        });
    }
    group.finish();
}

fn bench_contradiction(c: &mut Criterion) {
    let mut group = c.benchmark_group("contradiction");
    for &size in &[100usize, 500, 1_000] {
        group.bench_function(BenchmarkId::new("detect", size), |b| {
            b.iter(|| {
                let config = EngineConfig::default();
                let mut state = CognitiveState::new(config);
                let mut ids = Vec::new();
                for i in 0..size {
                    let atom = CognitiveAtom::new(AtomId::new(), AtomType::Concept, format!("c_{}", i));
                    ids.push(atom.id);
                    state.graph.add_atom(atom);
                }
                for i in (0..size).step_by(2) {
                    if i + 1 < size {
                        state.graph.add_relation(ids[i], ids[i + 1], RelationType::Contradicts).unwrap();
                    }
                }
                let mut runtime = Runtime::with_state(EngineConfig::default(), state);
                let stimulus = Stimulus { source_nodes: ids[..10].to_vec(), query_context: vec![], label: "contradiction_bench".to_string() };
                runtime.tick(&stimulus);
                runtime.state().contradictions.len()
            });
        });
    }
    group.finish();
}

fn bench_full_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_pipeline");
    group.measurement_time(Duration::from_secs(60));
    for &size in &[100usize, 1_000, 5_000] {
        group.bench_function(BenchmarkId::new("ingest_tick_retrieve", size), |b| {
            b.iter(|| {
                let (state, ids) = build_graph(size, 0.3);
                let config = EngineConfig::default();
                let mut runtime = Runtime::with_state(config, state);
                let stimulus = Stimulus { source_nodes: ids[..5].to_vec(), query_context: vec![], label: "pipeline".to_string() };
                runtime.tick(&stimulus);
                let query = Stimulus { source_nodes: vec![ids[0]], query_context: vec![], label: "retrieve".to_string() };
                runtime.tick(&query);
                runtime.tick_count()
            });
        });
    }
    group.finish();
}

fn bench_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization");
    for &size in &[1_000usize, 10_000, 50_000] {
        let (state, _) = build_graph(size, 0.5);
        let json = serde_json::to_string(&state).unwrap();
        group.bench_function(BenchmarkId::new("serialize", size), |b| {
            b.iter(|| serde_json::to_string(&state).unwrap().len());
        });
        group.bench_function(BenchmarkId::new("deserialize", size), |b| {
            b.iter(|| {
                let _: CognitiveState = serde_json::from_str(&json).unwrap();
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_ingestion, bench_ingestion_with_relations, bench_tick, bench_retrieval, bench_contradiction, bench_full_pipeline, bench_serialization);
criterion_main!(benches);
