# DCMA Phase E — Persistence + HTTP API + CLI

## Context

DCMA workspace at `/workspace/dcma`. Phases 1-7 are complete (core, activation, contradiction, entropy, retrieval, consolidation, topology, runtime). Phase E makes DCMA a **usable system** by adding persistence, an HTTP API, and a CLI.

The formal spec is at `docs/DCMA_FORMAL_SPEC.md`. The build plan is at `docs/BUILD_PLAN.md` (Phase 9-10).

## What to Implement

### 1. New crate: `crates/dcma-storage/` — Persistence Layer

File: `crates/dcma-storage/Cargo.toml`
```toml
[package]
name = "dcma-storage"
version = "0.1.0"
edition = "2021"

[dependencies]
dcma-core = { path = "../dcma-core" }
dcma-runtime = { path = "../dcma-runtime" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
bincode = "1"
thiserror = "1"
```

File: `crates/dcma-storage/src/lib.rs`

```rust
use dcma_core::state::CognitiveState;
use dcma_core::types::EngineConfig;
use dcma_runtime::{Runtime, TickResult, Stimulus};
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),
    #[error("State file corrupted")]
    Corrupted,
}

pub struct Storage;

impl Storage {
    /// Save full state as binary (bincode for speed)
    pub fn save_state(state: &CognitiveState, path: &Path) -> Result<(), StorageError> { ... }
    
    /// Load full state from binary
    pub fn load_state(path: &Path) -> Result<CognitiveState, StorageError> { ... }
    
    /// Save state as JSON (human-readable, for debugging)
    pub fn save_state_json(state: &CognitiveState, path: &Path) -> Result<(), StorageError> { ... }
    
    /// Load state from JSON
    pub fn load_state_json(path: &Path) -> Result<CognitiveState, StorageError> { ... }
    
    /// Append tick diff to write-ahead log
    pub fn append_wal(wal_path: &Path, tick: u64, state: &CognitiveState) -> Result<(), StorageError> { ... }
    
    /// Replay WAL from a checkpoint
    pub fn replay_wal(wal_path: &Path, checkpoint: &CognitiveState) -> Result<Vec<(u64, CognitiveState)>, StorageError> { ... }
    
    /// Create checkpoint (full state snapshot)
    pub fn checkpoint(state: &CognitiveState, dir: &Path, tick: u64) -> Result<(), StorageError> { ... }
    
    /// Load latest checkpoint from directory
    pub fn load_latest_checkpoint(dir: &Path) -> Result<(u64, CognitiveState), StorageError> { ... }
}
```

### 2. New crate: `crates/dcma-api/` — HTTP API

File: `crates/dcma-api/Cargo.toml`
```toml
[package]
name = "dcma-api"
version = "0.1.0"
edition = "2021"

[dependencies]
dcma-core = { path = "../dcma-core" }
dcma-runtime = { path = "../dcma-runtime" }
dcma-storage = { path = "../dcma-storage" }
axum = "0.7"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tower = "0.5"
tracing = "0.1"
```

File: `crates/dcma-api/src/lib.rs`

HTTP endpoints:
- `POST /atoms` — Insert atom (body: {type, name, attributes?})
- `POST /relations` — Create relation (body: {src, dst, type})
- `POST /query` — Retrieval (body: {nodes}) → CognitiveField
- `POST /tick` — Execute tick (body: {sources?, query?}) → TickResult
- `GET /state` — Current state summary (node count, edge count, entropy, tick count)
- `GET /atoms/:id` — Atom details
- `GET /stats` — Graph statistics
- `GET /health` — Health check

Use `tokio::sync::Mutex<Runtime>` for shared state.

### 3. New crate: `crates/dcma-cli/` — Command-Line Interface

File: `crates/dcma-cli/Cargo.toml`
```toml
[package]
name = "dcma-cli"
version = "0.1.0"
edition = "2021"

[dependencies]
dcma-core = { path = "../dcma-core" }
dcma-runtime = { path = "../dcma-runtime" }
dcma-storage = { path = "../dcma-storage" }
clap = { version = "4", features = ["derive"] }
serde_json = "1"
```

File: `crates/dcma-cli/src/main.rs`

Commands:
- `dcma init [--path <dir>]` — Initialize new cognitive state
- `dcma insert --type <TYPE> --name <NAME>` — Add atom
- `dcma connect --src <ID> --dst <ID> --type <TYPE>` — Add relation
- `dcma query --nodes <ID1,ID2>` — Retrieval query
- `dcma tick [--sources <ID1,ID2>]` — Execute one tick
- `dcma serve [--port <N>]` — Start HTTP API server
- `dcma stats` — Show graph statistics
- `dcma export --format <json|binary>` — Export state
- `dcma import --format <json|binary> --file <PATH>` — Import state

### 4. Update workspace `Cargo.toml`

Add `dcma-storage`, `dcma-api`, `dcma-cli` to workspace members.

## Testing Requirements

### Storage tests (`dcma-storage`):
1. **test_save_load_binary** — Save state, load it back, verify byte-identical
2. **test_save_load_json** — Save as JSON, load back, verify identical
3. **test_roundtrip_with_runtime** — Create Runtime, tick 10 times, save, load, tick 10 more, verify continuity
4. **test_wal_append** — Append to WAL, replay, verify all ticks recovered
5. **test_checkpoint** — Create checkpoint, load latest, verify correct state
6. **test_empty_state_roundtrip** — Save/load empty state

### API tests (`dcma-api`):
7. **test_insert_atom** — POST /atoms, verify 200 + atom in state
8. **test_create_relation** — POST /relations, verify edge exists
9. **test_query** — POST /query, verify CognitiveField returned
10. **test_tick** — POST /tick, verify TickResult with convergence info
11. **test_state_endpoint** — GET /state, verify correct counts
12. **test_health** — GET /health, verify 200

### CLI tests (`dcma-cli`):
13. **test_init_creates_state** — `dcma init` creates valid state file
14. **test_insert_and_query** — Insert atom, query it back
15. **test_tick_advances** — `dcma tick` increments tick count

## Important Constraints

- Use `rust_decimal::Decimal` for all arithmetic
- Storage must be crash-safe (WAL + checkpointing)
- API must be async (axum + tokio)
- CLI must be user-friendly with clear error messages
- Run `cargo build` and `cargo test` after implementation
- Fix all compiler warnings

## Existing Code Reference

- `crates/dcma-runtime/src/tick.rs` — Runtime, TickResult, Stimulus
- `crates/dcma-core/src/state.rs` — CognitiveState (already has Serialize/Deserialize)
- `crates/dcma-core/src/types.rs` — EngineConfig, all newtypes
- `crates/dcma-core/src/graph.rs` — CognitiveGraph with full Serialize/Deserialize
