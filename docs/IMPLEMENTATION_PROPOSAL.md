# DCMA Implementation Proposal

**Date:** June 11, 2026
**Author:** Ciel (on behalf of Whilmar M. Bitoco)
**Status:** Draft — pending review

---

## 1. Current State Assessment

### What Exists

DCMA has a complete formal specification (v1.0) and 9 Rust crates that compile cleanly. The architecture is sound: `petgraph` for graph ops, `rust_decimal` for deterministic rational arithmetic, `serde` for serialization. Five implementation phases have been committed covering core types, activation dynamics, contradiction, entropy, and retrieval scaffolding.

### Gap Analysis

| Area | Status | Gap Severity |
|------|--------|-------------|
| Core types (atom, relation, graph, state) | ✅ Implemented + unit tests | Low |
| Activation engine (propagation, convergence) | ✅ Implemented + unit tests | Low |
| Contradiction engine (pressure, equilibrium) | ✅ Implemented + unit tests | Low |
| Entropy regulation | ✅ Implemented + unit tests | Low |
| Retrieval engine (beam search, relevance) | ⚠️ Scaffolded only — `lib.rs` and `beam.rs` are empty | **High** |
| Consolidation engine (motif, crystallization) | ⚠️ Scaffolded only — `lib.rs` and `motif.rs` are empty | **High** |
| Topology engine (edge, partition) | ⚠️ Scaffolded only — `lib.rs` and `edge.rs` are empty | **Medium** |
| Runtime orchestrator (Φ, tick) | ⚠️ Scaffolded only — `phi.rs` and `tick.rs` are empty | **Critical** |
| Distributed protocol | ⚠️ Module declarations only — no implementation | **High** |
| Persistence / storage | ❌ None | **Critical** |
| API / server | ❌ None | **High** |
| CLI | ❌ None | **Medium** |
| Integration tests | ❌ None | **High** |
| Property-based tests (proptest) | ❌ None | **High** |
| Benchmarks (criterion) | ❌ None | **Medium** |
| Examples | ❌ None | **Medium** |
| Python bindings (PyO3) | ❌ None | **Low** (deferred) |

### Summary

The project has **~30% of the implementation done** — the foundational types and 3 of 7 engines are solid. The remaining 4 engines are empty stubs, the runtime orchestrator (the most critical piece) is empty, and there is no way to persist, interact with, or observe the system.

---

## 2. Proposed Implementation Plan

The plan is organized into 6 phases, ordered by dependency and impact. Each phase is self-contained and produces a verifiable milestone.

---

### Phase A — Retrieval Engine (Highest Priority)

**Why first:** Retrieval is the primary interface for getting information *out* of DCMA. Without it, the system has no observable behavior. The activation, contradiction, and entropy engines are already done — retrieval is the first missing piece that prevents end-to-end operation.

**What to implement:**

1. **`dcma-retrieval/src/relevance.rs`** — The relevance functional R(G_q, S, q) with 5 terms:
   - Contextual coherence C(G_q, q) — Jaccard structural similarity over k-hop neighborhoods
   - Salience Σ(G_q) — average node salience
   - Activation resonance A(G_q) — average current activation
   - Temporal relevance T(G_q) — exponential decay from last access
   - Traversal cost D(G_q) — diameter ratio penalty
   - Coefficient constraints: α + β + γ + δ + λ = 1

2. **`dcma-retrieval/src/similarity.rs`** — Structural similarity:
   - `sim_struct(v, q)` = Jaccard(N_k(v), N_k(q)) × weight(path(v,q)) / diameter(G)
   - k-hop neighborhood extraction (reuse `CognitiveGraph::k_hop_neighbors`)
   - Path strength computation

3. **`dcma-retrieval/src/beam.rs`** — Bounded beam search:
   - Initialize frontier with query nodes
   - Expand by d_max hops, pruning to beam_width candidates by relevance
   - Enforce n_max subgraph size constraint
   - Deterministic tie-breaking by AtomId lexicographic order
   - Return activated cognitive field (subgraph + per-node activation values)

4. **`dcma-retrieval/src/lib.rs`** — Public API:
   - `retrieve(graph, query_nodes, config) -> CognitiveField`
   - `CognitiveField` struct with subgraph reference, relevance score, metadata

