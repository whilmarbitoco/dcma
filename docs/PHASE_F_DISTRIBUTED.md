# DCMA Phase F — Distributed Cognition Protocol

## Context

DCMA workspace at `/workspace/dcma`. Phases 1-8 are complete (core, activation, contradiction, entropy, retrieval, consolidation, topology, runtime, persistence/API/CLI). Phase F implements the **Distributed Cognition Protocol** — the final piece from the spec (Section 11).

This enables multiple agents to share a single cognitive substrate with eventual consistency and causal ordering.

The formal spec is at `docs/DCMA_FORMAL_SPEC.md` (Section 11). The build plan is at `docs/BUILD_PLAN.md` (Phase 9).

## What to Implement

### 1. `crates/dcma-distributed/src/hlc.rs`

Implement Hybrid Logical Clock (spec Section 11.2):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Timestamp {
    pub physical: u64,    // physical time (milliseconds since epoch)
    pub logical: u64,     // logical counter for same physical time
    pub node_id: AtomId,  // unique node identifier
}

impl Timestamp {
    pub fn new(node_id: AtomId) -> Self { ... }
    pub fn now(node_id: AtomId) -> Self { ... }
    
    /// Happens-before relation: self happens before other
    pub fn happens_before(&self, other: &Self) -> bool { ... }
    
    /// Happens-after relation
    pub fn happens_after(&self, other: &Self) -> bool { ... }
    
    /// Concurrent (neither happens-before nor happens-after)
    pub fn concurrent(&self, other: &Self) -> bool { ... }
    
    /// Advance clock on local event
    pub fn tick(&mut self) { ... }
    
    /// Advance clock on receiving a message with given timestamp
    pub fn receive(&mut self, received: &Timestamp) { ... }
}

impl PartialOrd for Timestamp {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> { ... }
}

impl Ord for Timestamp {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering { ... }
}
```

HLC algorithm:
- On local event: if physical time > last physical, set physical = now, logical = 0. Else increment logical.
- On receive: set physical = max(local physical, received physical). If equal, logical = max(local logical, received logical) + 1. If received > local, logical = received logical + 1.

### 2. `crates/dcma-distributed/src/merge.rs`

Implement CRDT-style merge (spec Section 11.2):

```rust
pub fn merge_edge_weights(w1: Decimal, w2: Decimal) -> Decimal {
    // w_merged = max(w1, w2)
    w1.max(w2)
}

pub fn merge_atoms(atom1: &CognitiveAtom, atom2: &CognitiveAtom) -> CognitiveAtom {
    // Union of attributes, max confidence, max salience
    // Keep earlier creation time, later access time
    ...
}

pub fn merge_states(state1: &CognitiveState, state2: &CognitiveState) -> CognitiveState {
    // Merge atoms (by AtomId), merge edges (by src+dst+type)
    // For conflicts: max weight, union attributes
    ...
}
```

Properties to verify:
- Commutative: merge(a, b) == merge(b, a)
- Associative: merge(merge(a, b), c) == merge(a, merge(b, c))
- Idempotent: merge(a, a) == a

### 3. `crates/dcma-distributed/src/protocol.rs`

Implement wire protocol (spec Section 11.2-11.3):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    /// Sync a tick result to peers
    TickSync {
        timestamp: Timestamp,
        tick: u64,
        state_fragment: CognitiveState,
    },
    /// Request state fragment from peer
    StateRequest {
        timestamp: Timestamp,
        node_ids: Vec<AtomId>,
    },
    /// Respond with state fragment
    StateResponse {
        timestamp: Timestamp,
        state_fragment: CognitiveState,
    },
    /// Propose merge
    MergeRequest {
        timestamp: Timestamp,
        state_fragment: CognitiveState,
    },
    /// Acknowledge merge
    MergeAck {
        timestamp: Timestamp,
        merged_state_hash: u64,
    },
    /// Heartbeat for partition detection
    Heartbeat {
        timestamp: Timestamp,
        node_id: AtomId,
    },
}

pub fn serialize_message(msg: &Message) -> Result<Vec<u8>, StorageError> { ... }
pub fn deserialize_message(data: &[u8]) -> Result<Message, StorageError> { ... }
```

