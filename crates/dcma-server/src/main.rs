use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, patch, post},
    Json, Router,
};
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
    activation: Option<f64>,
    created_at: Option<String>,
    updated_at: Option<String>,
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

#[derive(Serialize, Clone)]
struct RelationView {
    src_id: String,
    dst_id: String,
    src_name: String,
    dst_name: String,
    rel_type: String,
    weight: f64,
    stability: f64,
}

#[derive(Serialize)]
struct RetrieveResult {
    atoms: Vec<AtomSummary>,
    total: usize,
    query: String,
    method: String,
    seeds: usize,
    depth: usize,
    relations: Vec<RelationView>,
}

#[derive(Serialize)]
struct RelationsResult {
    atom_id: String,
    outgoing: Vec<RelationEntry>,
    incoming: Vec<RelationEntry>,
}

#[derive(Serialize)]
struct RelationEntry {
    rel_type: String,
    weight: f64,
    stability: f64,
    dst_atom: Option<AtomSummary>,
    src_atom: Option<AtomSummary>,
}

#[derive(Serialize)]
struct IngestResponse {
    atoms_created: usize,
    relations_found: usize,
    relations_created: usize,
    entity_names: Vec<String>,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Serialize)]
struct DeleteResponse {
    id: String,
    deleted: bool,
}

#[derive(Deserialize)]
struct IngestRequest {
    text: String,
}

#[derive(Deserialize)]
struct AddAtomRequest {
    name: String,
    #[serde(rename = "type", default = "default_atom_type")]
    atom_type: String,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tags: Option<String>,
    #[serde(default)]
    salience: Option<f64>,
    #[serde(default)]
    attributes: HashMap<String, String>,
}

#[derive(Deserialize)]
struct UpdateAtomRequest {
    name: Option<String>,
    #[serde(rename = "type")]
    atom_type: Option<String>,
    content: Option<String>,
    tags: Option<String>,
    salience: Option<f64>,
    confidence: Option<f64>,
    state: Option<String>,
    #[serde(default)]
    attributes: HashMap<String, String>,
}

#[derive(Deserialize)]
struct AddRelationRequest {
    src: String,
    dst: String,
    #[serde(rename = "type")]
    rel_type: String,
}

fn default_atom_type() -> String {
    "Concept".to_string()
}

fn atom_to_summary(atom: &dcma_core::atom::CognitiveAtom, rank: Option<f64>, activation: Option<f64>) -> AtomSummary {
    AtomSummary {
        id: atom.id.to_string(),
        atom_type: format!("{:?}", atom.atom_type),
        name: atom.canonical.clone(),
        state: format!("{:?}", atom.state),
        salience: atom.salience.get().to_string(),
        confidence: atom.confidence.get().to_string(),
        content: atom.attributes.get("content").cloned(),
        tags: atom.attributes.get("tags").cloned(),
        rank,
        activation,
        created_at: None,
        updated_at: None,
    }
}

fn parse_atom_type(s: &str) -> AtomType {
    match s.to_lowercase().as_str() {
        "entity" => AtomType::Entity,
        "procedure" => AtomType::Procedure,
        "event" => AtomType::Event,
        "abstraction" => AtomType::Abstraction,
        _ => AtomType::Concept,
    }
}