**Tests:**
- Unit: relevance functional returns correct value for known graph
- Unit: beam search finds optimal subgraph within bounded depth
- Unit: deterministic — same query on same graph produces same result
- Unit: empty query returns empty field
- Unit: contradiction-modified relevance deprioritizes contradicted nodes

**Estimated effort:** 3–4 days

---

### Phase B — Consolidation Engine (Second Priority)

**Why second:** Consolidation is the learning mechanism — it's how DCMA accumulates intelligence over time. Without it, the graph never grows beyond what's explicitly inserted. This is the "abstraction is compression" law in action.

**What to implement:**

1. **`dcma-consolidation/src/motif.rs`** — Motif detection:
   - Bounded VF2-like subgraph isomorphism
   - Enumerate connected subgraphs of size 2..=s_max (default 5)
   - Group by isomorphism class (canonical form)
   - Count occurrences per pattern
   - Filter by min_recurrence (default 5)

2. **`dcma-consolidation/src/crystallization.rs`** — Crystallization:
   - Stability score: 1 - var(weights) / (mean(weights) + ε)
   - Crystallize when recurrence ≥ θ_cryst AND stability ≥ θ_stab
   - Create ABSTRACTION node with edges to participating nodes
   - Salience scales with log(recurrence)
   - Hierarchical: abstraction nodes can participate in higher-level motifs (up to max_depth = 5)

3. **`dcma-consolidation/src/lib.rs`** — Public API:
   - `detect_motifs(graph, config) -> Vec<Motif>`
   - `crystallize_motifs(graph, motifs, config) -> Vec<AtomId>` (returns new abstraction IDs)
   - `consolidate(graph, config) -> ConsolidationReport`

**Tests:**
- Unit: detects a triangle motif repeated 5+ times
- Unit: crystallizes only when both recurrence and stability thresholds met
- Unit: abstraction node has correct edges and salience
- Unit: hierarchical — level-1 abstraction participates in level-2 motif
- Unit: no false positives on random graphs

**Estimated effort:** 4–5 days (motif detection is algorithmically complex)

---

### Phase C — Topology Engine (Third Priority)

**Why third:** Topology handles edge weight evolution and graph maintenance. It's needed before the runtime can properly execute ticks, since edge weights change over time via reinforcement, decay, and contradiction.

**What to implement:**

1. **`dcma-topology/src/edge.rs`** — Edge weight evolution:
   - `w(e, t+1) = clamp(w(e,t) + Δw_reinforce - Δw_decay + Δw_contradiction)`
   - Reinforcement increment: η_r × r(e,t)
   - Proportional decay: η_d × w(e,t)
   - Contradiction suppression: -η_c × c(e,t)
   - Relation stabilization score over window T_s
   - Hub splitting (partition node with degree > 5× average)

2. **`dcma-topology/src/partition.rs`** — Spectral partitioning:
   - Graph Laplacian construction from StableGraph
   - Spectral clustering for distributed deployment
   - Minimize inter-partition edges, balance partition sizes

3. **`dcma-topology/src/lib.rs`** — Public API:
   - `evolve_weights(graph, config, tick) -> WeightChangeReport`
   - `stabilize_relations(graph, config) -> Vec<EdgeIndex>` (returns newly stabilized edges)
   - `partition(graph, k) -> Vec<Partition>`

**Tests:**
- Unit: weight increases with reinforcement, decreases with decay
- Unit: contradiction reduces weight
- Unit: stability score is 1.0 for constant weights, <1.0 for varying
- Unit: hub splitting creates sub-nodes with correct edges
- Unit: partitioning produces k balanced partitions

**Estimated effort:** 2–3 days

---

### Phase D — Runtime Orchestrator (Critical Integration)

**Why now:** With all 7 engines implemented, the runtime is what makes DCMA *run*. This is the Φ function — the cognitive evolution function that composes all engines into a single deterministic tick.

**What to implement:**

