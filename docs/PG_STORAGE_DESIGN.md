# DCMA PostgreSQL Storage — Design & Implementation Plan

## Overview

DCMA gets its own dedicated PostgreSQL database. Clean separation from the `ciel` memory DB. Plug-and-play: the storage backend is a trait, PostgreSQL is one implementation, and swapping it out requires zero changes to engine code.

## Architecture

```
┌─────────────────────────────────────────────┐
│              dcma-runtime                    │
│         (Runtime, Tick, Phi)                 │
└──────────────┬──────────────────────────────┘
               │ uses
┌──────────────▼──────────────────────────────┐
│         dcma-storage (trait)                 │
│  ┌─────────────────────────────────────┐    │
│  │ trait StorageBackend                 │    │
│  │   save_state(&CognitiveState)       │    │
│  │   load_state() -> CognitiveState    │    │
│  │   save_tick(tick, &state)           │    │
│  │   load_ticks(from, to) -> Vec       │    │
│  │   save_atom(&CognitiveAtom)         │    │
│  │   load_atoms() -> Vec<CognitiveAtom>│    │
│  │   save_relation(&Relation)          │    │
│  │   load_relations() -> Vec<Relation> │    │
│  │   search_atoms(query) -> Vec<AtomId>│    │
│  │   get_stats() -> StorageStats       │    │
│  └─────────────────────────────────────┘    │
│                    │                         │
│        ┌───────────┼───────────┐            │
│        ▼           ▼           ▼            │
│   ┌─────────┐ ┌─────────┐ ┌─────────┐     │
│   │  File   │ │   PG    │ │  Memory │     │
│   │(existing)│ │ (new)   │ │ (tests) │     │
│   └─────────┘ └─────────┘ └─────────┘     │
└─────────────────────────────────────────────┘
```

## PostgreSQL Schema

