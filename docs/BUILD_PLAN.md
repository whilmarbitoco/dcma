# DCMA Build Plan

## Overview

This document outlines the complete plan to implement the Deterministic Cognitive Memory Architecture from the formal specification (`DCMA_FORMAL_SPEC.md`).

**Current state:** Single-commit repo with only two Markdown files (README + formal spec). No code, no config, no build system.

---

## Language: Rust

| Requirement | Why Rust fits |
|---|---|
| Deterministic rational arithmetic | `rust_decimal` — fixed-point, no float |
| Graph operations | `petgraph` — stable, feature-rich graph library |
| Performance-critical (beam search, activation loops) | Zero-cost abstractions, no GC |
| Determinism guaranteed | No undefined behavior, strict type system |
| Distributed protocol | `tokio` async runtime |
| Language bindings | `PyO3` (Python), `wasm-bindgen` (JS) |
| FFI for AI integration | C-compatible ABI, easy to call from any language |

**Alternatives considered and rejected:**
- **Python** — too slow for the core loop, no way to enforce rational arithmetic at the language level, GIL issues
- **Go** — weaker graph ecosystem, no fixed-point decimal in stdlib, harder to enforce determinism
- **C++** — no modern package management, template-heavy for newtypes, harder to maintain

---

## Project Structure

```
dcma/
├── Cargo.toml                  # Workspace root
├── Cargo.lock
├── rust-toolchain.toml         # Pin Rust version
├── .gitignore
├── LICENSE
├── README.md
├── docs/
│   ├── DCMA_FORMAL_SPEC.md     # Existing spec
│   └── BUILD_PLAN.md           # This file
├── crates/
│   ├── dcma-core/              # Core graph types, Atom, Relation, State
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── atom.rs         # CognitiveAtom type
│   │       ├── relation.rs     # Relation/Edge types
│   │       ├── graph.rs        # CognitiveGraph (wraps petgraph)
│   │       ├── state.rs        # Full cognitive state S(t)
│   │       ├── types.rs        # Decimal newtypes, enums, UUID
│   │       └── canonical.rs    # Symbol canonicalization
│   ├── dcma-activation/        # Activation Engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── propagation.rs  # ACTIVATE algorithm
│   │       └── convergence.rs  # Fixed-point detection
│   ├── dcma-retrieval/         # Retrieval Engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── relevance.rs    # R(G_q, S, q) functional
│   │       ├── beam.rs         # Bounded beam search
│   │       └── similarity.rs   # structual_similarity
│   ├── dcma-contradiction/     # Contradiction Engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── pressure.rs     # Pressure dynamics
│   │       └── equilibrium.rs  # Equilibrium classification
│   ├── dcma-entropy/           # Entropy Regulation Engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── entropy.rs      # H(G) Shannon entropy
│   │       └── regulation.rs   # Regulation operations
│   ├── dcma-consolidation/     # Consolidation Engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── motif.rs        # DETECT_MOTIFS algorithm
│   │       └── crystallization.rs  # CRYSTALLIZE algorithm
│   ├── dcma-topology/          # Topology Engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── edge.rs         # Edge creation/stabilization
│   │       └── partition.rs    # Spectral partitioning
│   ├── dcma-runtime/           # Orchestrator — all engines together
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── phi.rs          # Phi — cognitive evolution function
│   │       └── tick.rs         # One full tick: all engines in order
│   └── dcma-distributed/       # Distributed Cognition Protocol
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── hlc.rs          # Hybrid logical clock
│           ├── protocol.rs     # Wire protocol
│           └── merge.rs        # CRDT-style merge
├── bindings/
│   └── python/                 # Python bindings via PyO3
│       ├── Cargo.toml
│       └── src/lib.rs
├── benches/                    # Criterion benchmarks
│   ├── activation.rs
│   ├── retrieval.rs
│   └── consolidation.rs
├── examples/                   # Runnable examples
│   ├── simple_retrieval.rs
│   ├── contradiction_demo.rs
│   └── multi_agent.rs
└── tests/                      # Integration tests
    ├── determinism.rs          # Given same S(t)+I(t), same S(t+1)?
    ├── convergence.rs          # Activation converges to fixed point
    └── retrieval_accuracy.rs   # Precision/recall on known graphs
```

---

## Dependencies

| Crate | Purpose |
|---|---|
| `petgraph` | Graph primitives (StableGraph, visit, algorithms) |
| `rust_decimal` | Fixed-point rational arithmetic (matches spec Q constraint) |
| `uuid` | Cognitive Atom IDs |
| `serde` / `serde_json` | State serialization |
| `proptest` | Property-based testing (critical for determinism) |
| `criterion` | Benchmarks |
| `thiserror` | Error types |
| `tracing` | Observability per tick |
| `tokio` | Async runtime (distributed protocol) |
| `PyO3` / `maturin` | Python bindings |

---

## Implementation Phases

### Phase 0 — Scaffolding (1–2 days)
- `cargo init` workspace, set up `rust-toolchain.toml`
- Create empty crates with proper dependencies
- `.gitignore` (target/, *.pyc, etc.)
- Add `criterion`, `proptest` dev-dependencies
- Verify `cargo build`, `cargo test`, `cargo bench` all pass

