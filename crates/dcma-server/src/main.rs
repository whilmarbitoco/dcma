use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use dcma_core::atom::CognitiveAtom;
use dcma_core::types::*;
use dcma_runtime::{PgRuntime, Stimulus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

type SharedPgRuntime = Arc<Mutex<PgRuntime>>;

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    atoms: usize,
    relations: usize,
    tick: u64,
}

#[derive(Serialize, Clone)]
struct AtomSummary {
    id: String,
    atom_type: String,
    name: String,
    state: String,
    salience: String,
    confidence: String,
    content: Option<String>,
    tags: Option<String>,
    rank: Option<f64>,
}

#[derive(Serialize)]
struct SearchResult {
    atoms: Vec<AtomSummary>,
    total: usize,
    query: String,
    method: String,
}

#[derive(Serialize)]
struct GraphResult {
    query_atoms: Vec<AtomSummary>,
    related: Vec<AtomSummary>,
    relations: Vec<RelationView>,
}

#[derive(Serialize)]
struct RelationView {
    src_id: String,
    dst_id: String,
    src_name: String,
    dst_name: String,
    rel_type: String,
}

#[derive(Deserialize)]
struct IngestRequest {
    text: String,
}

#[derive(Serialize)]
struct IngestResponse {
    atoms_created: usize,
    relations_found: usize,
    entity_names: Vec<String>,
}

#[derive(Deserialize)]
struct AddAtomRequest {
    name: String,
    #[serde(rename = "type", default = "default_atom_type")]
    atom_type: String,
    #[serde(default)]
    attributes: HashMap<String, String>,
    #[serde(default)]
    salience: Option<f64>,
}

#[derive(Deserialize)]
struct AddRelationRequest {
    src: String,
    dst: String,
    #[serde(rename = "type")]
    rel_type: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

fn default_atom_type() -> String {
    "Concept".to_string()
}

async fn health(State(rt): State<SharedPgRuntime>) -> Json<HealthResponse> {
    let rt = rt.lock().await;
    let state = rt.state();
    Json(HealthResponse {
        status: "ok".to_string(),
        atoms: state.graph.atom_count(),
        relations: state.graph.relation_count(),
        tick: state.tick,
    })
}

async fn ingest(
    State(rt): State<SharedPgRuntime>,
    Json(req): Json<IngestRequest>,
) -> Result<Json<IngestResponse>, (StatusCode, Json<ErrorResponse>)> {
    if req.text.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: "Text cannot be empty".into() }),
        ));
    }

    let engine = dcma_ingest::IngestEngine::new();
    let extracted = engine.extract_entities(&req.text);
    let relations = engine.extract_relations(&extracted, &req.text);

    let mut rt = rt.lock().await;
    let mut entity_names = Vec::new();

    for entity in &extracted {
        let atom_type = match entity.entity_type.as_str() {
            "Entity" => AtomType::Entity,
            "Procedure" => AtomType::Procedure,
            "Event" => AtomType::Event,
            "Abstraction" => AtomType::Abstraction,
            _ => AtomType::Concept,
        };
        let id = AtomId::new();
        let atom = CognitiveAtom::new(id, atom_type, entity.name.clone());
        rt.state_mut().graph.add_atom(atom);
        entity_names.push(entity.name.clone());
    }

    let state = rt.state().clone();
    if let Err(e) = rt.storage().save_state(&state).await {
        tracing::warn!("Failed to persist after ingest: {}", e);
    }

    Ok(Json(IngestResponse {
        atoms_created: extracted.len(),
        relations_found: relations.len(),
        entity_names,
    }))
}

async fn remember(
    State(rt): State<SharedPgRuntime>,
    Json(req): Json<AddAtomRequest>,
) -> Result<Json<AtomSummary>, (StatusCode, Json<ErrorResponse>)> {
    if req.name.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: "Name cannot be empty".into() }),
        ));
    }

    let atom_type = match req.atom_type.to_lowercase().as_str() {
        "entity" => AtomType::Entity,
        "procedure" => AtomType::Procedure,
        "event" => AtomType::Event,
        "abstraction" => AtomType::Abstraction,
        _ => AtomType::Concept,
    };

    // Read content/tags before moving attributes into atom
    let content_val = req.attributes.get("content").cloned();
    let tags_val = req.attributes.get("tags").cloned();
    let name_val = req.name.clone();

    let mut rt = rt.lock().await;
    let id = AtomId::new();
    let mut atom = CognitiveAtom::new(id, atom_type, req.name.clone());
    if !req.attributes.is_empty() {
        atom.attributes = req.attributes;
    }
    if let Some(sal) = req.salience {
        use rust_decimal::Decimal;
        atom.salience = Salience::new(Decimal::try_from(sal).unwrap_or(Decimal::ZERO));
    }

    // Read values from atom BEFORE moving it
    let salience_str = atom.salience.get().to_string();
    let confidence_str = atom.confidence.get().to_string();

    rt.state_mut().graph.add_atom(atom);

    let summary = AtomSummary {
        id: id.to_string(),
        atom_type: format!("{:?}", atom_type),
        name: name_val,
        state: "Active".into(),
        salience: salience_str,
        confidence: confidence_str,
        content: content_val,
        tags: tags_val,
        rank: None,
    };

    let state = rt.state().clone();
    if let Err(e) = rt.storage().save_state(&state).await {
        tracing::warn!("Failed to persist after remember: {}", e);
    }

    Ok(Json(summary))
}