```sql
-- Dedicated database: dcma
-- Created via: CREATE DATABASE dcma;

-- ─── Atoms (Cognitive Atoms) ────────────────────────────────────
CREATE TABLE dcma_atoms (
    id              UUID PRIMARY KEY,
    atom_type       TEXT NOT NULL CHECK (atom_type IN ('Concept','Event','Entity','Procedure','Abstraction')),
    canonical       TEXT NOT NULL,
    state           TEXT NOT NULL DEFAULT 'Active' CHECK (state IN ('Active','Dormant','Frozen','Consolidated')),
    attributes      JSONB NOT NULL DEFAULT '{}',
    salience        NUMERIC(10,8) NOT NULL DEFAULT 0,
    confidence      NUMERIC(10,8) NOT NULL DEFAULT 0 CHECK (confidence >= 0 AND confidence <= 1),
    origin          TEXT NOT NULL DEFAULT '',
    t_created       BIGINT NOT NULL DEFAULT 0,
    t_last_accessed BIGINT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_dcma_atoms_canonical ON dcma_atoms (canonical);
CREATE INDEX idx_dcma_atoms_type ON dcma_atoms (atom_type);
CREATE INDEX idx_dcma_atoms_state ON dcma_atoms (state);
CREATE INDEX idx_dcma_atoms_salience ON dcma_atoms (salience DESC);
CREATE INDEX idx_dcma_atoms_canonical_trgm ON dcma_atoms USING gin (canonical gin_trgm_ops);

-- ─── Relations (Edges) ──────────────────────────────────────────
CREATE TABLE dcma_relations (
    id              BIGSERIAL PRIMARY KEY,
    src_atom_id     UUID NOT NULL REFERENCES dcma_atoms(id) ON DELETE CASCADE,
    dst_atom_id     UUID NOT NULL REFERENCES dcma_atoms(id) ON DELETE CASCADE,
    relation_type   TEXT NOT NULL,
    weight          NUMERIC(10,8) NOT NULL DEFAULT 0.5 CHECK (weight >= 0 AND weight <= 1),
    stability       NUMERIC(10,8) NOT NULL DEFAULT 0 CHECK (stability >= 0 AND stability <= 1),
    confidence      NUMERIC(10,8) NOT NULL DEFAULT 0 CHECK (confidence >= 0 AND confidence <= 1),
    t_created       BIGINT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(src_atom_id, dst_atom_id, relation_type)
);

CREATE INDEX idx_dcma_rel_src ON dcma_relations (src_atom_id);
CREATE INDEX idx_dcma_rel_dst ON dcma_relations (dst_atom_id);
CREATE INDEX idx_dcma_rel_type ON dcma_relations (relation_type);

-- ─── Contradictions ──────────────────────────────────────────────
CREATE TABLE dcma_contradictions (
    atom_a_id       UUID NOT NULL REFERENCES dcma_atoms(id) ON DELETE CASCADE,
    atom_b_id       UUID NOT NULL REFERENCES dcma_atoms(id) ON DELETE CASCADE,
    pressure        NUMERIC(10,8) NOT NULL DEFAULT 0 CHECK (pressure >= 0 AND pressure <= 1),
    equilibrium     TEXT NOT NULL DEFAULT 'StableEquilibrium'
                    CHECK (equilibrium IN ('StableEquilibrium','DominantEquilibrium','Oscillating','Resolved')),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (atom_a_id, atom_b_id)
);

-- ─── Tick Log ───────────────────────────────────────────────────
CREATE TABLE dcma_ticks (
    tick_number     BIGINT PRIMARY KEY,
    state_hash      TEXT NOT NULL,  -- SHA256 of serialized state for integrity
    atom_count      INT NOT NULL,
    relation_count  INT NOT NULL,
    entropy         NUMERIC(10,8),
    convergence_iters INT,
    duration_ms     INT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_dcma_ticks_created ON dcma_ticks (created_at);

-- ─── State Snapshots (periodic) ────────────────────────────────
CREATE TABLE dcma_snapshots (
    id              BIGSERIAL PRIMARY KEY,
    tick_number     BIGINT NOT NULL REFERENCES dcma_ticks(tick_number),
    state_data      BYTEA NOT NULL,  -- bincode serialized CognitiveState
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_dcma_snapshots_tick ON dcma_snapshots (tick_number);

-- ─── Reinforcement History ──────────────────────────────────────
CREATE TABLE dcma_reinforcements (
    id              BIGSERIAL PRIMARY KEY,
    atom_id         UUID NOT NULL REFERENCES dcma_atoms(id) ON DELETE CASCADE,
    tick            BIGINT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_dcma_reinf_atom ON dcma_reinforcements (atom_id, tick);

-- ─── Activation History ────────────────────────────────────────
CREATE TABLE dcma_activations (
    id              BIGSERIAL PRIMARY KEY,
    atom_id         UUID NOT NULL REFERENCES dcma_atoms(id) ON DELETE CASCADE,
    tick            BIGINT NOT NULL,
    activation      NUMERIC(10,8) NOT NULL CHECK (activation >= 0 AND activation <= 1),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_dcma_act_atom ON dcma_activations (atom_id, tick);

-- ─── Configuration ──────────────────────────────────────────────
CREATE TABLE dcma_config (
    key             TEXT PRIMARY KEY,
    value           JSONB NOT NULL,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ─── Enable extensions ──────────────────────────────────────────
CREATE EXTENSION IF NOT EXISTS pg_trgm;       -- trigram text search
CREATE EXTENSION IF NOT EXISTS btree_gist;    -- for exclusion constraints
```

## Rust Implementation

### New crate: `dcma-pg/` (replaces file-based storage for production)

```rust
// crates/dcma-pg/Cargo.toml
[package]
name = "dcma-pg"
version = "0.1.0"
edition = "2021"

[dependencies]
dcma-core = { path = "../dcma-core" }
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.7", features = ["runtime-tokio","postgres","uuid","json","chrono","rust_decimal"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rust_decimal = { version = "1", features = ["db-postgres"] }
uuid = { version = "1", features = ["v4","serde"] }
thiserror = "1"
tracing = "0.1"
sha2 = "0.10"
```

### Core trait (in dcma-storage, refactored)

