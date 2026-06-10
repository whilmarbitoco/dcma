# Phase 1b — dcma-core: graph, state, canonical

Continue building dcma-core. Depends on Phase 1a (types, atom, relation) being complete.

## Files to create/modify

### 1. `crates/dcma-core/src/graph.rs`

Implement `CognitiveGraph` — a typed wrapper over `petgraph::StableGraph`.

```rust
use petgraph::stable_graph::{StableGraph, NodeIndex, EdgeIndex};
use petgraph::visit::{VisitMap, Visitable};
use petgraph::Direction;
use std::collections::HashMap;
use rust_decimal::Decimal;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveGraph {
    graph: StableGraph<CognitiveAtom, Relation>,
    id_map: HashMap<AtomId, NodeIndex>,  // fast lookup by AtomId
    next_relation_id: u64,
}

impl CognitiveGraph {
    pub fn new() -> Self

    /// Insert a cognitive atom. Returns its NodeIndex.
    pub fn add_atom(&mut self, atom: CognitiveAtom) -> NodeIndex

    /// Remove an atom and all its edges. Returns the atom if found.
    pub fn remove_atom(&mut self, id: AtomId) -> Option<CognitiveAtom>

    /// Get atom by ID
    pub fn get_atom(&self, id: AtomId) -> Option<&CognitiveAtom>
    pub fn get_atom_mut(&mut self, id: AtomId) -> Option<&mut CognitiveAtom>

    /// Create a typed edge between two atoms. Returns the edge NodeIndex.
    pub fn add_relation(&mut self, src: AtomId, dst: AtomId, rel_type: RelationType)
        -> Result<EdgeIndex, GraphError>
    pub fn add_relation_with_weight(&mut self, src: AtomId, dst: AtomId,
        rel_type: RelationType, weight: Weight) -> Result<EdgeIndex, GraphError>

    /// Get relation by edge index
    pub fn get_relation(&self, edge: EdgeIndex) -> Option<&Relation>
    pub fn get_relation_mut(&mut self, edge: EdgeIndex) -> Option<&mut Relation>

    /// Remove an edge
    pub fn remove_relation(&mut self, src: AtomId, dst: AtomId, rel_type: RelationType) -> bool

    /// Edges between two specific atoms
    pub fn relations_between(&self, src: AtomId, dst: AtomId) -> Vec<(EdgeIndex, &Relation)>

    /// All atoms
    pub fn atoms(&self) -> impl Iterator<Item = (NodeIndex, &CognitiveAtom)>
    pub fn atom_count(&self) -> usize

    /// All relations (edges)
    pub fn relations(&self) -> impl Iterator<Item = (EdgeIndex, &Relation)>
    pub fn relation_count(&self) -> usize

    /// Get all node indices
    pub fn node_indices(&self) -> Vec<NodeIndex>

    /// Get node index by AtomId
    pub fn node_index(&self, id: AtomId) -> Option<NodeIndex>

    /// Inbound neighbors of a node
    pub fn inbound_neighbors(&self, id: AtomId) -> Vec<(NodeIndex, &CognitiveAtom, &Relation)>

    /// Outbound neighbors of a node
    pub fn outbound_neighbors(&self, id: AtomId) -> Vec<(NodeIndex, &CognitiveAtom, &Relation)>

    /// All neighbors (inbound + outbound)
    pub fn neighbors(&self, id: AtomId) -> Vec<(NodeIndex, &CognitiveAtom)>

    /// k-hop neighborhood: all nodes within k hops from the given atom
    pub fn k_hop_neighbors(&self, id: AtomId, k: usize) -> HashMap<NodeIndex, usize>
        // returns map of NodeIndex -> hop distance

    /// Degree of a node (in + out)
    pub fn degree(&self, id: AtomId) -> usize

    /// In-degree of a node
    pub fn in_degree(&self, id: AtomId) -> usize

    /// Out-degree of a node
    pub fn out_degree(&self, id: AtomId) -> usize

    /// Degree distribution: maps degree -> count of nodes with that degree
    pub fn degree_distribution(&self) -> HashMap<usize, usize>

    /// Diameter approximation (longest shortest path via BFS from each node)
    /// For large graphs, returns None (use exact_diameter_small for graphs < 1000 nodes)
    pub fn diameter(&self) -> Option<usize>

    /// Get edge weight as Decimal
    pub fn edge_weight(&self, edge: EdgeIndex) -> Option<Decimal>

    /// Update edge weight
    pub fn update_edge_weight(&mut self, edge: EdgeIndex, weight: Weight) -> Result<(), GraphError>

    /// Extract subgraph: given a set of node indices, return a new CognitiveGraph
    /// containing only those nodes and the edges between them
    pub fn subgraph(&self, nodes: &[NodeIndex]) -> CognitiveGraph

    /// Get all Contradicts edges
    pub fn contradiction_edges(&self) -> Vec<(AtomId, AtomId, Decimal)>

    /// Total inbound weight for a node (sum of all incoming edge weights)
    pub fn total_inbound_weight(&self, id: AtomId) -> Decimal
}

#[derive(Debug, thiserror::Error)]
pub enum GraphError {
    #[error("Source atom not found: {0:?}")]
    SourceNotFound(AtomId),
    #[error("Destination atom not found: {0:?}")]
    DestNotFound(AtomId),
    #[error("Edge not found")]
    EdgeNotFound,
    #[error("Node not found: {0:?}")]
    NodeNotFound(AtomId),
}
```