1. **`dcma-runtime/src/phi.rs`** — The cognitive evolution function:
   ```
   Φ(S(t), I(t)) = Consolidate(Entropy(Topology(Contradiction(Activation(S(t), I(t))))))
   ```
   - Tick execution order (per spec):
     1. Apply input stimulus I(t) — activate source nodes
     2. Activation propagation (converge to fixed point)
     3. Contradiction pressure update + equilibrium classification
     4. Entropy check + regulation if out of bounds
     5. Topology weight evolution + relation stabilization
     6. Consolidation (every T_consolidation ticks, not every tick)
   - Deterministic: no randomness in the core loop (spontaneous activation uses deterministic hash)

2. **`dcma-runtime/src/tick.rs`** — Tick execution:
   - `TickResult` with full state snapshot, convergence info, regulation report, consolidation report
   - Configurable engine enable/disable for testing
   - Tick counter + timestamp

3. **`dcma-runtime/src/lib.rs`** — Public API:
   - `Runtime::new(config) -> Self`
   - `runtime.tick(state, input) -> (CognitiveState, TickResult)`
   - `runtime.run(state, inputs, n_ticks) -> Vec<(CognitiveState, TickResult)>`

**Tests (integration):**
- Full tick determinism: same S(t) + I(t) → same S(t+1), verified over 100 ticks
- Convergence: activation reaches fixed point within spec iteration bound
- Entropy stays in bounds after regulation
- Contradictions reach correct equilibrium class
- Serialization round-trip: state → JSON → deserialize → byte-identical

**Estimated effort:** 3–4 days

---

### Phase E — Persistence + API + CLI (Usability)

**Why now:** DCMA needs to survive restarts and be interactable. This phase makes it a usable system rather than a library.

**What to implement:**

1. **Persistence layer** (new crate: `dcma-storage`):
   - `serde` already implemented on `CognitiveGraph` and `CognitiveState`
   - File-backed storage: save/load state as JSON or binary (bincode for speed)
   - Write-ahead log (WAL) for crash safety: append each tick's state diff to a log file
   - Recovery: replay WAL from last checkpoint
   - Checkpointing: periodic full-state snapshots (every N ticks)

2. **HTTP API** (new crate: `dcma-api` using `axum`):
   - `POST /atoms` — insert atom
   - `POST /relations` — create relation
   - `POST /query` — retrieval (returns cognitive field)
   - `POST /tick` — execute one tick with input stimulus
   - `GET /state` — current cognitive state summary
   - `GET /atoms/:id` — get atom details
   - `GET /stats` — graph stats (node count, edge count, entropy, etc.)

3. **CLI** (new crate: `dcma-cli` using `clap`):
   - `dcma init` — initialize new cognitive state
   - `dcma insert --type CONCEPT --name "..."` — add atom
   - `dcma connect --src <id> --dst <id> --type CAUSES` — add relation
   - `dcma query --nodes <id1,id2>` — retrieval
   - `dcma tick` — execute one cognitive tick
   - `dcma serve` — start HTTP API server
   - `dcma stats` — show graph statistics
   - `dcma export --format json` — export state

**Tests:**
- Storage: save → load → byte-identical state
- WAL: crash mid-tick → recover → no data loss
- API: all endpoints return correct responses (integration tests with test server)
- CLI: all commands produce expected output

**Estimated effort:** 5–6 days

---

### Phase F — Distributed Protocol (Advanced)

**Why last:** Distributed cognition is the final piece from the spec. It depends on everything else working correctly first.

**What to implement:**

1. **`dcma-distributed/src/hlc.rs`** — Hybrid Logical Clock:
   - Combines physical time with logical counter
   - `Timestamp { physical: u64, logical: u64, node_id: AtomId }`
   - Happens-before relation
   - Monotonic, causal ordering

2. **`dcma-distributed/src/merge.rs`** — CRDT-style merge:
   - Edge weight merge: `w_merged = max(w1, w2)`
   - Atom merge: union of attributes, max confidence
   - Conflict-free: merge is commutative, associative, idempotent

3. **`dcma-distributed/src/protocol.rs`** — Wire protocol:
   - Message types: `TickSync`, `StateFragment`, `MergeRequest`, `Heartbeat`
   - TCP-based transport (or gRPC if preferred)
   - Partition healing: detect split, exchange state diffs, merge