/// Full-text search across atom names + content + tags, ranked by relevance
async fn search(
    State(rt): State<SharedPgRuntime>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Json<SearchResult> {
    let query = params.get("q").cloned().unwrap_or_default();
    let limit: usize = params.get("limit").and_then(|l| l.parse().ok()).unwrap_or(20);
    let offset: usize = params.get("offset").and_then(|o| o.parse().ok()).unwrap_or(0);

    let rt = rt.lock().await;
    let state = rt.state();
    let query_lower = query.to_lowercase();

    // Score atoms: exact match > content match > name match > tag match
    let mut scored: Vec<(f64, AtomSummary)> = state
        .graph
        .atoms()
        .filter_map(|(_, atom)| {
            let name_lower = atom.canonical.to_lowercase();
            let content = atom.attributes.get("content").map(|c| c.to_lowercase()).unwrap_or_default();
            let tags = atom.attributes.get("tags").map(|t| t.to_lowercase()).unwrap_or_default();

            let mut score = 0.0;

            // Exact name match (highest)
            if name_lower == query_lower {
                score = 100.0;
            }
            // Name contains query
            else if name_lower.contains(&query_lower) {
                score = 50.0;
            }
            // Content contains query
            else if content.contains(&query_lower) {
                score = 30.0;
            }
            // Tags contain query
            else if tags.contains(&query_lower) {
                score = 20.0;
            }
            // Word-level match in name
            else {
                let query_words: Vec<&str> = query_lower.split_whitespace().collect();
                for word in &query_words {
                    if name_lower.contains(word) {
                        score += 10.0;
                    }
                    if content.contains(word) {
                        score += 5.0;
                    }
                    if tags.contains(word) {
                        score += 3.0;
                    }
                }
            }

            if score > 0.0 {
                Some((score, AtomSummary {
                    id: atom.id.to_string(),
                    atom_type: format!("{:?}", atom.atom_type),
                    name: atom.canonical.clone(),
                    state: format!("{:?}", atom.state),
                    salience: atom.salience.get().to_string(),
                    confidence: atom.confidence.get().to_string(),
                    content: atom.attributes.get("content").cloned(),
                    tags: atom.attributes.get("tags").cloned(),
                    rank: None,
                }))
            } else {
                None
            }
        })
        .collect();

    // Sort by score descending
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let total = scored.len();
    let atoms: Vec<AtomSummary> = scored
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|(score, mut atom)| {
            atom.rank = Some(score);
            atom
        })
        .collect();

    Json(SearchResult {
        atoms,
        total,
        query,
        method: "fulltext".into(),
    })
}

