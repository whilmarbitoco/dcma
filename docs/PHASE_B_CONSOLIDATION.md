# DCMA Phase B — Consolidation Engine

## Context

DCMA workspace at `/workspace/dcma`. Phases 1-5 are complete (core, activation, contradiction, entropy, retrieval). Phase B implements the **Consolidation Engine** — motif detection and crystallization.

The formal spec is at `docs/DCMA_FORMAL_SPEC.md`. The build plan is at `docs/BUILD_PLAN.md`.

## What to Implement

### 1. `crates/dcma-consolidation/src/motif.rs`

Implement bounded VF2-like subgraph isomorphism for motif detection (spec Section 7.1):

```
Algorithm: DETECT_MOTIFS(G, min_size, max_size, min_recurrence)
```

- Enumerate all connected subgraphs of size min_size..=max_size (default max_size=5)
- For each subgraph, compute a canonical form (sorted adjacency representation)
- Group by canonical form to find isomorphic patterns
- Count occurrences per pattern
- Filter by min_recurrence (default 5)
- Return motifs with their recurrence count and stability score

Key types:
```rust
#[derive(Debug, Clone)]
pub struct Motif {
    pub pattern: Vec<NodeIndex>,           // canonical representative nodes
    pub size: usize,
    pub recurrence: usize,
    pub stability: Activation,              // 1 - var(weights)/mean(weights)
    pub canonical_form: String,            // string representation for dedup
}

#[derive(Debug, Clone)]
pub struct MotifReport {
    pub motifs: Vec<Motif>,
    pub total_patterns_found: usize,
    pub total_recurring: usize,
}
```

Implementation approach:
- For each node, grow connected subgraphs up to size s_max using BFS/DFS
- Canonical form: sort node degrees + sorted edge (type, weight) pairs as string
- Use HashMap<String, Vec<Vec<NodeIndex>>> to group isomorphic patterns
- Count occurrences per canonical form

### 2. `crates/dcma-consolidation/src/crystallization.rs`

Implement crystallization (spec Section 7.2-7.4):

```
Algorithm: CRYSTALLIZE(motif m, recurrence r)
```

- Check criteria: recurrence >= θ_cryst (default 5) AND stability >= θ_stab (default 0.7)
- Create new ABSTRACTION node:
  - canonical = canonicalize(m) — descriptive string from pattern
  - edges to all participating nodes with type=ABSTRACTION_OF, weight=1.0
  - edges for GENERALIZES with mean motif weight
  - salience = mean(salience of participating nodes) * ln(recurrence)
  - confidence = stability score
- Mark participating atoms as CONSOLIDATED (not deleted — Law 4)
- Hierarchical: abstraction nodes can participate in higher-level motifs (max depth 5)

Key types:
```rust
#[derive(Debug, Clone)]
pub struct CrystallizationResult {
    pub new_abstractions: Vec<AtomId>,
    pub consolidated_nodes: Vec<AtomId>,
    pub report: CrystallizationReport,
}

#[derive(Debug, Clone)]
pub struct CrystallizationReport {
    pub motifs_checked: usize,
    pub motifs_crystallized: usize,
    pub motifs_skipped_low_recurrence: usize,
    pub motifs_skipped_low_stability: usize,
}
```

### 3. `crates/dcma-consolidation/src/lib.rs`

Public API:
```rust
pub fn detect_motifs(graph: &CognitiveGraph, config: &EngineConfig) -> MotifReport;
pub fn crystallize(graph: &mut CognitiveGraph, motifs: &[Motif], config: &EngineConfig) -> CrystallizationResult;
pub fn consolidate(graph: &mut CognitiveGraph, config: &EngineConfig) -> CrystallizationReport;
```

Re-export Motif, MotifReport, CrystallizationResult, CrystallizationReport.

### 4. `crates/dcma-consolidation/Cargo.toml`

```toml
[package]
name = "dcma-consolidation"
version = "0.1.0"
edition = "2021"

[dependencies]
dcma-core = { path = "../dcma-core" }
rust_decimal = "1"
serde = { version = "1", features = ["derive"] }
thiserror = "1"
```

## Testing Requirements

1. **test_detect_single_motif** — Create graph with 3 identical triangle motifs, verify detection
2. **test_no_motifs_below_threshold** — Motif appears 4 times (< 5 threshold), not reported
3. **test_motif_stability** — Verify stability score for uniform vs varying edge weights
4. **test_crystallize_creates_abstraction_node** — After crystallization, ABSTRACTION node exists with correct edges
5. **test_crystallize_marks_consolidated** — Participating atoms marked CONSOLIDATED
6. **test_crystallize_skips_low_recurrence** — Motif with recurrence < 5 not crystallized
7. **test_crystallize_skips_low_stability** — Motif with stability < 0.7 not crystallized
8. **test_hierarchical_abstraction** — Level-1 abstractions can form level-2 motifs
9. **test_empty_graph_no_motifs** — Empty graph returns empty report
10. **test_single_node_no_motifs** — Single node has no motifs (min_size=2)
11. **test_consolidate_end_to_end** — detect_motifs → crystallize → verify graph state
12. **test_crystallize_preserves_existing_edges** — Non-motif edges untouched after crystallization

## Important Constraints

- Use `rust_decimal::Decimal` for all arithmetic
- All functions must be deterministic
- Follow existing code style (see dcma-activation, dcma-contradiction for reference)
- Use `thiserror` for error types
- Run `cargo build` and `cargo test` after implementation
- Fix all compiler warnings

## Existing Code Reference

- `crates/dcma-core/src/graph.rs` — CognitiveGraph with k_hop_neighbors, atoms(), relations(), subgraph(), degree_distribution()
- `crates/dcma-core/src/types.rs` — EngineConfig with crystallization_threshold, stability_threshold, max_motif_size, max_abstraction_depth
- `crates/dcma-core/src/atom.rs` — CognitiveAtom with AtomType::Abstraction, AtomState::Consolidated
- `crates/dcma-core/src/relation.rs` — Relation with RelationType::AbstractionOf, Generalizes
- `crates/dcma-entropy/src/entropy.rs` — Reference for ln_approx (needed for salience scaling)
