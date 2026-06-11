# DCMA PostgreSQL Storage — Implementation Instructions

## Context

DCMA workspace at `/workspace/dcma`. All 6 phases complete. We need a PostgreSQL-backed storage crate.

The existing `dcma-storage` crate uses file-based JSON storage. We need `dcma-pg` with PostgreSQL.

## IMPORTANT CONSTRAINTS

1. The Rust toolchain is 1.96.0, edition 2021
2. `async fn` in traits requires `async-trait` crate or Rust 1.75+ native support
3. The `async-trait` macro seems to have edition detection issues in this environment
4. SOLUTION: Do NOT use `async-trait`. Use `Box<dyn Future>` return types instead, or just implement `PgStorage` directly without a trait for now
5. Use `sqlx` version 0.8 with `runtime-tokio` and `postgres` features
6. Use `rust_decimal` with `db-postgres` feature for NUMERIC columns
7. Use `serde_json` for JSONB columns (attributes)
8. The database `dcma` already exists on `llm-ciel-memory-qk6cz9:5432`
9. User: `dcma`, Password: `dcma_secure_2026`
10. Schema is already migrated (tables created)

## What to Implement

### 1. `crates/dcma-pg/Cargo.toml`

```toml
[package]
name = "dcma-pg"
version = "0.1.0"
edition = "2021"

[dependencies]
dcma-core = { path = "../dcma-core" }
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.8", features = ["runtime-tokio","postgres","uuid","json","chrono","rust_decimal"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rust_decimal = { version = "1", features = ["db-postgres"] }
uuid = { version = "1", features = ["v4","serde"] }
thiserror = "1"
tracing = "0.1"
sha2 = "0.10"
chrono = { version = "0.4", features = ["serde"] }
```

Do NOT include `async-trait` — it has edition issues in this environment.

### 2. `crates/dcma-pg/src/lib.rs`

Simple module declarations:
```rust
pub mod error;
pub mod storage;
pub use error::PgError;
pub use storage::PgStorage;
```

### 3. `crates/dcma-pg/src/error.rs`

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PgError {
    #[error("Database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Schema migration failed: {0}")]
    Migration(String),
}
```

### 4. `crates/dcma-pg/src/storage.rs`

Implement `PgStorage` struct with methods (NOT using async-trait):

```rust
use dcma_core::atom::CognitiveAtom;
use dcma_core::state::CognitiveState;
use dcma_core::types::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::collections::HashMap;
use std::pin::Pin;
use uuid::Uuid;
use crate::error::PgError;

pub struct PgStorage {
    pool: PgPool,
}