fn parse_atom_state(s: &str) -> Option<AtomState> {
    match s.to_lowercase().as_str() {
        "active" => Some(AtomState::Active),
        "dormant" => Some(AtomState::Dormant),
        "frozen" => Some(AtomState::Frozen),
        "consolidated" => Some(AtomState::Consolidated),
        _ => None,
    }
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

async fn get_state(State(rt): State<SharedPgRuntime>) -> Json<serde_json::Value> {
    let rt = rt.lock().await;
    let state = rt.state();
    Json(serde_json::json!({
        "atoms": state.graph.atom_count(),
        "relations": state.graph.relation_count(),
        "tick": state.tick,
    }))
}


/// Find an existing atom by name (case-insensitive exact match).
fn find_atom_by_name(rt: &dcma_runtime::PgRuntime, name: &str) -> Option<dcma_core::atom::CognitiveAtom> {
    let name_lower = name.to_lowercase();
    rt.state().graph.atoms()
        .find_map(|(_, atom)| {
            if atom.canonical.to_lowercase() == name_lower {
                Some(atom.clone())
            } else {
                None
            }
        })
}

/// Find all atoms whose names are prefixes or duplicates of the given name.
fn find_similar_atoms(rt: &dcma_runtime::PgRuntime, name: &str) -> Vec<dcma_core::atom::CognitiveAtom> {
    let name_lower = name.to_lowercase();
    rt.state().graph.atoms()
        .filter_map(|(_, atom)| {
            let atom_lower = atom.canonical.to_lowercase();
            if atom_lower == name_lower
                || (name_lower.len() >= 4 && atom_lower.starts_with(&name_lower))
                || (atom_lower.len() >= 4 && name_lower.starts_with(&atom_lower))
            {
                Some(atom.clone())
            } else {
                None
            }
        })
        .collect()
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
    let extracted_relations = engine.extract_relations(&extracted, &req.text);
    let mut rt = rt.lock().await;
    let mut entity_names = Vec::new();
    let mut atoms_created = 0;
    let mut atoms_merged = 0;
    let mut relations_created = 0;
    let mut entity_position_to_id: Vec<(usize, AtomId)> = Vec::new();

    for entity in &extracted {
        let atom_type = match entity.entity_type.as_str() {
            "Entity" => AtomType::Entity,
            "Procedure" => AtomType::Procedure,
            "Event" => AtomType::Event,
            "Abstraction" => AtomType::Abstraction,
            _ => AtomType::Concept,
        };

        if let Some(existing) = find_atom_by_name(&rt, &entity.name) {
            let existing_content = existing.attributes.get("content").map(|s| s.len()).unwrap_or(0);
            if entity.name.len() > existing_content {
                if let Some(atom) = rt.state_mut().graph.get_atom_mut(existing.id) {
                    atom.attributes.insert("content".to_string(), entity.name.clone());
                }
            }
            entity_names.push(format!("{} (merged)", entity.name));
            entity_position_to_id.push((entity.position, existing.id));
            atoms_merged += 1;
        } else {
            let similar = find_similar_atoms(&rt, &entity.name);
            if similar.len() > 1 {
                let best = similar.iter().max_by_key(|a| {
                    a.salience.get().to_string().parse::<f64>().unwrap_or(0.0) as i64
                }).unwrap();
                if let Some(atom) = rt.state_mut().graph.get_atom_mut(best.id) {
                    atom.attributes.insert("content".to_string(), entity.name.clone());
                }
                entity_names.push(format!("{} (merged with similar)", entity.name));
                entity_position_to_id.push((entity.position, best.id));
                atoms_merged += 1;
            } else {
                let id = AtomId::new();
                let mut atom = dcma_core::atom::CognitiveAtom::new(id, atom_type, entity.name.clone());
                atom.attributes.insert("content".to_string(), entity.name.clone());
                rt.state_mut().graph.add_atom(atom);
                entity_names.push(entity.name.clone());
                entity_position_to_id.push((entity.position, id));
                atoms_created += 1;
            }
        }
    }

    for relation in &extracted_relations {
        if relation.src_idx < entity_position_to_id.len()
            && relation.dst_idx < entity_position_to_id.len()
        {
            let src = entity_position_to_id[relation.src_idx].1;
            let dst = entity_position_to_id[relation.dst_idx].1;
            if src == dst {
                continue;
            }
            let rel_type = parse_relation_type(&relation.relation_type);
            let _ = rt.state_mut().graph.add_relation(src, dst, rel_type);
            relations_created += 1;
        }
    }

    let state = rt.state().clone();
    if let Err(e) = rt.storage().save_state(&state).await {
        tracing::warn!("Failed to persist after ingest: {}", e);
    }
    Ok(Json(IngestResponse {
        atoms_created: atoms_created + atoms_merged,
        relations_found: extracted_relations.len(),
        relations_created,
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
    let atom_type = parse_atom_type(&req.atom_type);
    let mut rt = rt.lock().await;

    // Entity resolution: check if atom with this name already exists
    if let Some(existing) = find_atom_by_name(&rt, &req.name) {
        // Update existing atom
        let atom = rt.state_mut().graph.get_atom_mut(existing.id).unwrap();
        let mut attrs = req.attributes.clone();
        if let Some(c) = req.content {
            attrs.insert("content".to_string(), c);
        }
        if let Some(t) = req.tags {
            attrs.insert("tags".to_string(), t);
        }
        if !attrs.is_empty() {
            atom.attributes = attrs;
        }
        if let Some(sal) = req.salience {
            use rust_decimal::Decimal;
            atom.salience = Salience::new(Decimal::try_from(sal).unwrap_or(Decimal::ZERO));
        }
        atom.atom_type = atom_type;
        let summary = atom_to_summary(atom, None, None);
        let state = rt.state().clone();
        if let Err(e) = rt.storage().save_state(&state).await {
            tracing::warn!("Failed to persist after remember: {}", e);
        }
        return Ok(Json(summary));
    }

    let id = AtomId::new();
    let mut atom = dcma_core::atom::CognitiveAtom::new(id, atom_type, req.name.clone());
    let mut attrs = req.attributes.clone();
    if let Some(c) = req.content {
        attrs.insert("content".to_string(), c);
    }
    if let Some(t) = req.tags {
        attrs.insert("tags".to_string(), t);
    }
    if !attrs.is_empty() {
        atom.attributes = attrs;
    }
    if let Some(sal) = req.salience {
        use rust_decimal::Decimal;
        atom.salience = Salience::new(Decimal::try_from(sal).unwrap_or(Decimal::ZERO));
    }
    let summary = atom_to_summary(&atom, None, None);
    rt.state_mut().graph.add_atom(atom);
    let state = rt.state().clone();
    if let Err(e) = rt.storage().save_state(&state).await {
        tracing::warn!("Failed to persist after remember: {}", e);
    }
    Ok(Json(summary))
}

async fn update_atom(
    State(rt): State<SharedPgRuntime>,
    Path(id): Path<String>,
    Json(req): Json<UpdateAtomRequest>,
) -> Result<Json<AtomSummary>, (StatusCode, Json<ErrorResponse>)> {
    let uuid = Uuid::parse_str(&id).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: "Invalid UUID".into() }))
    })?;
    let atom_id = AtomId(uuid);
    let mut rt = rt.lock().await;
    let atom = rt.state_mut().graph.get_atom_mut(atom_id).ok_or_else(|| {
        (StatusCode::NOT_FOUND, Json(ErrorResponse { error: format!("Atom {} not found", id) }))
    })?;
    if let Some(name) = req.name { atom.canonical = name; }
    if let Some(t) = req.atom_type { atom.atom_type = parse_atom_type(&t); }
    if let Some(sal) = req.salience {
        use rust_decimal::Decimal;
        atom.salience = Salience::new(Decimal::try_from(sal).unwrap_or(Decimal::ZERO));
    }
    if let Some(conf) = req.confidence {
        use rust_decimal::Decimal;
        atom.confidence = Confidence::new(Decimal::try_from(conf).unwrap_or(Decimal::ZERO));
    }
    if let Some(st) = req.state {
        if let Some(state) = parse_atom_state(&st) { atom.state = state; }
    }
    if let Some(c) = req.content { atom.attributes.insert("content".to_string(), c); }
    if let Some(t) = req.tags { atom.attributes.insert("tags".to_string(), t); }
    for (k, v) in req.attributes { atom.attributes.insert(k, v); }
    let summary = atom_to_summary(atom, None, None);
    let state = rt.state().clone();
    if let Err(e) = rt.storage().save_state(&state).await {
        tracing::warn!("Failed to persist after update: {}", e);
    }
    Ok(Json(summary))
}