#### Implementation Notes

- `k_hop_neighbors` should use BFS. Track visited set with distances. Stop at k hops. Return map of `{NodeIndex: distance}`.
- `subgraph`: iterate edges in original graph, keep those where both endpoints are in the node set. Build new CognitiveGraph from those atoms and relations.
- `diameter`: BFS from each node, track max shortest path. O(V*(V+E)). Only run if node count < 1000.
- Thread safety: `CognitiveGraph` does NOT need to be Send+Sync (single-tick access pattern).

#### Tests for graph.rs

- `test_add_and_get_atom` — add atom, retrieve by ID
- `test_add_relation` — connect two atoms with Causes, retrieve edge
- `test_remove_atom` — atom and its edges are removed
- `test_k_hop_neighbors` — linear chain of 5 nodes, k=2 from one end returns 3 nodes
- `test_degree_distribution` — star graph (1 center + 4 leaves)
- `test_subgraph` — create 6-node graph, extract 3-node subgraph
- `test_contradiction_edges` — add Contradicts relation, verify it's returned
- `test_edge_weight_evolution` — add edge, evolve weight, verify new value

---

### 2. `crates/dcma-core/src/state.rs`

Implement the full cognitive state `S(t)`.

```rust
use serde::{Serialize, Deserialize};
use rust_decimal::Decimal;
use std::collections::HashMap;

/// Full cognitive state S(t) = ⟨V(t), E(t), W(t), A(t), C(t), H(t)⟩
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveState {
    pub graph: CognitiveGraph,
    /// Activation vector: node_index -> activation value
    pub activations: HashMap<NodeIndex, Activation>,
    /// Contradiction structure: (atom_a_id, atom_b_id) -> pressure
    pub contradictions: HashMap<(AtomId, AtomId), Pressure>,
    /// Tick counter
    pub tick: u64,
    /// State transition history (last N ticks, stored as tick numbers)
    pub history: Vec<u64>,
    /// Engine configuration
    pub config: EngineConfig,
}

impl CognitiveState {
    pub fn new(config: EngineConfig) -> Self

    /// Get activation for an atom by ID
    pub fn get_activation(&self, id: AtomId) -> Option<Activation>

    /// Set activation for an atom by ID
    pub fn set_activation(&mut self, id: AtomId, activation: Activation)

    /// Get all activations as a vector (for iteration)
    pub fn activation_values(&self) -> Vec<(AtomId, Activation)>

    /// Advance tick counter
    pub fn advance_tick(&mut self)

    /// Record current tick in history
    pub fn record_history(&mut self)

    /// Set contradiction pressure between two atoms
    pub fn set_contradiction(&mut self, a: AtomId, b: AtomId, pressure: Pressure)

    /// Get contradiction pressure
    pub fn get_contradiction(&self, a: AtomId, b: AtomId) -> Option<Pressure>

    /// Serialize state to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error>

    /// Deserialize state from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error>

    /// Validate state invariants (all activations in [0,1], all weights in [0,1], etc.)
    pub fn validate(&self) -> Result<(), StateError>
}

#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error("Activation out of range for atom {0:?}: {1}")]
    InvalidActivation(AtomId, Decimal),
    #[error("Weight out of range for edge")]
    InvalidWeight,
}
```