impl PgStorage {
    pub async fn connect(url: &str) -> Result<Self, PgError> {
        let pool = PgPool::connect(url).await?;
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> Result<(), PgError> {
        sqlx::query(include_str!("../sql/schema.sql"))
            .execute(&self.pool)
            .await
            .map_err(|e| PgError::Migration(e.to_string()))?;
        Ok(())
    }

    pub fn pool(&self) -> &PgPool { &self.pool }

    pub async fn save_state(&self, state: &CognitiveState) -> Result<(), PgError> {
        // Use a transaction
        let mut tx = self.pool.begin().await?;

        // Upsert atoms
        for (_, atom) in state.graph.atoms() {
            let attrs = serde_json::to_value(&atom.attributes).unwrap_or(serde_json::json!({}));
            sqlx::query(
                r#"INSERT INTO dcma_atoms (id, atom_type, canonical, state, attributes, salience, confidence, origin, t_created, t_last_accessed, updated_at)
                   VALUES ($1::uuid, $2, $3, $4, $5, $6, $7, $8, $9, $10, now())
                   ON CONFLICT (id) DO UPDATE SET
                   atom_type=EXCLUDED.atom_type, canonical=EXCLUDED.canonical, state=EXCLUDED.state,
                   attributes=EXCLUDED.attributes, salience=EXCLUDED.salience, confidence=EXCLUDED.confidence,
                   origin=EXCLUDED.origin, t_last_accessed=EXCLUDED.t_last_accessed, updated_at=now()"#
            )
            .bind(atom.id.to_string())
            .bind(format!("{:?}", atom.atom_type))
            .bind(&atom.canonical)
            .bind(format!("{:?}", atom.state))
            .bind(attrs)
            .bind(atom.salience.get())
            .bind(atom.confidence.get())
            .bind(&atom.origin)
            .bind(atom.t_created as i64)
            .bind(atom.t_last_accessed as i64)
            .execute(&mut *tx).await?;
        }

        // Upsert relations
        for (_, rel) in state.graph.relations() {
            sqlx::query(
                r#"INSERT INTO dcma_relations (src_atom_id, dst_atom_id, relation_type, weight, stability, confidence, t_created, updated_at)
                   VALUES ($1::uuid, $2::uuid, $3, $4, $5, $6, $7, now())
                   ON CONFLICT (src_atom_id, dst_atom_id, relation_type) DO UPDATE SET
                   weight=EXCLUDED.weight, stability=EXCLUDED.stability, confidence=EXCLUDED.confidence, updated_at=now()"#
            )
            .bind(rel.src.to_string())
            .bind(rel.dst.to_string())
            .bind(format!("{:?}", rel.relation_type))
            .bind(rel.weight.get())
            .bind(rel.stability.get())
            .bind(rel.confidence.get())
            .bind(rel.t_created as i64)
            .execute(&mut *tx).await?;
        }

        // Upsert contradictions
        for ((a, b), pressure) in &state.contradictions {
            sqlx::query(
                r#"INSERT INTO dcma_contradictions (atom_a_id, atom_b_id, pressure, updated_at)
                   VALUES ($1::uuid, $2::uuid, $3, now())
                   ON CONFLICT (atom_a_id, atom_b_id) DO UPDATE SET pressure=EXCLUDED.pressure, updated_at=now()"#
            )
            .bind(a.to_string())
            .bind(b.to_string())
            .bind(pressure.get())
            .execute(&mut *tx).await?;
        }

        // Record tick
        let hash = Self::state_hash(state);
        sqlx::query(
            r#"INSERT INTO dcma_ticks (tick_number, state_hash, atom_count, relation_count, created_at)
               VALUES ($1, $2, $3, $4, now())
               ON CONFLICT (tick_number) DO UPDATE SET state_hash=EXCLUDED.state_hash, atom_count=EXCLUDED.atom_count, relation_count=EXCLUDED.relation_count"#
        )
        .bind(state.tick as i64)
        .bind(&hash)
        .bind(state.graph.atom_count() as i32)
        .bind(state.graph.relation_count() as i32)
        .execute(&mut *tx).await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn load_state(&self) -> Result<CognitiveState, PgError> {
        let config = EngineConfig::default();

        // Load atoms
        let rows: Vec<(String, String, String, String, serde_json::Value, Decimal, Decimal, String, i64, i64)> =
            sqlx::query_as("SELECT id, atom_type, canonical, state, attributes, salience, confidence, origin, t_created, t_last_accessed FROM dcma_atoms")
            .fetch_all(&self.pool).await?;

        let mut graph = dcma_core::graph::CognitiveGraph::new();
        let mut id_map: HashMap<String, AtomId> = HashMap::new();

        for row in &rows {
            let id = AtomId(Uuid::parse_str(&row.0).map_err(|e| PgError::Migration(e.to_string()))?);
            let atom_type = parse_atom_type(&row.1);
            let state = parse_atom_state(&row.3);
            let attributes: HashMap<String, String> = serde_json::from_value(row.4.clone()).unwrap_or_default();

            let mut atom = CognitiveAtom::new(id, atom_type, row.2.clone());
            atom.state = state;
            atom.attributes = attributes;
            atom.salience = Salience::new(row.5);
            atom.confidence = Confidence::new(row.6);
            atom.origin = row.7.clone();
            atom.t_created = row.8 as u64;
            atom.t_last_accessed = row.9 as u64;

            graph.add_atom(atom);
            id_map.insert(row.0.clone(), id);
        }

        // Load relations
        let rel_rows: Vec<(String, String, String, Decimal, Decimal, Decimal, i64)> =
            sqlx::query_as("SELECT src_atom_id, dst_atom_id, relation_type, weight, stability, confidence, t_created FROM dcma_relations")
            .fetch_all(&self.pool).await?;

        for row in &rel_rows {
            let src = id_map.get(&row.0).copied().unwrap_or_else(AtomId::new);
            let dst = id_map.get(&row.1).copied().unwrap_or_else(AtomId::new);
            let rel_type = parse_relation_type(&row.2);

            if let Ok(edge) = graph.add_relation_with_weight(src, dst, rel_type, Weight::new(row.3)) {
                if let Some(rel) = graph.get_relation_mut(edge) {
                    rel.stability = Confidence::new(row.4);
                    rel.confidence = Confidence::new(row.5);
                    rel.t_created = row.6 as u64;
                }
            }
        }

        // Load contradictions
        let contr_rows: Vec<(String, String, Decimal)> =
            sqlx::query_as("SELECT atom_a_id, atom_b_id, pressure FROM dcma_contradictions")
            .fetch_all(&self.pool).await?;

        let mut contradictions = HashMap::new();
        for row in &contr_rows {
            let a = id_map.get(&row.0).copied().unwrap_or_else(AtomId::new);
            let b = id_map.get(&row.1).copied().unwrap_or_else(AtomId::new);
            contradictions.insert((a, b), Pressure::new(row.2));
        }

        // Get latest tick
        let tick_row: Option<(i64,)> = sqlx::query_as("SELECT tick_number FROM dcma_ticks ORDER BY tick_number DESC LIMIT 1")
            .fetch_optional(&self.pool).await?;
        let tick = tick_row.map(|r| r.0 as u64).unwrap_or(0);

        Ok(CognitiveState {
            graph,
            activations: HashMap::new(),
            contradictions,
            tick,
            history: Vec::new(),
            config,
        })
    }

    pub async fn save_atom(&self, atom: &CognitiveAtom) -> Result<(), PgError> {
        let attrs = serde_json::to_value(&atom.attributes).unwrap_or(serde_json::json!({}));
        sqlx::query(
            r#"INSERT INTO dcma_atoms (id, atom_type, canonical, state, attributes, salience, confidence, origin, t_created, t_last_accessed, updated_at)
               VALUES ($1::uuid, $2, $3, $4, $5, $6, $7, $8, $9, $10, now())
               ON CONFLICT (id) DO UPDATE SET
               atom_type=EXCLUDED.atom_type, canonical=EXCLUDED.canonical, state=EXCLUDED.state,
               attributes=EXCLUDED.attributes, salience=EXCLUDED.salience, confidence=EXCLUDED.confidence,
               origin=EXCLUDED.origin, t_last_accessed=EXCLUDED.t_last_accessed, updated_at=now()"#
        )
        .bind(atom.id.to_string())
        .bind(format!("{:?}", atom.atom_type))
        .bind(&atom.canonical)
        .bind(format!("{:?}", atom.state))
        .bind(attrs)
        .bind(atom.salience.get())
        .bind(atom.confidence.get())
        .bind(&atom.origin)
        .bind(atom.t_created as i64)
        .bind(atom.t_last_accessed as i64)
        .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn delete_atom(&self, id: AtomId) -> Result<(), PgError> {
        sqlx::query("DELETE FROM dcma_atoms WHERE id = $1::uuid")
            .bind(id.to_string())
            .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn save_relation(&self, src: AtomId, dst: AtomId, rel_type: RelationType, weight: Weight, stability: Confidence) -> Result<(), PgError> {
        sqlx::query(
            r#"INSERT INTO dcma_relations (src_atom_id, dst_atom_id, relation_type, weight, stability, updated_at)
               VALUES ($1::uuid, $2::uuid, $3, $4, $5, now())
               ON CONFLICT (src_atom_id, dst_atom_id, relation_type) DO UPDATE SET
               weight=EXCLUDED.weight, stability=EXCLUDED.stability, updated_at=now()"#
        )
        .bind(src.to_string())
        .bind(dst.to_string())
        .bind(format!("{:?}", rel_type))
        .bind(weight.get())
        .bind(stability.get())
        .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn delete_relation(&self, src: AtomId, dst: AtomId, rel_type: RelationType) -> Result<(), PgError> {
        sqlx::query("DELETE FROM dcma_relations WHERE src_atom_id = $1::uuid AND dst_atom_id = $2::uuid AND relation_type = $3")
            .bind(src.to_string())
            .bind(dst.to_string())
            .bind(format!("{:?}", rel_type))
            .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn search_atoms(&self, query: &str, limit: i64) -> Result<Vec<AtomId>, PgError> {
        let pattern = format!("%{}%", query);
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT id FROM dcma_atoms WHERE canonical ILIKE $1 OR canonical % $2 ORDER BY salience DESC, canonical <-> $2 LIMIT $3"
        )
        .bind(&pattern)
        .bind(query)
        .bind(limit)
        .fetch_all(&self.pool).await?;

        Ok(rows.into_iter()
            .filter_map(|r| Uuid::parse_str(&r.0).ok())
            .map(AtomId)
            .collect())
    }

    pub async fn get_stats(&self) -> Result<StorageStats, PgError> {
        let atom_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM dcma_atoms").fetch_one(&self.pool).await?;
        let relation_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM dcma_relations").fetch_one(&self.pool).await?;
        let tick_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM dcma_ticks").fetch_one(&self.pool).await?;
        Ok(StorageStats { atom_count: atom_count.0, relation_count: relation_count.0, tick_count: tick_count.0, current_entropy: None })
    }

    fn state_hash(state: &CognitiveState) -> String {
        let json = serde_json::to_string(&state.graph).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub atom_count: i64,
    pub relation_count: i64,
    pub tick_count: i64,
    pub current_entropy: Option<Decimal>,
}

fn parse_atom_type(s: &str) -> AtomType {
    match s { "Concept" => AtomType::Concept, "Event" => AtomType::Event, "Entity" => AtomType::Entity, "Procedure" => AtomType::Procedure, "Abstraction" => AtomType::Abstraction, _ => AtomType::Concept }
}

fn parse_atom_state(s: &str) -> AtomState {
    match s { "Active" => AtomState::Active, "Dormant" => AtomState::Dormant, "Frozen" => AtomState::Frozen, "Consolidated" => AtomState::Consolidated, _ => AtomState::Active }
}

fn parse_relation_type(s: &str) -> RelationType {
    match s {
        "Causes" => RelationType::Causes, "Prevents" => RelationType::Prevents, "Enables" => RelationType::Enables,
        "Triggers" => RelationType::Triggers, "DerivesFrom" => RelationType::DerivesFrom, "PartOf" => RelationType::PartOf,
        "InstanceOf" => RelationType::InstanceOf, "Specializes" => RelationType::Specializes, "Generalizes" => RelationType::Generalizes,
        "Contains" => RelationType::Contains, "Implies" => RelationType::Implies, "Negates" => RelationType::Negates,
        "Supports" => RelationType::Supports, "Contradicts" => RelationType::Contradicts, "Precedes" => RelationType::Precedes,
        "Follows" => RelationType::Follows, "CoOccursWith" => RelationType::CoOccursWith, "PersistsWith" => RelationType::PersistsWith,
        "Reinforces" => RelationType::Reinforces, "Inhibits" => RelationType::Inhibits, "Activates" => RelationType::Activates,
        "Suppresses" => RelationType::Suppresses, "Focuses" => RelationType::Focuses,
        _ => RelationType::Supports,
    }
}
```

### 5. Tests

Write tests in `crates/dcma-pg/src/storage.rs` under `#[cfg(test)]`. Use `#[tokio::test]` for async tests.

Test DB URL: `postgres://dcma:dcma_secure_2026@llm-ciel-memory-qk6cz9:5432/dcma`

Tests needed:
1. `test_connect_and_migrate` — connect + migrate is idempotent
2. `test_save_load_empty_state` — save empty, load empty
3. `test_save_load_with_atoms` — save atoms + relations, load and verify
4. `test_save_atom_individual` — save single atom, search for it
5. `test_search_atoms` — fuzzy search with trigram
6. `test_get_stats` — returns valid counts
7. `test_save_relation` — save relation, verify in DB
8. `test_delete_atom` — save then delete, verify gone

### 6. Update workspace `Cargo.toml`

Add `"crates/dcma-pg"` to workspace members.

### 7. Build and test

```bash
cd /workspace/dcma
cargo build -p dcma-pg
cargo test -p dcma-pg
cargo test  # full workspace
```

## CRITICAL: Do NOT use `async-trait`. Use native `async fn` methods on the struct directly.