async fn delete_atom(
    State(rt): State<SharedPgRuntime>,
    Path(id): Path<String>,
) -> Result<Json<DeleteResponse>, (StatusCode, Json<ErrorResponse>)> {
    let uuid = Uuid::parse_str(&id).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: "Invalid UUID".into() }))
    })?;
    let atom_id = AtomId(uuid);
    let mut rt = rt.lock().await;
    let removed = rt.state_mut().graph.remove_atom(atom_id).is_some();
    if !removed {
        return Err((StatusCode::NOT_FOUND, Json(ErrorResponse { error: format!("Atom {} not found", id) })));
    }
    let state = rt.state().clone();
    if let Err(e) = rt.storage().save_state(&state).await {
        tracing::warn!("Failed to persist after delete: {}", e);
    }
    Ok(Json(DeleteResponse { id, deleted: true }))
}

async fn get_atom(
    State(rt): State<SharedPgRuntime>,
    Path(id): Path<String>,
) -> Result<Json<AtomSummary>, (StatusCode, Json<ErrorResponse>)> {
    let uuid = Uuid::parse_str(&id).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: "Invalid UUID".into() }))
    })?;
    let atom_id = AtomId(uuid);
    let rt = rt.lock().await;
    let state = rt.state();
    match state.graph.get_atom(atom_id) {
        Some(atom) => Ok(Json(atom_to_summary(atom, None, None))),
        None => Err((StatusCode::NOT_FOUND, Json(ErrorResponse { error: format!("Atom {} not found", id) }))),
    }
}

