# DCMA Phase D — Runtime Orchestrator

## Context

DCMA workspace at `/workspace/dcma`. Phases 1-6 are complete (core, activation, contradiction, entropy, retrieval, consolidation, topology). Phase D implements the **Runtime Orchestrator** — the Φ (Phi) cognitive evolution function and tick loop that composes all 7 engines into a single deterministic cycle.

This is the most critical integration phase. The formal spec is at `docs/DCMA_FORMAL_SPEC.md` (Section 10 for determinism proof). The build plan is at `docs/BUILD_PLAN.md` (Phase 8).

## What to Implement

### 1. `crates/dcma-runtime/src/phi.rs`

Implement the cognitive evolution function Φ(S(t), I(t)):

The tick execution order (per spec Section 10 proof structure):
```
Φ(S(t), I(t)) = Consolidate(Entropy(Topology(Contradiction(Activation(S(t), I(t))))))
```

Detailed tick sequence:
1. **Apply input stimulus I(t)** — set activation of source nodes to 1.0
2. **Activation propagation** — run ACTIVATE algorithm until convergence
3. **Contradiction update** — update pressure for all Contradicts edges, classify equilibrium
4. **Entropy check** — compute H(G), apply regulation if out of bounds
5. **Topology evolution** — evolve edge weights, update relation stability
6. **Consolidation** — run every T_consolidation ticks (default 100), not every tick
7. **Advance tick counter** — increment, update timestamps

Import from all engine crates:
- `dcma_activation::{activate, spontaneous_activation, ActivationVector}`
- `dcma_contradiction::{ContradictionStructure, EquilibriumState}`
- `dcma_entropy::{graph_entropy, EntropyRegulator}`
- `dcma_retrieval::{retrieve, CognitiveField}`
- `dcma_consolidation::{consolidate, detect_motifs}`
- `dcma_topology::{evolve_weights, stabilize_relations}`

Key types:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickResult {
    pub tick: u64,
    pub convergence: ConvergenceInfo,
    pub entropy_status: EntropyStatus,
    pub regulation_applied: bool,
    pub consolidation_applied: bool,
    pub consolidation_report: Option<CrystallizationReport>,
    pub weight_changes: WeightChangeReport,
    pub stabilization: StabilizationReport,
    pub activations: HashMap<AtomId, Activation>,
}

#[derive(Debug, Clone)]
pub struct Stimulus {
    pub source_nodes: Vec<AtomId>,
    pub query_context: Vec<AtomId>,
    pub label: String,
}
```

### 2. `crates/dcma-runtime/src/tick.rs`

Implement tick execution:

```rust
pub struct Runtime {
    config: EngineConfig,
    tick_count: u64,
    state: CognitiveState,
    contradiction_structure: ContradictionStructure,
    entropy_regulator: EntropyRegulator,
}

impl Runtime {
    pub fn new(config: EngineConfig) -> Self { ... }
    pub fn with_state(config: EngineConfig, state: CognitiveState) -> Self { ... }
    
    /// Execute one full cognitive tick
    pub fn tick(&mut self, stimulus: &Stimulus) -> TickResult { ... }
    
    /// Execute multiple ticks
    pub fn run(&mut self, stimuli: &[Stimulus]) -> Vec<TickResult> { ... }
    
    /// Get current state
    pub fn state(&self) -> &CognitiveState { ... }
    
    /// Get current tick count
    pub fn tick_count(&self) -> u64 { ... }
    
    /// Get activation for a specific atom
    pub fn get_activation(&self, id: AtomId) -> Activation { ... }
    
    /// Get all activations
    pub fn activations(&self) -> HashMap<AtomId, Activation> { ... }
}
```

### 3. `crates/dcma-runtime/src/lib.rs`

Public API:
```rust
pub use phi::{TickResult, Stimulus};
pub use tick::Runtime;
pub use dcma_core::state::CognitiveState;
pub use dcma_core::types::EngineConfig;
```

### 4. `crates/dcma-runtime/Cargo.toml`

```toml
[package]
name = "dcma-runtime"
version = "0.1.0"
edition = "2021"

[dependencies]
dcma-core = { path = "../dcma-core" }
dcma-activation = { path = "../dcma-activation" }
dcma-retrieval = { path = "../dcma-retrieval" }
dcma-contradiction = { path = "../dcma-contradiction" }
dcma-entropy = { path = "../dcma-entropy" }
dcma-consolidation = { path = "../dcma-consolidation" }
dcma-topology = { path = "../dcma-topology" }
rust_decimal = "1"
serde = { version = "1", features = ["derive"] }
thiserror = "1"
tracing = "0.1"
```

## Testing Requirements (Integration Tests)

Create `tests/` directory in the workspace root for integration tests:

### `tests/determinism.rs`
1. **test_tick_determinism** — Create state, run 50 ticks with same stimulus, serialize result. Create identical state, run again. Results must be byte-identical.
2. **test_tick_determinism_across_runs** — Save state to JSON, load in a new Runtime, continue ticking. Results must match original.

### `tests/convergence.rs`
3. **test_activation_converges** — After tick, activation values have converged (max diff < ε)
4. **test_convergence_within_bound** — Convergence happens within spec iteration bound (≤80 iterations for typical params)

### `tests/entropy.rs`
5. **test_entropy_stays_in_bounds** — After 100 ticks, entropy remains within [H_min, H_max]
6. **test_entropy_regulation_triggers** — Create graph with extreme entropy, verify regulation brings it in bounds

### `tests/retrieval.rs`
7. **test_retrieval_after_tick** — Insert atoms, run tick, query retrieval returns relevant results
8. **test_retrieval_empty_state** — Retrieval on empty graph returns empty field

### `tests/contradiction.rs`
9. **test_contradiction_pressure_updates** — After tick, contradiction pressures have been updated
10. **test_contradiction_equilibrium** — Contradictions reach correct equilibrium class

### `tests/consolidation.rs`
11. **test_consolidation_periodic** — Consolidation runs every T_consolidation ticks, not every tick
12. **test_motifs_crystallize** — Create graph with recurring motifs, run enough ticks, verify ABSTRACTION nodes created

### `tests/full_pipeline.rs`
13. **test_end_to_end_100_ticks** — Full pipeline: insert 20 atoms, 15 relations, 2 contradictions, run 100 ticks, verify:
    - All activations converged
    - Entropy in bounds
    - Contradictions classified
    - State serializes/deserializes correctly

## Important Constraints

- **DETERMINISM IS CRITICAL** — This is DCMA's core promise. The integration tests must verify it.
- Use `rust_decimal::Decimal` for all arithmetic
- No randomness in the core tick loop (spontaneous activation uses deterministic hash)
- Follow existing code style
- Run `cargo build` and `cargo test` after implementation
- Fix ALL compiler warnings across the entire workspace

## Existing Code Reference

- `crates/dcma-activation/src/propagation.rs` — `activate()`, `ActivationVector`, `EngineConfig`
- `crates/dcma-contradiction/src/pressure.rs` — `ContradictionStructure::update_all()`, `classify()`
- `crates/dcma-entropy/src/entropy.rs` — `graph_entropy()`, `EntropyRegulator::regulate()`
- `crates/dcma-retrieval/src/beam.rs` — `retrieve()`, `CognitiveField`
- `crates/dcma-consolidation/src/crystallization.rs` — `consolidate()`, `detect_motifs()`
- `crates/dcma-topology/src/edge.rs` — `evolve_weights()`, `stabilize_relations()`
- `crates/dcma-core/src/state.rs` — `CognitiveState` with full graph + metadata
- `crates/dcma-core/src/types.rs` — `EngineConfig` with all parameters