4. **`dcma-distributed/src/lib.rs`** — Public API:
   - `Node::new(config, node_id) -> Self`
   - `node.connect(peer_addr)`
   - `node.broadcast_tick(result)`
   - `node.receive_merge() -> Option<MergeRequest>`

**Tests:**
- Unit: HLC ordering is correct (happens-before)
- Unit: CRDT merge is commutative and associative
- Integration: two nodes share state, execute ticks independently, merge → identical state
- Integration: partition heals correctly after reconnection

**Estimated effort:** 5–7 days

---

## 3. Cross-Cutting Concerns

### Testing Strategy (applies to all phases)

| Test Type | Tool | When |
|-----------|------|------|
| Unit tests | Built-in `#[test]` | Every crate, every phase |
| Property-based | `proptest` | Phase D+ — determinism, convergence, merge |
| Integration tests | `tests/` directory | Phase D (runtime), Phase E (API/storage) |
| Snapshot tests | `insta` | Phase D — known input/output states |
| Benchmarks | `criterion` | After Phase D — validate complexity bounds |

### Determinism Verification (critical)

DCMA's core promise is determinism. This must be verified at every phase boundary:

1. **Within a crate:** unit tests with fixed inputs
2. **Across crates:** integration tests that run Φ and verify output
3. **Across serialization:** state → serialize → deserialize → run tick → same result
4. **Across restarts:** Phase E persistence — save → load → continue ticking → same trajectory

### Warning Cleanup

7 compiler warnings exist (unused variables in entropy, activation, contradiction). Fix as part of Phase D when touching those crates.

---

## 4. Dependency Additions

| Phase | New Dependencies | Crate |
|-------|-----------------|-------|
| A (Retrieval) | None | — |
| B (Consolidation) | None | — |
| C (Topology) | `nalgebra` or `ndarray` (for Laplacian eigen decomposition) | `dcma-topology` |
| D (Runtime) | `tracing` (observability) | `dcma-runtime` |
| E (Persistence) | `bincode` (binary serialization) | `dcma-storage` |
| E (API) | `axum`, `tower` | `dcma-api` |
| E (CLI) | `clap` | `dcma-cli` |
| F (Distributed) | `tokio` (async runtime) | `dcma-distributed` |
| All (Testing) | `proptest`, `criterion`, `insta` | dev-dependencies |

---

## 5. Estimated Timeline

| Phase | Duration | Cumulative | Deliverable |
|-------|----------|------------|-------------|
| A — Retrieval | 3–4 days | Week 1 | Working retrieval engine with tests |
| B — Consolidation | 4–5 days | Week 2 | Motif detection + crystallization |
| C — Topology | 2–3 days | Week 2–3 | Edge evolution + partitioning |
| D — Runtime | 3–4 days | Week 3 | Full deterministic tick loop |
| E — Persistence + API + CLI | 5–6 days | Week 4–5 | Usable system with storage and HTTP API |
| F — Distributed | 5–7 days | Week 6–7 | Multi-node shared cognition |

**Total: ~22–29 days of focused implementation**

This can be parallelized — Phases B and C can run concurrently after Phase A, and Phase F is fully independent of E.

---

## 6. Risks and Mitigations

| Risk | Severity | Mitigation |
|------|----------|-----------|
| Motif detection (VF2) is O(|V|^5) worst case | High | Bound s_max to 5, add early termination, benchmark early |
| `rust_decimal` performance for large graphs | Medium | Benchmark in Phase D; switch to `rational` or fixed-point i128 if needed |
| Beam search determinism (tie-breaking) | Medium | Strict lexicographic AtomId ordering, tested explicitly |
| Spectral partitioning dependency (`nalgebra`) | Low | Can use power iteration instead of full eigen decomposition |
| WAL replay performance for long-running instances | Medium | Periodic checkpointing + incremental WAL compaction |

---

## 7. Immediate Next Steps

1. **Approve this proposal** — sir reviews and confirms direction
2. **Start Phase A** — retrieval engine implementation via OpenCode
3. **Set up proptest** — add property-based testing infrastructure to workspace
4. **Add `tracing` to workspace** — observability from the start

---

*This proposal is a living document. It should be updated as implementation reveals new constraints or opportunities.*