/// Graph-aware search: find matching atoms, then traverse their relations
async fn graph_search(
    State(rt): State<SharedPgRuntime>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Json<GraphResult> {
    let query = params.get("q").cloned().unwrap_or_default();
    let limit: usize = params.get("limit").and_then(|l| l.parse().ok()).unwrap_or(10);

    let rt = rt.lock().await;
    let state = rt.state();
    let query_lower = query.to_lowercase();

    // Find matching atoms
    let matched_ids: Vec<AtomId> = state
        .graph
        .atoms()
        .filter_map(|(_, atom)| {
            if atom.canonical.to_lowercase().contains(&query_lower)
                || atom.attributes.get("content").map_or(false, |c| c.to_lowercase().contains(&query_lower))
            {
                Some(atom.id)
            } else {
                None
            }
        })
        .take(limit)
        .collect();

    let query_atoms: Vec<AtomSummary> = matched_ids
        .iter()
        .filter_map(|id| {
            state.graph.get_atom(*id).map(|atom| AtomSummary {
                id: atom.id.to_string(),
                atom_type: format!("{:?}", atom.atom_type),
                name: atom.canonical.clone(),
                state: format!("{:?}", atom.state),
                salience: atom.salience.get().to_string(),
                confidence: atom.confidence.get().to_string(),
                content: atom.attributes.get("content").cloned(),
                tags: atom.attributes.get("tags").cloned(),
                rank: None,
            })
        })
        .collect();

    // Find related atoms via relations
    let mut related_ids = std::collections::HashSet::new();
    let mut relations = Vec::new();

    for id in &matched_ids {
        // Get outgoing relations
        for (_, rel) in state.graph.relations() {
            if rel.src == *id {
                related_ids.insert(rel.dst);
                if let Some(dst_atom) = state.graph.get_atom(rel.dst) {
                    relations.push(RelationView {
                        src_id: rel.src.to_string(),
                        dst_id: rel.dst.to_string(),
                        src_name: state.graph.get_atom(rel.src).map(|a| a.canonical.clone()).unwrap_or_default(),
                        dst_name: dst_atom.canonical.clone(),
                        rel_type: format!("{:?}", rel.relation_type),
                    });
                }
            } else if rel.dst == *id {
                related_ids.insert(rel.src);
                if let Some(src_atom) = state.graph.get_atom(rel.src) {
                    relations.push(RelationView {
                        src_id: rel.src.to_string(),
                        dst_id: rel.dst.to_string(),
                        src_name: src_atom.canonical.clone(),
                        dst_name: state.graph.get_atom(rel.dst).map(|a| a.canonical.clone()).unwrap_or_default(),
                        rel_type: format!("{:?}", rel.relation_type),
                    });
                }
            }
        }
    }

    // Remove query atoms from related
    for id in &matched_ids {
        related_ids.remove(id);
    }

    let related: Vec<AtomSummary> = related_ids
        .iter()
        .filter_map(|id| {
            state.graph.get_atom(*id).map(|atom| AtomSummary {
                id: atom.id.to_string(),
                atom_type: format!("{:?}", atom.atom_type),
                name: atom.canonical.clone(),
                state: format!("{:?}", atom.state),
                salience: atom.salience.get().to_string(),
                confidence: atom.confidence.get().to_string(),
                content: atom.attributes.get("content").cloned(),
                tags: atom.attributes.get("tags").cloned(),
                rank: None,
            })
        })
        .take(limit)
        .collect();

    Json(GraphResult {
        query_atoms,
        related,
        relations,
    })
}

async fn list_atoms(
    State(rt): State<SharedPgRuntime>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Json<SearchResult> {
    let limit: usize = params.get("limit").and_then(|l| l.parse().ok()).unwrap_or(50);
    let offset: usize = params.get("offset").and_then(|o| o.parse().ok()).unwrap_or(0);

    let rt = rt.lock().await;
    let state = rt.state();

    let mut atoms: Vec<AtomSummary> = state
        .graph
        .atoms()
        .map(|(_, atom)| AtomSummary {
            id: atom.id.to_string(),
            atom_type: format!("{:?}", atom.atom_type),
            name: atom.canonical.clone(),
            state: format!("{:?}", atom.state),
            salience: atom.salience.get().to_string(),
            confidence: atom.confidence.get().to_string(),
            content: atom.attributes.get("content").cloned(),
            tags: atom.attributes.get("tags").cloned(),
            rank: None,
        })
        .collect();

    // Sort by name
    atoms.sort_by(|a, b| a.name.cmp(&b.name));

    let total = atoms.len();
    let atoms: Vec<_> = atoms.into_iter().skip(offset).take(limit).collect();

    Json(SearchResult {
        atoms,
        total,
        query: "".into(),
        method: "list".into(),
    })
}

async fn get_atom(
    State(rt): State<SharedPgRuntime>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<AtomSummary>, (StatusCode, Json<ErrorResponse>)> {
    let uuid = Uuid::parse_str(&id).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: "Invalid UUID".into() }))
    })?;
    let atom_id = AtomId(uuid);

    let rt = rt.lock().await;
    let state = rt.state();

    match state.graph.get_atom(atom_id) {
        Some(atom) => Ok(Json(AtomSummary {
            id: atom.id.to_string(),
            atom_type: format!("{:?}", atom.atom_type),
            name: atom.canonical.clone(),
            state: format!("{:?}", atom.state),
            salience: atom.salience.get().to_string(),
            confidence: atom.confidence.get().to_string(),
            content: atom.attributes.get("content").cloned(),
            tags: atom.attributes.get("tags").cloned(),
            rank: None,
        })),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse { error: format!("Atom {} not found", id) }),
        )),
    }
}