async fn get_relations(
    State(rt): State<SharedPgRuntime>,
    Path(id): Path<String>,
) -> Result<Json<RelationsResult>, (StatusCode, Json<ErrorResponse>)> {
    let uuid = Uuid::parse_str(&id).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: "Invalid UUID".into() }))
    })?;
    let atom_id = AtomId(uuid);
    let rt = rt.lock().await;
    let state = rt.state();
    if state.graph.get_atom(atom_id).is_none() {
        return Err((StatusCode::NOT_FOUND, Json(ErrorResponse { error: format!("Atom {} not found", id) })));
    }
    let mut outgoing = Vec::new();
    for (_, dst_atom, rel) in state.graph.outbound_neighbors(atom_id) {
        outgoing.push(RelationEntry {
            rel_type: format!("{:?}", rel.relation_type),
            weight: rel.weight.get().try_into().unwrap_or(0.5),
            stability: rel.stability.get().try_into().unwrap_or(0.0),
            dst_atom: Some(atom_to_summary(dst_atom, None, None)),
            src_atom: None,
        });
    }
    let mut incoming = Vec::new();
    for (_, src_atom, rel) in state.graph.inbound_neighbors(atom_id) {
        incoming.push(RelationEntry {
            rel_type: format!("{:?}", rel.relation_type),
            weight: rel.weight.get().try_into().unwrap_or(0.5),
            stability: rel.stability.get().try_into().unwrap_or(0.0),
            dst_atom: None,
            src_atom: Some(atom_to_summary(src_atom, None, None)),
        });
    }
    Ok(Json(RelationsResult { atom_id: id, outgoing, incoming }))
}

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
    let mut scored: Vec<(f64, AtomSummary)> = state
        .graph
        .atoms()
        .filter_map(|(_, atom)| {
            let name_lower = atom.canonical.to_lowercase();
            let content = atom.attributes.get("content").map(|c| c.to_lowercase()).unwrap_or_default();
            let tags = atom.attributes.get("tags").map(|t| t.to_lowercase()).unwrap_or_default();
            let mut score = 0.0;
            if name_lower == query_lower { score = 100.0; }
            else if name_lower.starts_with(&query_lower) { score = 80.0; }
            else if name_lower.contains(&query_lower) { score = 50.0; }
            else if content.contains(&query_lower) { score = 30.0; }
            else if tags.contains(&query_lower) { score = 20.0; }
            else {
                for word in query_lower.split_whitespace() {
                    if name_lower.contains(word) { score += 10.0; }
                    if content.contains(word) { score += 5.0; }
                    if tags.contains(word) { score += 3.0; }
                }
            }
            if score > 0.0 { Some((score, atom_to_summary(atom, Some(score), None))) } else { None }
        })
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let total = scored.len();
    let atoms: Vec<AtomSummary> = scored.into_iter().skip(offset).take(limit).map(|(_, a)| a).collect();
    Json(SearchResult { atoms, total, query, method: "seeds".into() })
}

