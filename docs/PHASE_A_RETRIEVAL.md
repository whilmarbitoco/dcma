# DCMA Phase A — Retrieval Engine Implementation

## Context

DCMA (Deterministic Cognitive Memory Architecture) is a Rust workspace at `/workspace/dcma`. The formal spec is at `docs/DCMA_FORMAL_SPEC.md`. The build plan is at `docs/BUILD_PLAN.md`.

Phases 1–5 are complete (core types, activation, contradiction, entropy). Phase A implements the **Retrieval Engine** — the most critical missing piece.

## What to Implement

### 1. `crates/dcma-retrieval/src/relevance.rs`

Implement the relevance functional R(G_q, S, q) from spec Section 4.1:

```
R(G_q, S, q) = α·C(G_q,q) + β·Σ(G_q) + γ·A(G_q) + δ·T(G_q) - λ·D(G_q)
```

- `C(G_q, q)` — contextual coherence: average structural similarity of nodes in subgraph to query
- `Σ(G_q)` — salience: average node salience in subgraph
- `A(G_q)` — activation resonance: average activation in subgraph
- `T(G_q)` — temporal relevance: exp(-μ · (t - t_last)) averaged
- `D(G_q)` — traversal cost: diameter(subgraph) / diameter(full graph)
- Coefficients: α=0.35, β=0.20, γ=0.20, δ=0.15, λ=0.10 (must sum to 1)
- Also implement contradiction-modified relevance: R_contrad(G_q) = R(G_q) · (1 - avg_contradiction_pressure)

Import from dcma_core:
- `CognitiveGraph`, `CognitiveAtom`, `Relation`, `RelationType`
- `AtomId`, `AtomType`, `AtomState`, `Activation`, `Weight`, `Salience`, `Confidence`
- `EngineConfig`

### 2. `crates/dcma-retrieval/src/similarity.rs`

Implement structural similarity from spec Section 4.2:

```
sim_struct(v, q) = Jaccard(N_k(v), N_k(q)) · weight(path(v,q)) / diameter(G)
```

- k-hop neighborhood: use `graph.k_hop_neighbors(id, k)` from CognitiveGraph
- Jaccard: |intersection| / |union| of NodeIndex sets
- Path strength: product of edge weights along shortest path (use BFS for unweighted path existence, then multiply weights)
- If no path exists, similarity = 0

### 3. `crates/dcma-retrieval/src/beam.rs`

Implement bounded beam search from spec Section 4.4:

```
Algorithm: RETRIEVE(G, q, d_max, n_max, beam_width)
```

- Initialize frontier with singleton sets for each query node
- For each depth 1..d_max:
  - Expand each candidate by adding neighbors not already in subgraph
  - Score each candidate with relevance functional
  - Keep top beam_width by score
  - Track best overall
- Enforce n_max constraint (skip expansion if |V| = n_max)
- Deterministic tie-breaking: by AtomId (UUID) lexicographic order
- Return best subgraph as CognitiveField

Define `CognitiveField` struct:
```rust
pub struct CognitiveField {
    pub nodes: Vec<AtomId>,
    pub relevance_score: Decimal,
    pub salience_score: Decimal,
    pub activation_score: Decimal,
    pub contradiction_flags: Vec<bool>,
    pub metadata: RetrievalMetadata,
}

pub struct RetrievalMetadata {
    pub traversal_depth: usize,
    pub beam_width_used: usize,
    pub query_nodes: Vec<AtomId>,
}
```

### 4. `crates/dcma-retrieval/src/lib.rs`

Public API:
```rust
pub fn retrieve(graph: &CognitiveGraph, query_nodes: &[AtomId], activations: &HashMap<AtomId, Activation>, tick: u64, config: &EngineConfig) -> CognitiveField;
pub fn retrieve_with_contradiction(graph: &CognitiveGraph, query_nodes: &[AtomId], activations: &HashMap<AtomId, Activation>, contradictions: &HashMap<(AtomId, AtomId), Pressure>, tick: u64, config: &EngineConfig) -> CognitiveField;
```

Re-export CognitiveField, RetrievalMetadata.

### 5. `crates/dcma-retrieval/Cargo.toml`

```toml
[package]
name = "dcma-retrieval"
version = "0.1.0"
edition = "2021"

[dependencies]
dcma-core = { path = "../dcma-core" }
rust_decimal = "1"
serde = { version = "1", features = ["derive"] }
thiserror = "1"
```

## Testing Requirements

Add `#[cfg(test)]` modules with tests for:

1. **test_relevance_functional** — Create a small graph (5 nodes, 4 edges), compute relevance manually, verify the function matches
2. **test_structural_similarity_identical** — Same node queried against itself → similarity = 1.0
3. **test_structural_similarity_disconnected** — Disconnected nodes → similarity = 0.0
4. **test_beam_search_finds_query** — Query nodes are always in the result
5. **test_beam_search_respects_n_max** — Result never exceeds n_max nodes
6. **test_beam_search_respects_d_max** — Result nodes are within d_max hops of query
7. **test_beam_search_deterministic** — Same query on same graph produces same result
8. **test_retrieve_empty_query** — Empty query returns empty CognitiveField
9. **test_contradiction_reduces_relevance** — Contradicted nodes have lower effective relevance
10. **test_retrieve_single_node** — Single query node returns at minimum that node

## Important Constraints

- Use `rust_decimal::Decimal` for all arithmetic (no f64)
- All functions must be deterministic (no randomness)
- Follow existing code style in the workspace (see dcma-activation for reference)
- Use `thiserror` for error types
- Add `pub use` re-exports in lib.rs
- Run `cargo build` and `cargo test` after implementation
- Fix any compiler warnings

## Existing Code Reference

- `crates/dcma-core/src/graph.rs` — CognitiveGraph with k_hop_neighbors, inbound/outbound_neighbors, degree, diameter, subgraph methods
- `crates/dcma-core/src/types.rs` — All newtype definitions, EngineConfig with retrieval params (beam_width, max_retrieval_depth, max_subgraph_size)
- `crates/dcma-core/src/atom.rs` — CognitiveAtom with salience, activation_history, t_last_accessed
- `crates/dcma-activation/src/propagation.rs` — Reference for code style, sigmoid, ActivationVector pattern
