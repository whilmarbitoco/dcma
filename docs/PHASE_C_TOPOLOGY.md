# DCMA Phase C — Topology Engine

## Context

DCMA workspace at `/workspace/dcma`. Phases 1-5 complete. Phase B (Consolidation) is being implemented in parallel. Phase C implements the **Topology Engine** — edge weight evolution, relation stabilization, and graph partitioning.

The formal spec is at `docs/DCMA_FORMAL_SPEC.md`. The build plan is at `docs/BUILD_PLAN.md`.

## What to Implement

### 1. `crates/dcma-topology/src/edge.rs`

Implement edge weight evolution (spec Section 2.3 and 9.2):

```
w(e, t+1) = clamp_{[0,1]}(w(e,t) + Δw_reinforce - Δw_decay + Δw_contradiction)
```

- `Δw_reinforce = η_r × r(e,t)` — reinforcement increment
- `Δw_decay = η_d × w(e,t)` — proportional decay
- `Δw_contradiction = -η_c × c(e,t)` — contradiction suppression
- `clamp_{[0,1]}(x) = max(0, min(1, x))`

Relation stabilization (spec Section 9.2):
```
stability(e, t) = 1 - σ({w(e,τ) : τ ∈ [t-T_s, t]}) / (μ({w(e,τ) : τ ∈ [t-T_s, t]}) + ε)
```

- T_s = stabilization window (default 50 ticks)
- θ_stab = 0.8 — edges above this are stable and resist decay

Hub splitting (spec Section 6.3):
- If degree(v) > 5 × average_degree AND degree(v) > 5:
  - Create ⌈degree(v)/3⌉ sub-nodes
  - Distribute edges among sub-nodes
  - Mark original as DORMANT

Key types:
```rust
#[derive(Debug, Clone)]
pub struct WeightChangeReport {
    pub edges_updated: usize,
    pub total_reinforcement: Decimal,
    pub total_decay: Decimal,
    pub total_contradiction_suppression: Decimal,
}

#[derive(Debug, Clone)]
pub struct StabilizationReport {
    pub newly_stabilized: Vec<EdgeIndex>,
    pub destabilized: Vec<EdgeIndex>,
    pub stable_count: usize,
}

#[derive(Debug, Clone)]
pub struct HubSplitResult {
    pub original: AtomId,
    pub new_nodes: Vec<AtomId>,
    pub edges_redistributed: usize,
}
```

### 2. `crates/dcma-topology/src/partition.rs`

Implement spectral partitioning (spec Section 9.3):

```
partition(G, k) = spectral_cluster(L, k)
```

where L is the graph Laplacian.

Implementation approach:
- Build adjacency matrix and degree matrix from CognitiveGraph
- Compute Laplacian L = D - A
- Use power iteration to approximate first k eigenvectors
- Assign nodes to partitions by eigenvector sign/value
- Minimize inter-partition edges, balance partition sizes

Key types:
```rust
#[derive(Debug, Clone)]
pub struct Partition {
    pub id: usize,
    pub nodes: Vec<AtomId>,
    pub internal_edges: usize,
    pub external_edges: usize,
}

#[derive(Debug, Clone)]
pub struct PartitionReport {
    pub partitions: Vec<Partition>,
    pub cut_edges: usize,
    pub balance_score: Decimal,  // 1.0 = perfectly balanced
}
```

### 3. `crates/dcma-topology/src/lib.rs`

Public API:
```rust
pub fn evolve_weights(graph: &mut CognitiveGraph, config: &EngineConfig, tick: u64) -> WeightChangeReport;
pub fn stabilize_relations(graph: &mut CognitiveGraph, config: &EngineConfig) -> StabilizationReport;
pub fn split_hubs(graph: &mut CognitiveGraph, config: &EngineConfig) -> Vec<HubSplitResult>;
pub fn partition(graph: &CognitiveGraph, k: usize) -> PartitionReport;
```

Re-export all report types.

### 4. `crates/dcma-topology/Cargo.toml`

```toml
[package]
name = "dcma-topology"
version = "0.1.0"
edition = "2021"

[dependencies]
dcma-core = { path = "../dcma-core" }
rust_decimal = "1"
serde = { version = "1", features = ["derive"] }
thiserror = "1"
```

## Testing Requirements

1. **test_weight_reinforce** — Edge weight increases after reinforcement
2. **test_weight_decay** — Edge weight decreases over time with no reinforcement
3. **test_weight_clamped** — Weight stays in [0, 1] after many iterations
4. **test_weight_contradiction** — Contradiction reduces weight
5. **test_stability_constant** — Constant weight → stability ≈ 1.0
6. **test_stability_varying** — Varying weight → stability < 1.0
7. **test_stability_threshold** — Edges above θ_stab marked stable
8. **test_hub_split** — High-degree node split into sub-nodes
9. **test_hub_no_split_low_degree** — Low-degree node not split
10. **test_partition_count** — partition(graph, k) produces k partitions
11. **test_partition_coverage** — All nodes assigned to exactly one partition
12. **test_partition_balance** — No partition has >2x nodes of another
13. **test_evolve_deterministic** — Same graph + config → same result

## Important Constraints

- Use `rust_decimal::Decimal` for all arithmetic
- All functions must be deterministic
- Follow existing code style
- Use `thiserror` for error types
- Run `cargo build` and `cargo test` after implementation
- Fix all compiler warnings

## Existing Code Reference

- `crates/dcma-core/src/graph.rs` — CognitiveGraph with atoms(), relations(), update_edge_weight(), degree_distribution(), node_indices()
- `crates/dcma-core/src/types.rs` — EngineConfig with all parameters
- `crates/dcma-core/src/relation.rs` — Relation with stability field
- `crates/dcma-contradiction/src/pressure.rs` — Reference for contradiction integration
