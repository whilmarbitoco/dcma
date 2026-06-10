# Phase 1a — dcma-core: types, atom, relation

Implement the core types for the DCMA Rust workspace. All arithmetic uses `rust_decimal::Decimal` (no f64 in core).

## Files to create/modify

### 1. `crates/dcma-core/src/types.rs`

Create the following:

**Decimal newtypes** (each is a struct wrapper around `rust_decimal::Decimal` with serde support):
- `Activation` — clamped to [0, 1]
- `Weight` — clamped to [0, 1]
- `Confidence` — clamped to [0, 1]
- `Salience` — non-negative (Q≥0)
- `Pressure` — non-negative (Q≥0)
- `LearningRate` — non-negative (Q≥0)

Each newtype must implement:
- `From<Decimal>` + `Into<Decimal>`
- `Add`, `Sub`, `Mul`, `Div` via `std::ops` (delegating to inner Decimal)
- `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Clone`, `Debug`, `Default`
- `Serialize`, `Deserialize` via serde
- `new(value: Decimal) -> Self` that clamps to valid range
- `get(&self) -> Decimal` that returns inner value
- `ZERO`, `ONE`, `HALF` constants where applicable
- `fn clamp(&self) -> Self` — re-clamp to valid range

**Enums:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AtomType {
    Concept,
    Event,
    Entity,
    Procedure,
    Abstraction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AtomState {
    Active,
    Dormant,
    Frozen,
    Consolidated,
}
```

**RelationType taxonomy** (per formal spec Section 2.2):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelationType {
    // Causal
    Causes, Prevents, Enables, Triggers, DerivesFrom,
    // Structural
    PartOf, InstanceOf, Specializes, Generalizes, Contains,
    // Logical
    Implies, Negates, Supports, Contradicts,
    // Temporal
    Precedes, Follows, CoOccursWith, PersistsWith,
    // Cognitive
    Reinforces, Inhibits, Activates, Suppresses, Focuses,
}
```

**EngineConfig** — all 24 parameters from spec Appendix A:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub eta_d: LearningRate,       // 0.1
    pub eta_r: LearningRate,       // 0.05
    pub eta_c: LearningRate,       // 0.15
    pub eta_q: LearningRate,       // 0.1
    pub lambda: LearningRate,      // 0.9
    pub reinforcement_window: usize, // 10
    pub convergence_threshold: Activation, // 1e-6
    pub max_iterations: usize,     // 100
    pub equilibrium_threshold: Pressure, // 0.05
    pub equilibrium_window: usize, // 100
    pub resolution_threshold: Pressure, // 0.95
    pub crystallization_threshold: usize, // 5
    pub stability_threshold: Activation, // 0.7
    pub entropy_min: Activation,   // 1.5
    pub entropy_max: Activation,   // 5.0
    pub dead_node_threshold: Activation, // 1e-6
    pub dead_node_window: usize,   // 10000
    pub spontaneous_activation_prob: Activation, // 1e-4
    pub max_motif_size: usize,     // 5
    pub max_abstraction_depth: usize, // 5
    pub beam_width: usize,         // 20
    pub max_retrieval_depth: usize, // 4
    pub max_subgraph_size: usize,  // 500
}
```
Implement `Default` for EngineConfig with the values shown above (defaults from spec).

**AtomId** — UUID v4 wrapper:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AtomId(Uuid);
impl AtomId { pub fn new() -> Self } // v4
impl From<Uuid> for AtomId
```

### 2. `crates/dcma-core/src/atom.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveAtom {
    pub id: AtomId,
    pub atom_type: AtomType,
    pub canonical: String,
    pub state: AtomState,
    pub attributes: HashMap<String, String>,
    pub t_created: u64,        // tick created
    pub t_last_accessed: u64,  // tick last accessed
    pub salience: Salience,
    pub confidence: Confidence,
    pub origin: String,        // provenance
    pub reinforcement_history: Vec<u64>,     // ticks when reinforced
    pub activation_history: Vec<Activation>, // last N activation values
    pub contradiction_indices: Vec<usize>,    // indices into contradiction structure
}

impl CognitiveAtom {
    pub fn new(id: AtomId, atom_type: AtomType, canonical: String) -> Self
    pub fn is_active(&self) -> bool
    pub fn mark_dormant(&mut self)
    pub fn record_activation(&mut self, activation: Activation)
    pub fn record_reinforcement(&mut self, tick: u64)
}
```

### 3. `crates/dcma-core/src/relation.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub id: u64,
    pub src: AtomId,
    pub dst: AtomId,
    pub relation_type: RelationType,
    pub weight: Weight,
    pub stability: Confidence,
    pub confidence: Confidence,
    pub t_created: u64,
}

impl Relation {
    pub fn new(id: u64, src: AtomId, dst: AtomId, relation_type: RelationType) -> Self {
        // weight = 0.5, stability = 0.0, confidence = 0.0
    }

    /// Edge weight evolution per spec Section 2.3:
    /// w(t+1) = clamp(w(t) + η_r·r(e,t) - η_d·w(t) - η_c·c(e,t))
    pub fn evolve_weight(&mut self, reinforcement: Decimal, contradiction: Decimal, config: &EngineConfig) {
        let delta_reinforce = config.eta_r.get() * reinforcement;
        let delta_decay = config.eta_d.get() * self.weight.get();
        let delta_contradiction = config.eta_c.get() * contradiction;
        let new_weight = (self.weight.get() + delta_reinforce - delta_decay - delta_contradiction)
            .clamp(Decimal::ZERO, Decimal::ONE);
        self.weight = Weight::new(new_weight);
    }
}
```

### 4. Update `crates/dcma-core/src/lib.rs`

Add:
```rust
pub mod types;
pub mod atom;
pub mod relation;
pub mod graph;
pub mod state;
pub mod canonical;

// Re-export key types for convenience
pub use types::*;
pub use atom::CognitiveAtom;
pub use relation::Relation;
```

### Testing

Add a `#[cfg(test)] mod tests` in `types.rs`, `atom.rs`, and `relation.rs`:

- `types.rs`: Test newtype clamping (values outside range get clamped), arithmetic ops work, ZERO/ONE constants, EngineConfig defaults
- `atom.rs`: Test creation, mark_dormant, record_activation, record_reinforcement
- `relation.rs`: Test new() initializes weight=0.5, evolve_weight increases with reinforcement, decreases with decay and contradiction, clamping at 0 and 1

### Constraints

- NO floating-point arithmetic (f32/f64) anywhere — use `rust_decimal::Decimal` exclusively
- NO `panic!` — use proper error handling
- All newtypes enforce their invariant at construction time via `new()`
- `cargo build` must succeed for the workspace
- `cargo test -p dcma-core` must pass