### Phase 1 — `dcma-core` (1 week)
- `Decimal` newtypes: `Activation`, `Weight`, `Confidence`, `Salience`, `Pressure`
- `AtomId` (UUID v4 wrapper), `AtomType` enum, `AtomState` enum
- `RelationType` enum with full taxonomy (causal, structural, logical, temporal, cognitive)
- `CognitiveAtom` struct
- `Relation` struct with typed edges
- `CognitiveGraph` — wraps `petgraph::StableGraph` with typed ops
- `CognitiveState` — the full S(t) tuple
- Symbol canonicalization pipeline (lowercase, unicode normalize, alias resolution)
- Identity merge algorithm
- **Milestone:** Can create atoms, connect them with relations, serialize/deserialize full state

### Phase 2 — `dcma-activation` (1 week)
- Activation vector A(t) as `Vec<Decimal>`
- Normalized weight computation
- R_i(t) reinforcement, D_i(t) decay, C_i(t) contradiction suppression, Q_i(t) contextual charge
- Sigmoid via piecewise rational approximation (spec requires error < 1e-10)
- `ACTIVATE` algorithm with convergence check (Banach fixed-point theorem)
- Spontaneous activation (Bernoulli with p_spont)
- **Milestone:** Activation converges to fixed point, matches spec Theorem 3.2

### Phase 3 — `dcma-retrieval` (1 week)
- Structural similarity (Jaccard over k-hop neighborhoods)
- Relevance functional R(G_q, S, q) with 5 terms
- Bounded beam search
- Subgraph constraints (d_max, n_max)
- Contradiction-modified relevance R_contrad
- **Milestone:** Given query nodes, reconstructs optimal cognitive field subgraph

### Phase 4 — `dcma-contradiction` (3–4 days)
- Contradiction structure C(t)
- Pressure dynamics equation
- Equilibrium classification (stable, dominant, oscillating, resolved)
- Threshold-based state transitions
- **Milestone:** Contradictions stabilize to correct equilibrium class per spec

### Phase 5 — `dcma-entropy` (2–3 days)
- Shannon entropy over degree distribution
- Bounds checking (H_min, H_max)
- Regulation operations (abstraction promotion, redundancy collapse, bridging, hub splitting, dead-path elimination)
- **Milestone:** Graph entropy stays within operational bounds

### Phase 6 — `dcma-consolidation` (1 week)
- Bounded VF2-like subgraph isomorphism for motif detection
- Crystallization criteria (recurrence >= threshold, stability >= threshold)
- `CRYSTALLIZE` algorithm — creates abstraction nodes
- Hierarchical abstraction (up to max_depth levels)
- **Milestone:** Recurring motifs crystallize into abstraction nodes

### Phase 7 — `dcma-topology` (2–3 days)
- Edge creation with initial weight
- Weight evolution (reinforce, decay, contradiction)
- Relation stabilization (stability score over window)
- Spectral partitioning (for distributed deployment)
- **Milestone:** Edge weights evolve correctly under all three forces

### Phase 8 — `dcma-runtime` (3–4 days)
- Phi — cognitive evolution function composing all engines
- Tick execution order: Activation -> Contradiction -> Entropy -> Topology -> Consolidation
- Configurable engine enable/disable
- Determinism integration test: same input -> same output
- **Milestone:** Full tick executes deterministically, passes spec Section 10 proof

### Phase 9 — `dcma-distributed` (1 week)
- Hybrid Logical Clock (HLC)
- Wire protocol (gRPC or custom TCP)
- CRDT merge: w_merged = max(w_1, w_2)
- Partition healing
- **Milestone:** Two agent processes can share a cognitive substrate

### Phase 10 — Bindings, Benchmarks, Examples (1 week)
- Python bindings via PyO3 (for AI/LLM integration)
- Criterion benchmarks for all operations
- Example: RAG-like retrieval query
- Example: Multi-agent shared cognition demo
- Documentation with `cargo doc`
- **Milestone:** `pip install dcma` works; benchmarks match spec complexity bounds

---

## Testing Strategy

Testing is critical because determinism is DCMA's core promise.

| Test Type | Tool | What it validates |
|---|---|---|
| Property-based | `proptest` | Given any valid S(t)+I(t), Phi produces identical S(t+1) across runs |
| Convergence | Unit | Activation reaches fixed point within spec iteration bound |
| Equilibrium | Unit | Contradictions reach correct equilibrium class |
| Determinism | Integration | Serialize random initial state, run 100 ticks, repeat — must be byte-identical |
| Regression | Snapshot | Known input graphs produce known output states |
| Benchmark | `criterion` | Complexity matches spec: O(T * (|V|+|E|)), O(d_max * b * d_avg * n_max), etc. |

---

## Complexity Targets (from spec)

| Operation | Amortized Target |
|---|---|
| Activation propagation | O(|V| + |E|) per tick |
| Retrieval | O(d_max * b * d_avg * n_max) |
| Full consolidation | O(|V|^s_max) — runs infrequently |
| Contradiction update | O(|C|) |
| Full tick (all engines) | O(|V| + |E|) amortized (consolidation excluded) |