async fn add_relation(
    State(rt): State<SharedPgRuntime>,
    Json(req): Json<AddRelationRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let src_uuid = Uuid::parse_str(&req.src).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: "Invalid source UUID".into() }))
    })?;
    let dst_uuid = Uuid::parse_str(&req.dst).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: "Invalid destination UUID".into() }))
    })?;

    let rel_type = parse_relation_type(&req.rel_type);

    let mut rt = rt.lock().await;
    let src_id = AtomId(src_uuid);
    let dst_id = AtomId(dst_uuid);

    rt.state_mut().graph.add_relation(src_id, dst_id, rel_type).map_err(|e| {
        (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: format!("{:?}", e) }))
    })?;

    let state = rt.state().clone();
    if let Err(e) = rt.storage().save_state(&state).await {
        tracing::warn!("Failed to persist after add_relation: {}", e);
    }

    Ok(Json(serde_json::json!({
        "status": "created", "src": req.src, "dst": req.dst, "type": req.rel_type
    })))
}

async fn execute_tick(State(rt): State<SharedPgRuntime>) -> Json<dcma_runtime::TickResult> {
    let mut rt = rt.lock().await;
    let stimulus = Stimulus {
        source_nodes: vec![],
        query_context: vec![],
        label: "server".into(),
    };
    let result = rt.tick(&stimulus).await;

    let state = rt.state().clone();
    if let Err(e) = rt.storage().save_state(&state).await {
        tracing::warn!("Failed to persist after tick: {}", e);
    }

    Json(result)
}

async fn get_state(State(rt): State<SharedPgRuntime>) -> Json<serde_json::Value> {
    let rt = rt.lock().await;
    let state = rt.state();
    Json(serde_json::json!({
        "atoms": state.graph.atom_count(),
        "relations": state.graph.relation_count(),
        "tick": state.tick,
    }))
}

fn parse_relation_type(s: &str) -> RelationType {
    match s.to_lowercase().as_str() {
        "causes" => RelationType::Causes,
        "prevents" => RelationType::Prevents,
        "enables" => RelationType::Enables,
        "triggers" => RelationType::Triggers,
        "derivesfrom" | "derives_from" => RelationType::DerivesFrom,
        "partof" | "part_of" => RelationType::PartOf,
        "instanceof" | "instance_of" => RelationType::InstanceOf,
        "specializes" => RelationType::Specializes,
        "generalizes" => RelationType::Generalizes,
        "contains" => RelationType::Contains,
        "abstractionof" | "abstraction_of" => RelationType::AbstractionOf,
        "implies" => RelationType::Implies,
        "negates" => RelationType::Negates,
        "supports" => RelationType::Supports,
        "contradicts" => RelationType::Contradicts,
        "precedes" => RelationType::Precedes,
        "follows" => RelationType::Follows,
        "cooccurswith" | "co_occurs_with" => RelationType::CoOccursWith,
        "persistswith" | "persists_with" => RelationType::PersistsWith,
        "reinforces" => RelationType::Reinforces,
        "inhibits" => RelationType::Inhibits,
        "activates" => RelationType::Activates,
        "suppresses" => RelationType::Suppresses,
        "focuses" => RelationType::Focuses,
        _ => RelationType::Supports,
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let pg_url = env::var("DCMA_DATABASE_URL").unwrap_or_else(|_| {
        "postgres://ciel:***@llm-ciel-memory-qk6cz9:5432/dcma".to_string()
    });

    tracing::info!("DCMA Server starting");

    let storage = dcma_pg::PgStorage::connect(&pg_url).await
        .expect("Failed to connect to PostgreSQL");
    storage.migrate().await.expect("Failed to run migrations");
    tracing::info!("Schema migrated");

    let config = EngineConfig::default();
    let runtime = PgRuntime::with_state(config, &pg_url).await
        .expect("Failed to create PgRuntime");

    let state: SharedPgRuntime = Arc::new(Mutex::new(runtime));

    let app = Router::new()
        .route("/health", get(health))
        .route("/state", get(get_state))
        .route("/ingest", post(ingest))
        .route("/remember", post(remember))
        .route("/search", get(search))
        .route("/graph", get(graph_search))
        .route("/atoms", get(list_atoms))
        .route("/atoms/:id", get(get_atom))
        .route("/relations", post(add_relation))
        .route("/tick", post(execute_tick))
        .with_state(state);

    let port: u16 = env::var("DCMA_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(3030);
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    tracing::info!(%addr, "Server listening");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service()).await.unwrap();
}