#### Tests for state.rs

- `test_new_state` — create with default config, tick=0
- `test_activation_roundtrip` — set and get activation
- `test_contradiction_roundtrip` — set and get pressure
- `test_tick_advance` — tick increments
- `test_json_roundtrip` — serialize then deserialize, compare
- `test_validate_valid` — fresh state passes validation
- `test_validate_invalid_activation` — activation > 1.0 fails validation

---

### 3. `crates/dcma-core/src/canonical.rs`

Implement the symbol canonicalization pipeline (spec Section 8).

```rust
use unicode_normalization::UnicodeNormalization;

/// Canonicalize a raw symbol string per the pipeline:
/// 1. Lowercase
/// 2. Unicode normalization (NFC)
/// 3. Trim whitespace
/// 4. Collapse internal whitespace to single spaces
pub fn canonicalize(raw: &str) -> String {
    // Step 1: lowercase
    let s = raw.to_lowercase();
    // Step 2: Unicode NFC normalization
    let s = s.nfc().collect::<String>();
    // Step 3+4: trim and collapse whitespace
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    s
}

/// Identity merge (spec Section 8.2):
/// Merge source atom into destination atom.
/// - Edges from source are moved to dest (max weight if duplicate)
/// - Salience: max of both
/// - Reinforcement histories: merged
/// - Source is marked DORMANT
pub fn merge_identifiers(
    graph: &mut CognitiveGraph,
    source_id: AtomId,
    dest_id: AtomId,
) -> Result<(), GraphError> {
    // Implementation per spec algorithm:
    // 1. Get all inbound edges of source
    // 2. For each inbound edge (u, source):
    //    a. If (u, dest) exists: w(u, dest) = max(w(u, dest), w(u, source))
    //    b. Else: create edge (u, dest) with same type and weight
    // 3. Get all outbound edges of source
    // 4. For each outbound edge (source, u):
    //    a. If (dest, u) exists: w(dest, u) = max(w(dest, u), w(source, u))
    //    b. Else: create edge (dest, u) with same type and weight
    // 5. dest.salience = max(dest.salience, source.salience)
    // 6. dest.reinforcement_history merged with source's
    // 7. Mark source as DORMANT
    // Note: we do NOT delete the source atom (Law 4: preserve)
    unimplemented!()
}

/// Check if two symbols are approximate matches (edit distance threshold)
pub fn is_approximate_match(a: &str, b: &str, threshold: usize) -> bool {
    levenshtein_distance(a, b) <= threshold
}

/// Levenshtein distance
fn levenshtein_distance(a: &str, b: &str) -> usize {
    // Standard DP implementation
    unimplemented!()
}
```

**Note:** `merge_identifiers` needs graph edge iteration. Since `petgraph::StableGraph` doesn't give easy "all edges of a node" access, you'll need to iterate all edges and filter. Add a helper method to `CognitiveGraph` if needed:
```rust
impl CognitiveGraph {
    /// Get all edges (inbound and outbound) for a given node
    pub fn edges_of(&self, id: AtomId) -> Vec<(EdgeIndex, Direction, &Relation)>
}
```

#### Tests for canonical.rs

- `test_canonicalize_lowercase` — "Hello World" → "hello world"
- `test_canonicalize_unicode` — "café" (composed vs decomposed) → same output
- `test_canonicalize_whitespace` — "  foo   bar  " → "foo bar"
- `test_merge_identifiers` — create two atoms with edges, merge, verify dest has all edges at max weight, source is DORMANT
- `test_approximate_match` — levenshtein distance tests
- `test_no_delete_on_merge` — source atom still exists after merge

---

### Dependencies to add

Add to `crates/dcma-core/Cargo.toml` if not already present:
```toml
[dependencies]
unicode-normalization = "0.1"
```

### Constraints

- NO floating-point arithmetic
- `cargo build` must succeed for the entire workspace
- `cargo test -p dcma-core` must pass
- `merge_identifiers` must NOT delete the source atom (Law 4)