### 4. `crates/dcma-distributed/src/lib.rs`

Public API:

```rust
pub struct Node {
    id: AtomId,
    config: EngineConfig,
    runtime: Runtime,
    peers: HashMap<AtomId, Timestamp>,
    hlc: Timestamp,
}

impl Node {
    pub fn new(config: EngineConfig) -> Self { ... }
    
    /// Connect to a peer
    pub fn connect(&mut self, peer_id: AtomId) { ... }
    
    /// Broadcast tick result to all peers
    pub fn broadcast_tick(&mut self, result: &TickResult) { ... }
    
    /// Receive and merge state from peer
    pub fn receive_state(&mut self, msg: &Message) -> Result<(), StorageError> { ... }
    
    /// Detect partition (peer hasn't sent heartbeat recently)
    pub fn detect_partitions(&self) -> Vec<AtomId> { ... }
    
    /// Heal partition by requesting full state from peer
    pub fn heal_partition(&mut self, peer_id: &AtomId) -> Result<(), StorageError> { ... }
    
    /// Get node ID
    pub fn id(&self) -> AtomId { ... }
    
    /// Get current HLC timestamp
    pub fn timestamp(&self) -> &Timestamp { ... }
}
```

### 5. `crates/dcma-distributed/Cargo.toml`

```toml
[package]
name = "dcma-distributed"
version = "0.1.0"
edition = "2021"

[dependencies]
dcma-core = { path = "../dcma-core" }
dcma-runtime = { path = "../dcma-runtime" }
rust_decimal = "1"
serde = { version = "1", features = ["derive"] }
thiserror = "1"
tokio = { version = "1", features = ["full"] }
```

### 6. Update workspace `Cargo.toml`

Add `dcma-distributed` to workspace members (it may already be there).

## Testing Requirements

### Unit tests (`dcma-distributed`):

1. **test_hlc_creation** — New timestamp has correct node_id
2. **test_hlc_local_event** — Tick advances logical counter
3. **test_hlc_receive_later** — Receiving message with later physical time updates clock
4. **test_hlc_receive_same_physical** — Same physical time increments logical
5. **test_hlc_happens_before** — Correct ordering for sequential events
6. **test_hlc_concurrent** — Concurrent events detected correctly
7. **test_hlc_ord** — Total ordering is consistent
8. **test_merge_edge_weights** — max(w1, w2) returns correct value
9. **test_merge_edge_weights_commutative** — merge(a,b) == merge(b,a)
10. **test_merge_edge_weights_idempotent** — merge(a,a) == a
11. **test_merge_atoms** — Attributes unioned, confidence maxed
12. **test_merge_states** — Two states merge correctly
13. **test_merge_commutative** — merge(a,b) == merge(b,a) for full states
14. **test_merge_associative** — merge(merge(a,b),c) == merge(a,merge(b,c))
15. **test_serialize_message** — Message round-trips through serialization
16. **test_node_creation** — Node has correct ID and initial state
17. **test_detect_partitions** — Stale peers detected as partitioned

### Integration tests (in `tests/` directory):

18. **test_two_nodes_share_state** — Create two nodes, tick one, sync to other, verify both have same state
19. **test_partition_healing** — Disconnect nodes, tick independently, reconnect, merge → identical state
20. **test_concurrent_ticks** — Two nodes tick independently, merge → no data loss

## Important Constraints

- Use `rust_decimal::Decimal` for all arithmetic
- HLC must be monotonic (never goes backward)
- CRDT merge must be commutative, associative, idempotent
- All functions must be deterministic (except network I/O)
- Follow existing code style
- Run `cargo build` and `cargo test` after implementation
- Fix all compiler warnings

## Existing Code Reference

- `crates/dcma-runtime/src/tick.rs` — Runtime, TickResult, Stimulus
- `crates/dcma-core/src/state.rs` — CognitiveState with full graph + metadata
- `crates/dcma-core/src/types.rs` — EngineConfig, all newtypes, AtomId
- `crates/dcma-core/src/graph.rs` — CognitiveGraph with subgraph(), atoms(), relations()
- `crates/dcma-storage/src/lib.rs` — Storage for serialization