async fn retrieve(
    State(rt): State<SharedPgRuntime>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Json<RetrieveResult> {
    let query = params.get("q").cloned().unwrap_or_default();
    let limit: usize = params.get("limit").and_then(|l| l.parse().ok()).unwrap_or(20);
    let depth: usize = params.get("depth").and_then(|d| d.parse().ok()).unwrap_or(3);
    let decay: f64 = params.get("decay").and_then(|d| d.parse().ok()).unwrap_or(0.7);

    let mut rt = rt.lock().await;
    let state = rt.state();
    let query_lower = query.to_lowercase();

    // 1. Find seed atoms
    let seed_ids: Vec<AtomId> = state
        .graph
        .atoms()
        .filter_map(|(_, atom)| {
            let name_lower = atom.canonical.to_lowercase();
            if name_lower.starts_with(&query_lower) || name_lower.contains(&query_lower) {
                Some(atom.id)
            } else {
                None
            }
        })
        .take(5)
        .collect();

    let seeds_count = seed_ids.len();

    // 2. Activation spreading via BFS with decay
    let mut activation_map: HashMap<AtomId, f64> = HashMap::new();
    let mut visited: HashMap<AtomId, usize> = HashMap::new();
    let mut queue: std::collections::VecDeque<(AtomId, f64, usize)> = std::collections::VecDeque::new();
    let mut relation_paths: Vec<RelationView> = Vec::new();

    for seed_id in &seed_ids {
        activation_map.insert(*seed_id, 1.0);
        visited.insert(*seed_id, 0);
        queue.push_back((*seed_id, 1.0, 0));
    }

    while let Some((current_id, energy, hop)) = queue.pop_front() {
        if hop >= depth || energy < 0.01 { continue; }
        for (_, neighbor_atom, rel) in state.graph.outbound_neighbors(current_id) {
            let neighbor_id = neighbor_atom.id;
            let edge_weight: f64 = rel.weight.get().try_into().unwrap_or(0.5);
            let propagated = energy * edge_weight * decay;
            let entry = activation_map.entry(neighbor_id).or_insert(0.0);
            *entry = (*entry + propagated).min(1.0);
            if !visited.contains_key(&neighbor_id) || visited[&neighbor_id] > hop + 1 {
                visited.insert(neighbor_id, hop + 1);
                queue.push_back((neighbor_id, propagated, hop + 1));
            }
            let src_name = state.graph.get_atom(current_id).map(|a| a.canonical.clone()).unwrap_or_default();
            relation_paths.push(RelationView {
                src_id: current_id.to_string(),
                dst_id: neighbor_id.to_string(),
                src_name,
                dst_name: neighbor_atom.canonical.clone(),
                rel_type: format!("{:?}", rel.relation_type),
                weight: edge_weight,
                stability: rel.stability.get().try_into().unwrap_or(0.0),
            });
        }
        for (_, src_atom, rel) in state.graph.inbound_neighbors(current_id) {
            let src_id = src_atom.id;
            let edge_weight: f64 = rel.weight.get().try_into().unwrap_or(0.5);
            let propagated = energy * edge_weight * decay * 0.5;
            let entry = activation_map.entry(src_id).or_insert(0.0);
            *entry = (*entry + propagated).min(1.0);
            if !visited.contains_key(&src_id) || visited[&src_id] > hop + 1 {
                visited.insert(src_id, hop + 1);
                queue.push_back((src_id, propagated, hop + 1));
            }
        }
    }

    // 3. Build result ranked by activation energy
    let mut activated: Vec<(f64, AtomSummary)> = activation_map
        .iter()
        .filter_map(|(id, energy)| {
            state.graph.get_atom(*id).map(|atom| (*energy, atom_to_summary(atom, None, Some(*energy))))
        })
        .collect();
    activated.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let total = activated.len();
    let atoms: Vec<AtomSummary> = activated.into_iter().take(limit).map(|(_, a)| a).collect();

    let mut seen_rels = std::collections::HashSet::new();
    relation_paths.retain(|r| {
        let key = (r.src_id.clone(), r.dst_id.clone(), r.rel_type.clone());
        seen_rels.insert(key)
    });

    Json(RetrieveResult {
        atoms, total, query, method: "activation".into(), seeds: seeds_count, depth,
        relations: relation_paths.into_iter().take(limit * 2).collect(),
    })
}

async fn graph_search(
    State(rt): State<SharedPgRuntime>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Json<GraphResult> {
    let query = params.get("q").cloned().unwrap_or_default();
    let limit: usize = params.get("limit").and_then(|l| l.parse().ok()).unwrap_or(10);
    let depth: usize = params.get("depth").and_then(|d| d.parse().ok()).unwrap_or(3);
    let decay: f64 = params.get("decay").and_then(|d| d.parse().ok()).unwrap_or(0.7);
    let rt = rt.lock().await;
    let state = rt.state();
    let query_lower = query.to_lowercase();

    let matched_ids: Vec<AtomId> = state
        .graph
        .atoms()
        .filter_map(|(_, atom)| {
            if atom.canonical.to_lowercase().contains(&query_lower)
                || atom.attributes.get("content").map_or(false, |c| c.to_lowercase().contains(&query_lower))
            { Some(atom.id) } else { None }
        })
        .take(limit)
        .collect();

    let query_atoms: Vec<AtomSummary> = matched_ids
        .iter()
        .filter_map(|id| state.graph.get_atom(*id).map(|a| atom_to_summary(a, None, None)))
        .collect();

    let mut activation_map: HashMap<AtomId, f64> = HashMap::new();
    let mut visited: HashMap<AtomId, usize> = HashMap::new();
    let mut queue: std::collections::VecDeque<(AtomId, f64, usize)> = std::collections::VecDeque::new();
    let mut relations = Vec::new();

    for id in &matched_ids {
        activation_map.insert(*id, 1.0);
        visited.insert(*id, 0);
        queue.push_back((*id, 1.0, 0));
    }

    while let Some((current_id, energy, hop)) = queue.pop_front() {
        if hop >= depth || energy < 0.01 { continue; }
        for (_, neighbor_atom, rel) in state.graph.outbound_neighbors(current_id) {
            let neighbor_id = neighbor_atom.id;
            let edge_weight: f64 = rel.weight.get().try_into().unwrap_or(0.5);
            let propagated = energy * edge_weight * decay;
            let entry = activation_map.entry(neighbor_id).or_insert(0.0);
            *entry = (*entry + propagated).min(1.0);
            if !visited.contains_key(&neighbor_id) {
                visited.insert(neighbor_id, hop + 1);
                queue.push_back((neighbor_id, propagated, hop + 1));
            }
            let src_name = state.graph.get_atom(current_id).map(|a| a.canonical.clone()).unwrap_or_default();
            relations.push(RelationView {
                src_id: current_id.to_string(),
                dst_id: neighbor_id.to_string(),
                src_name,
                dst_name: neighbor_atom.canonical.clone(),
                rel_type: format!("{:?}", rel.relation_type),
                weight: edge_weight,
                stability: rel.stability.get().try_into().unwrap_or(0.0),
            });
        }
    }

    for id in &matched_ids { activation_map.remove(id); }

    let mut related: Vec<(f64, AtomSummary)> = activation_map
        .iter()
        .filter_map(|(id, energy)| {
            state.graph.get_atom(*id).map(|a| (*energy, atom_to_summary(a, None, Some(*energy))))
        })
        .collect();
    related.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let related: Vec<AtomSummary> = related.into_iter().take(limit).map(|(_, a)| a).collect();

    Json(GraphResult { query_atoms, related, relations })
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
        .map(|(_, atom)| atom_to_summary(atom, None, None))
        .collect();
    atoms.sort_by(|a, b| a.name.cmp(&b.name));
    let total = atoms.len();
    let atoms: Vec<_> = atoms.into_iter().skip(offset).take(limit).collect();
    Json(SearchResult { atoms, total, query: "".into(), method: "list".into() })
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
        .route("/retrieve", get(retrieve))
        .route("/graph", get(graph_search))
        .route("/atoms", get(list_atoms))
        .route("/atoms/:id", get(get_atom).patch(update_atom).delete(delete_atom))
        .route("/atoms/:id/relations", get(get_relations))
        .route("/relations", post(add_relation))
        .route("/tick", post(execute_tick))
        .with_state(state);

    let port: u16 = env::var("DCMA_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(3030);
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    tracing::info!(%addr, "Server listening");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service()).await.unwrap();
}