```rust
#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn save_state(&self, state: &CognitiveState) -> Result<(), StorageError>;
    async fn load_state(&self) -> Result<CognitiveState, StorageError>;
    async fn save_tick(&self, tick: u64, state: &CognitiveState) -> Result<(), StorageError>;
    async fn load_ticks(&self, from: u64, to: u64) -> Result<Vec<(u64, CognitiveState)>, StorageError>;
    async fn save_atom(&self, atom: &CognitiveAtom) -> Result<(), StorageError>;
    async fn save_relation(&self, relation: &Relation) -> Result<(), StorageError>;
    async fn search_atoms(&self, query: &str, limit: usize) -> Result<Vec<AtomId>, StorageError>;
    async fn get_stats(&self) -> Result<StorageStats, StorageError>;
}

pub struct StorageStats {
    pub atom_count: i64,
    pub relation_count: i64,
    pub tick_count: i64,
    pub current_entropy: Option<Decimal>,
}
```

### PostgreSQL implementation

```rust
pub struct PgStorage {
    pool: sqlx::PgPool,
}

impl PgStorage {
    pub async fn connect(url: &str) -> Result<Self, StorageError> {
        let pool = sqlx::PgPool::connect(url).await?;
        Ok(Self { pool })
    }

    pub async fn connect_from_config(host: &str, port: u16, db: &str, user: &str, pass: &str) -> Result<Self, StorageError> {
        let url = format!("postgres://{}:{}@{}:{}/{}", user, pass, host, port, db);
        Self::connect(&url).await
    }

    pub async fn migrate(&self) -> Result<(), StorageError> {
        sqlx::query(include_str!("../sql/schema.sql"))
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl StorageBackend for PgStorage {
    async fn save_state(&self, state: &CognitiveState) -> Result<(), StorageError> {
        // Upsert atoms
        for (_, atom) in state.graph.atoms() {
            sqlx::query("INSERT INTO dcma_atoms (...) VALUES (...) ON CONFLICT (id) DO UPDATE SET ...")
                .bind(atom.id)
                .bind(atom.canonical.as_str())
                // ...
                .execute(&self.pool).await?;
        }
        // Upsert relations
        for (_, rel) in state.graph.relations() {
            sqlx::query("INSERT INTO dcma_relations (...) VALUES (...) ON CONFLICT (...) DO UPDATE SET ...")
                .execute(&self.pool).await?;
        }
        // Record tick
        sqlx::query("INSERT INTO dcma_ticks (...) VALUES (...)")
            .execute(&self.pool).await?;
        Ok(())
    }
    // ... other methods
}
```

### Runtime integration

```rust
// Runtime gets a storage backend
pub struct Runtime<S: StorageBackend> {
    config: EngineConfig,
    state: CognitiveState,
    storage: S,
    // ...
}

// Or with type alias for convenience:
pub type FileRuntime = Runtime<FileStorage>;
pub type PgRuntime = Runtime<PgStorage>;
```

## Connection Parameters

```
Host:     llm-ciel-memory-qk6cz9 (or dedicated DCMA instance)
Port:     5432
Database: dcma (new, separate from ciel)
User:     dcma
Password: (new, dedicated)
```

## Migration Path

1. Create `dcma` database on existing PG instance
2. Run schema migration
3. `PgStorage::connect_from_config(...).migrate()`
4. Runtime::with_storage(config, pg_storage)
5. All engines unchanged — they operate on `CognitiveState` in memory
6. Storage is called at tick boundaries for persistence

## What This Gives Us

- **Full SQL queryability** — search atoms by canonical name, filter by type/state, graph traversal via JOINs
- **FTS** — trigram index on canonical for fuzzy search
- **Durability** — PostgreSQL WAL, crash-safe
- **Shared state** — multiple DCMA instances can connect to the same DB
- **Audit trail** — tick log, activation history, reinforcement history
- **Plug-and-play** — swap `FileStorage` ↔ `PgStorage` ↔ `MemoryStorage` via the trait
- **No contamination** — `ciel` DB is untouched
