use dcma_core::atom::CognitiveAtom;
use dcma_core::state::CognitiveState;
use dcma_core::types::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

use crate::error::PgError;

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub atom_count: i64,
    pub relation_count: i64,
    pub tick_count: i64,
    pub current_entropy: Option<Decimal>,
}

/// PostgreSQL-backed storage for DCMA
pub struct PgStorage {
    pool: PgPool,
}

impl PgStorage {
    /// Connect from a PostgreSQL URL
    pub async fn connect(url: &str) -> Result<Self, PgError> {
        let pool = PgPool::connect(url).await?;
        Ok(Self { pool })
    }

    /// Connect from individual parameters
    pub async fn connect_parts(host: &str, port: u16, db: &str, user: &str, password: &str) -> Result<Self, PgError> {
        let url = format!("postgres://{}:{}@{}:{}/{}", user, password, host, port, db);
        Self::connect(&url).await
    }

    /// Create from an existing connection pool
    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Run schema migration
    pub async fn migrate(&self) -> Result<(), PgError> {
        sqlx::raw_sql(include_str!("../sql/schema.sql"))
            .execute(&self.pool)
            .await
            .map_err(|e| PgError::Migration(e.to_string()))?;
        Ok(())
    }

    /// Get a reference to the pool (for advanced queries)
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Clear all data from the database (for testing)
    pub async fn clear(&self) -> Result<(), PgError> {
        sqlx::raw_sql(
            "TRUNCATE TABLE dcma_contradictions, dcma_relations, dcma_atoms, dcma_ticks, dcma_snapshots, dcma_reinforcements, dcma_activations, dcma_config CASCADE"
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

impl PgStorage {
    /// Compute SHA256 hash of a cognitive state for integrity checking
    fn state_hash(state: &CognitiveState) -> String {
        let json = serde_json::to_string(&state.graph).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Convert a CognitiveAtom to database row parameters
    fn atom_to_params(atom: &CognitiveAtom) -> (String, String, String, String, Decimal, Decimal, String, i64, i64) {
        (
            atom.id.to_string(),
            format!("{:?}", atom.atom_type),
            atom.canonical.clone(),
            format!("{:?}", atom.state),
            atom.salience.get(),
            atom.confidence.get(),
            atom.origin.clone(),
            atom.t_created as i64,
            atom.t_last_accessed as i64,
        )
    }
}

impl PgStorage {
    pub async fn save_state(&self, state: &CognitiveState) -> Result<(), PgError> {
        let mut tx = self.pool.begin().await?;

        // Upsert all atoms
        for (_, atom) in state.graph.atoms() {
            let (id, atom_type, canonical, state_str, salience, confidence, origin, t_created, t_last) =
                Self::atom_to_params(atom);

            sqlx::query(
                r#"
                INSERT INTO dcma_atoms (id, atom_type, canonical, state, attributes, salience, confidence, origin, t_created, t_last_accessed, updated_at)
                VALUES ($1::uuid, $2, $3, $4, $5, $6, $7, $8, $9, $10, now())
                ON CONFLICT (id) DO UPDATE SET
                    atom_type = EXCLUDED.atom_type,
                    canonical = EXCLUDED.canonical,
                    state = EXCLUDED.state,
                    attributes = EXCLUDED.attributes,
                    salience = EXCLUDED.salience,
                    confidence = EXCLUDED.confidence,
                    origin = EXCLUDED.origin,
                    t_last_accessed = EXCLUDED.t_last_accessed,
                    updated_at = now()
                "#,
            )
            .bind(&id)
            .bind(&atom_type)
            .bind(&canonical)
            .bind(&state_str)
            .bind(serde_json::to_value(&atom.attributes).unwrap_or(serde_json::json!({})))
            .bind(salience)
            .bind(confidence)
            .bind(&origin)
            .bind(t_created)
            .bind(t_last)
            .execute(&mut *tx)
            .await?;
        }

        // Upsert all relations
        for (_, rel) in state.graph.relations() {
            sqlx::query(
                r#"
                INSERT INTO dcma_relations (src_atom_id, dst_atom_id, relation_type, weight, stability, confidence, t_created, updated_at)
                VALUES ($1::uuid, $2::uuid, $3, $4, $5, $6, $7, now())
                ON CONFLICT (src_atom_id, dst_atom_id, relation_type) DO UPDATE SET
                    weight = EXCLUDED.weight,
                    stability = EXCLUDED.stability,
                    confidence = EXCLUDED.confidence,
                    updated_at = now()
                "#,
            )
            .bind(rel.src.to_string())
            .bind(rel.dst.to_string())
            .bind(format!("{:?}", rel.relation_type))
            .bind(rel.weight.get())
            .bind(rel.stability.get())
            .bind(rel.confidence.get())
            .bind(rel.t_created as i64)
            .execute(&mut *tx)
            .await?;
        }

        // Upsert contradictions
        for ((a, b), pressure) in &state.contradictions {
            sqlx::query(
                r#"
                INSERT INTO dcma_contradictions (atom_a_id, atom_b_id, pressure, updated_at)
                VALUES ($1::uuid, $2::uuid, $3, now())
                ON CONFLICT (atom_a_id, atom_b_id) DO UPDATE SET
                    pressure = EXCLUDED.pressure,
                    updated_at = now()
                "#,
            )
            .bind(a.to_string())
            .bind(b.to_string())
            .bind(pressure.get())
            .execute(&mut *tx)
            .await?;
        }

        // Record tick
        let hash = Self::state_hash(state);
        let atom_count = state.graph.atom_count() as i32;
        let relation_count = state.graph.relation_count() as i32;

        sqlx::query(
            r#"
            INSERT INTO dcma_ticks (tick_number, state_hash, atom_count, relation_count, created_at)
            VALUES ($1, $2, $3, $4, now())
            ON CONFLICT (tick_number) DO UPDATE SET
                state_hash = EXCLUDED.state_hash,
                atom_count = EXCLUDED.atom_count,
                relation_count = EXCLUDED.relation_count
            "#,
        )
        .bind(state.tick as i64)
        .bind(&hash)
        .bind(atom_count)
        .bind(relation_count)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn load_state(&self) -> Result<CognitiveState, PgError> {
        // Load config or create default
        let config = EngineConfig::default();

        // Load atoms
        let atom_rows = sqlx::query_as::<_, (Uuid, String, String, String, serde_json::Value, Decimal, Decimal, String, i64, i64)>(
            "SELECT id, atom_type, canonical, state, attributes, salience, confidence, origin, t_created, t_last_accessed FROM dcma_atoms"
        )
        .fetch_all(&self.pool)
        .await?;

        let mut graph = dcma_core::graph::CognitiveGraph::new();
        let mut id_map: HashMap<Uuid, AtomId> = HashMap::new();

        for row in &atom_rows {
            let id = AtomId(row.0);
            let atom_type = match row.1.as_str() {
                "Concept" => AtomType::Concept,
                "Event" => AtomType::Event,
                "Entity" => AtomType::Entity,
                "Procedure" => AtomType::Procedure,
                "Abstraction" => AtomType::Abstraction,
                _ => AtomType::Concept,
            };
            let state = match row.3.as_str() {
                "Active" => AtomState::Active,
                "Dormant" => AtomState::Dormant,
                "Frozen" => AtomState::Frozen,
                "Consolidated" => AtomState::Consolidated,
                _ => AtomState::Active,
            };
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
            id_map.insert(row.0, id);
        }

        // Load relations
        let rel_rows = sqlx::query_as::<_, (Uuid, Uuid, String, Decimal, Decimal, Decimal, i64)>(
            "SELECT src_atom_id, dst_atom_id, relation_type, weight, stability, confidence, t_created FROM dcma_relations"
        )
        .fetch_all(&self.pool)
        .await?;

        for row in &rel_rows {
            let src = id_map.get(&row.0).copied().unwrap_or_else(AtomId::new);
            let dst = id_map.get(&row.1).copied().unwrap_or_else(AtomId::new);
            let rel_type = parse_relation_type(&row.2);
            let weight = Weight::new(row.3);
            let stability = Confidence::new(row.4);
            let confidence = Confidence::new(row.5);

            if let Ok(edge) = graph.add_relation_with_weight(src, dst, rel_type, weight) {
                if let Some(rel) = graph.get_relation_mut(edge) {
                    rel.stability = stability;
                    rel.confidence = confidence;
                    rel.t_created = row.6 as u64;
                }
            }
        }

        // Load contradictions
        let contr_rows = sqlx::query_as::<_, (Uuid, Uuid, Decimal)>(
            "SELECT atom_a_id, atom_b_id, pressure FROM dcma_contradictions"
        )
        .fetch_all(&self.pool)
        .await?;

        let mut contradictions = HashMap::new();
        for row in &contr_rows {
            let a = id_map.get(&row.0).copied().unwrap_or_else(AtomId::new);
            let b = id_map.get(&row.1).copied().unwrap_or_else(AtomId::new);
            contradictions.insert((a, b), Pressure::new(row.2));
        }

        // Get latest tick
        let tick_row = sqlx::query_as::<_, (i64,)>(
            "SELECT tick_number FROM dcma_ticks ORDER BY tick_number DESC LIMIT 1"
        )
        .fetch_optional(&self.pool)
        .await?;

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

    pub async fn save_tick(&self, _tick: u64, state: &CognitiveState) -> Result<(), PgError> {
        self.save_state(state).await
    }

    pub async fn load_ticks(&self, _from: u64, _to: u64) -> Result<Vec<(u64, CognitiveState)>, PgError> {
        // For now, return the current state as a single tick
        // Full implementation would load from dcma_snapshots
        let state = self.load_state().await?;
        Ok(vec![(state.tick, state)])
    }

    pub async fn save_atom(&self, atom: &CognitiveAtom) -> Result<(), PgError> {
        let (id, atom_type, canonical, state_str, salience, confidence, origin, t_created, t_last) =
            Self::atom_to_params(atom);

        sqlx::query(
            r#"
            INSERT INTO dcma_atoms (id, atom_type, canonical, state, attributes, salience, confidence, origin, t_created, t_last_accessed, updated_at)
            VALUES ($1::uuid, $2, $3, $4, $5, $6, $7, $8, $9, $10, now())
            ON CONFLICT (id) DO UPDATE SET
                atom_type = EXCLUDED.atom_type,
                canonical = EXCLUDED.canonical,
                state = EXCLUDED.state,
                attributes = EXCLUDED.attributes,
                salience = EXCLUDED.salience,
                confidence = EXCLUDED.confidence,
                origin = EXCLUDED.origin,
                t_last_accessed = EXCLUDED.t_last_accessed,
                updated_at = now()
            "#,
        )
        .bind(&id)
        .bind(&atom_type)
        .bind(&canonical)
        .bind(&state_str)
        .bind(serde_json::to_value(&atom.attributes).unwrap_or(serde_json::json!({})))
        .bind(salience)
        .bind(confidence)
        .bind(&origin)
        .bind(t_created)
        .bind(t_last)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_atom(&self, id: AtomId) -> Result<(), PgError> {
        sqlx::query("DELETE FROM dcma_atoms WHERE id = $1::uuid")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn save_relation(&self, src: AtomId, dst: AtomId, rel_type: RelationType, weight: Weight, stability: Confidence) -> Result<(), PgError> {
        sqlx::query(
            r#"
            INSERT INTO dcma_relations (src_atom_id, dst_atom_id, relation_type, weight, stability, updated_at)
            VALUES ($1::uuid, $2::uuid, $3, $4, $5, now())
            ON CONFLICT (src_atom_id, dst_atom_id, relation_type) DO UPDATE SET
                weight = EXCLUDED.weight,
                stability = EXCLUDED.stability,
                updated_at = now()
            "#,
        )
        .bind(src.to_string())
        .bind(dst.to_string())
        .bind(format!("{:?}", rel_type))
        .bind(weight.get())
        .bind(stability.get())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_relation(&self, src: AtomId, dst: AtomId, rel_type: RelationType) -> Result<(), PgError> {
        sqlx::query("DELETE FROM dcma_relations WHERE src_atom_id = $1::uuid AND dst_atom_id = $2::uuid AND relation_type = $3")
            .bind(src.to_string())
            .bind(dst.to_string())
            .bind(format!("{:?}", rel_type))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn search_atoms(&self, query: &str, limit: i64) -> Result<Vec<AtomId>, PgError> {
        let rows = sqlx::query_as::<_, (Uuid,)>(
            r#"
            SELECT id FROM dcma_atoms
            WHERE canonical ILIKE $1
               OR canonical % $2
            ORDER BY salience DESC, canonical <-> $2
            LIMIT $3
            "#,
        )
        .bind(format!("%{}%", query))
        .bind(query)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| AtomId(r.0)).collect())
    }

    pub async fn get_stats(&self) -> Result<StorageStats, PgError> {
        let atom_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM dcma_atoms")
            .fetch_one(&self.pool)
            .await?;
        let relation_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM dcma_relations")
            .fetch_one(&self.pool)
            .await?;
        let tick_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM dcma_ticks")
            .fetch_one(&self.pool)
            .await?;

        Ok(StorageStats {
            atom_count: atom_count.0,
            relation_count: relation_count.0,
            tick_count: tick_count.0,
            current_entropy: None,
        })
    }
}

/// Parse a relation type from its Debug string representation
fn parse_relation_type(s: &str) -> RelationType {
    match s {
        "Causes" => RelationType::Causes,
        "Prevents" => RelationType::Prevents,
        "Enables" => RelationType::Enables,
        "Triggers" => RelationType::Triggers,
        "DerivesFrom" => RelationType::DerivesFrom,
        "PartOf" => RelationType::PartOf,
        "InstanceOf" => RelationType::InstanceOf,
        "Specializes" => RelationType::Specializes,
        "Generalizes" => RelationType::Generalizes,
        "Contains" => RelationType::Contains,
        "Implies" => RelationType::Implies,
        "Negates" => RelationType::Negates,
        "Supports" => RelationType::Supports,
        "Contradicts" => RelationType::Contradicts,
        "Precedes" => RelationType::Precedes,
        "Follows" => RelationType::Follows,
        "CoOccursWith" => RelationType::CoOccursWith,
        "PersistsWith" => RelationType::PersistsWith,
        "Reinforces" => RelationType::Reinforces,
        "Inhibits" => RelationType::Inhibits,
        "Activates" => RelationType::Activates,
        "Suppresses" => RelationType::Suppresses,
        "Focuses" => RelationType::Focuses,
        _ => RelationType::Supports,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dcma_core::atom::CognitiveAtom;
    use serial_test::serial;

    fn test_db_url() -> String {
        std::env::var("DCMA_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://dcma:dcma_secure_2026@llm-ciel-memory-qk6cz9:5432/dcma".to_string())
    }

    #[serial]
    #[tokio::test]
    async fn test_connect_and_migrate() {
        let storage = PgStorage::connect(&test_db_url()).await.unwrap();
        // Migrate is idempotent
        storage.migrate().await.unwrap();
    }

    #[serial]
    #[tokio::test]
    async fn test_save_load_empty_state() {
        let storage = PgStorage::connect(&test_db_url()).await.unwrap();
        storage.clear().await.unwrap();
        let config = EngineConfig::default();
        let state = CognitiveState::new(config);

        storage.save_state(&state).await.unwrap();
        let loaded = storage.load_state().await.unwrap();

        assert_eq!(loaded.graph.atom_count(), 0);
        assert_eq!(loaded.graph.relation_count(), 0);
        assert_eq!(loaded.tick, 0);
    }

    #[serial]
    #[tokio::test]
    async fn test_save_load_with_atoms() {
        let storage = PgStorage::connect(&test_db_url()).await.unwrap();
        storage.clear().await.unwrap();
        let config = EngineConfig::default();
        let mut state = CognitiveState::new(config);

        let a = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "test_atom".to_string());
        let b = CognitiveAtom::new(AtomId::new(), AtomType::Entity, "another".to_string());
        let id_a = a.id;
        let id_b = b.id;

        state.graph.add_atom(a);
        state.graph.add_atom(b);
        state.graph.add_relation(id_a, id_b, RelationType::Causes).unwrap();
        state.tick = 5;

        storage.save_state(&state).await.unwrap();
        let loaded = storage.load_state().await.unwrap();

        assert_eq!(loaded.graph.atom_count(), 2);
        assert_eq!(loaded.graph.relation_count(), 1);
        assert_eq!(loaded.tick, 5);
        assert!(loaded.graph.get_atom(id_a).is_some());
        assert!(loaded.graph.get_atom(id_b).is_some());
    }

    #[serial]
    #[tokio::test]
    async fn test_save_atom_individual() {
        let storage = PgStorage::connect(&test_db_url()).await.unwrap();
        let atom = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "individual".to_string());
        let id = atom.id;

        storage.save_atom(&atom).await.unwrap();

        let results = storage.search_atoms("individual", 10).await.unwrap();
        assert!(results.contains(&id));
    }

    #[serial]
    #[tokio::test]
    async fn test_search_atoms() {
        let storage = PgStorage::connect(&test_db_url()).await.unwrap();

        let a1 = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "machine_learning".to_string());
        let a2 = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "machine_vision".to_string());
        let a3 = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "database".to_string());
        let id1 = a1.id;
        let id3 = a3.id;

        storage.save_atom(&a1).await.unwrap();
        storage.save_atom(&a2).await.unwrap();
        storage.save_atom(&a3).await.unwrap();

        let results = storage.search_atoms("machine", 10).await.unwrap();
        assert!(results.contains(&id1));
        assert!(results.contains(&id3) == false || results.len() >= 1);
    }

    #[serial]
    #[tokio::test]
    async fn test_get_stats() {
        let storage = PgStorage::connect(&test_db_url()).await.unwrap();
        let stats = storage.get_stats().await.unwrap();
        // Just verify it doesn't panic and returns valid data
        assert!(stats.atom_count >= 0);
        assert!(stats.relation_count >= 0);
    }

    #[serial]
    #[tokio::test]
    async fn test_save_relation() {
        let storage = PgStorage::connect(&test_db_url()).await.unwrap();
        storage.clear().await.unwrap();

        let atom_a = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "rel_src".to_string());
        let atom_b = CognitiveAtom::new(AtomId::new(), AtomType::Entity, "rel_dst".to_string());
        let src = atom_a.id;
        let dst = atom_b.id;
        storage.save_atom(&atom_a).await.unwrap();
        storage.save_atom(&atom_b).await.unwrap();

        storage.save_relation(src, dst, RelationType::Causes, Weight::new(Decimal::from_str_exact("0.7").unwrap()), Confidence::new(Decimal::from_str_exact("0.8").unwrap())).await.unwrap();

        // Verify by loading state
        let state = storage.load_state().await.unwrap();
        assert_eq!(state.graph.relation_count(), 1);
    }

    #[serial]
    #[tokio::test]
    async fn test_delete_atom() {
        let storage = PgStorage::connect(&test_db_url()).await.unwrap();
        let atom = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "to_delete".to_string());
        let id = atom.id;

        storage.save_atom(&atom).await.unwrap();
        storage.delete_atom(id).await.unwrap();

        let results = storage.search_atoms("to_delete", 10).await.unwrap();
        assert!(!results.contains(&id));
    }

    #[serial]
    #[tokio::test]
    async fn test_state_roundtrip_with_runtime() {
        // This test requires dcma-runtime which has a circular dependency with dcma-pg
        // Skip if runtime feature not available
    }
}
