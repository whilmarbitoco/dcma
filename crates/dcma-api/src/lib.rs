use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json, Router,
    routing::{get, post},
};
use dcma_core::atom::CognitiveAtom;
use dcma_core::types::*;
use dcma_retrieval::CognitiveField;
use dcma_runtime::{Runtime, Stimulus, TickResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type ApiState = Arc<Mutex<Runtime>>;

#[derive(Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
}

#[derive(Serialize, Deserialize)]
pub struct StateSummary {
    pub nodes: usize,
    pub edges: usize,
    pub entropy: f64,
    pub tick: u64,
}

#[derive(Serialize, Deserialize)]
pub struct StatsResponse {
    pub atom_count: usize,
    pub relation_count: usize,
    pub tick: u64,
    pub degree_distribution: HashMap<usize, usize>,
    pub diameter: Option<usize>,
}

#[derive(Deserialize)]
pub struct CreateAtomRequest {
    #[serde(rename = "type")]
    pub atom_type: AtomType,
    pub name: String,
    pub attributes: Option<HashMap<String, String>>,
}

#[derive(Deserialize)]
pub struct CreateRelationRequest {
    pub src: AtomId,
    pub dst: AtomId,
    #[serde(rename = "type")]
    pub relation_type: RelationType,
}

#[derive(Deserialize)]
pub struct QueryRequest {
    pub nodes: Vec<AtomId>,
}

#[derive(Deserialize)]
pub struct TickRequest {
    pub sources: Option<Vec<AtomId>>,
    pub query: Option<Vec<AtomId>>,
}

#[derive(Serialize)]
pub struct AtomDetail {
    pub id: AtomId,
    pub atom_type: AtomType,
    pub name: String,
    pub state: AtomState,
    pub attributes: HashMap<String, String>,
    pub activation: Option<Activation>,
    pub salience: Salience,
    pub confidence: Confidence,
}

async fn insert_atom(
    State(runtime): State<ApiState>,
    Json(req): Json<CreateAtomRequest>,
) -> Result<Json<AtomId>, StatusCode> {
    let mut rt = runtime.lock().await;
    let id = AtomId::new();
    let mut atom = CognitiveAtom::new(id, req.atom_type, req.name);
    if let Some(attrs) = req.attributes {
        atom.attributes = attrs;
    }
    rt.state_mut().graph.add_atom(atom);
    Ok(Json(id))
}

async fn create_relation(
    State(runtime): State<ApiState>,
    Json(req): Json<CreateRelationRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut rt = runtime.lock().await;
    rt.state_mut()
        .graph
        .add_relation(req.src, req.dst, req.relation_type)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(Json(serde_json::json!({"status": "created"})))
}

async fn query(
    State(runtime): State<ApiState>,
    Json(req): Json<QueryRequest>,
) -> Json<CognitiveField> {
    let rt = runtime.lock().await;
    let state = rt.state();
    let activations: HashMap<AtomId, Activation> = state.activation_values().into_iter().collect();
    let field = dcma_retrieval::retrieve(
        &state.graph,
        &req.nodes,
        &activations,
        state.tick,
        &state.config,
    );
    Json(field)
}

async fn execute_tick(
    State(runtime): State<ApiState>,
    Json(req): Json<TickRequest>,
) -> Json<TickResult> {
    let mut rt = runtime.lock().await;
    let stimulus = Stimulus {
        source_nodes: req.sources.unwrap_or_default(),
        query_context: req.query.unwrap_or_default(),
        label: "api".into(),
    };
    let result = rt.tick(&stimulus);
    Json(result)
}

async fn get_state(State(runtime): State<ApiState>) -> Json<StateSummary> {
    let rt = runtime.lock().await;
    let state = rt.state();
    Json(StateSummary {
        nodes: state.graph.atom_count(),
        edges: state.graph.relation_count(),
        entropy: 0.0,
        tick: state.tick,
    })
}

async fn get_atom(
    State(runtime): State<ApiState>,
    Path(id): Path<AtomId>,
) -> Result<Json<AtomDetail>, StatusCode> {
    let rt = runtime.lock().await;
    let state = rt.state();
    let atom = state.graph.get_atom(id).ok_or(StatusCode::NOT_FOUND)?;
    let activation = state.get_activation(id);
    Ok(Json(AtomDetail {
        id: atom.id,
        atom_type: atom.atom_type,
        name: atom.canonical.clone(),
        state: atom.state,
        attributes: atom.attributes.clone(),
        activation,
        salience: atom.salience,
        confidence: atom.confidence,
    }))
}

async fn get_stats(State(runtime): State<ApiState>) -> Json<StatsResponse> {
    let rt = runtime.lock().await;
    let state = rt.state();
    Json(StatsResponse {
        atom_count: state.graph.atom_count(),
        relation_count: state.graph.relation_count(),
        tick: state.tick,
        degree_distribution: state.graph.degree_distribution(),
        diameter: state.graph.diameter(),
    })
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
    })
}

pub fn app(runtime: Runtime) -> Router {
    let state: ApiState = Arc::new(Mutex::new(runtime));
    Router::new()
        .route("/atoms", post(insert_atom))
        .route("/relations", post(create_relation))
        .route("/query", post(query))
        .route("/tick", post(execute_tick))
        .route("/state", get(get_state))
        .route("/atoms/:id", get(get_atom))
        .route("/stats", get(get_stats))
        .route("/health", get(health))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, Method},
    };
    use dcma_core::types::EngineConfig;
    use tower::util::ServiceExt;

    #[tokio::test]
    async fn test_insert_atom() {
        let rt = Runtime::new(EngineConfig::default());
        let app = super::app(rt);

        let req_body = serde_json::json!({
            "type": "Concept",
            "name": "test_atom"
        });
        let req = Request::builder()
            .method(Method::POST)
            .uri("/atoms")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&req_body).unwrap()))
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
        let id: AtomId = serde_json::from_str(&body_str).unwrap();

        let req2 = Request::builder()
            .method(Method::GET)
            .uri(&format!("/atoms/{}", id))
            .header("content-type", "application/json")
            .body(Body::empty())
            .unwrap();
        let res2 = app.oneshot(req2).await.unwrap();
        assert_eq!(res2.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_create_relation() {
        let rt = Runtime::new(EngineConfig::default());
        let app = super::app(rt);

        // First create two atoms
        let req1 = Request::builder()
            .method(Method::POST)
            .uri("/atoms")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({"type": "Concept", "name": "a"})).unwrap(),
            ))
            .unwrap();
        let res1 = app.clone().oneshot(req1).await.unwrap();
        let body_bytes = axum::body::to_bytes(res1.into_body(), 1024 * 1024).await.unwrap();
        let id_a: AtomId = serde_json::from_slice(&body_bytes).unwrap();

        let req2 = Request::builder()
            .method(Method::POST)
            .uri("/atoms")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({"type": "Concept", "name": "b"})).unwrap(),
            ))
            .unwrap();
        let res2 = app.clone().oneshot(req2).await.unwrap();
        let body_bytes = axum::body::to_bytes(res2.into_body(), 1024 * 1024).await.unwrap();
        let id_b: AtomId = serde_json::from_slice(&body_bytes).unwrap();

        let rel_body = serde_json::json!({
            "src": id_a,
            "dst": id_b,
            "type": "Causes"
        });
        let req3 = Request::builder()
            .method(Method::POST)
            .uri("/relations")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&rel_body).unwrap()))
            .unwrap();
        let res3 = app.oneshot(req3).await.unwrap();
        assert_eq!(res3.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_query() {
        let rt = Runtime::new(EngineConfig::default());
        let app = super::app(rt);

        let req_body = serde_json::json!({"nodes": []});
        let req = Request::builder()
            .method(Method::POST)
            .uri("/query")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&req_body).unwrap()))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
        let _field: CognitiveField = serde_json::from_slice(&body_bytes).unwrap();
    }

    #[tokio::test]
    async fn test_tick() {
        let rt = Runtime::new(EngineConfig::default());
        let app = super::app(rt);

        let req_body = serde_json::json!({"sources": [], "query": []});
        let req = Request::builder()
            .method(Method::POST)
            .uri("/tick")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&req_body).unwrap()))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_state_endpoint() {
        let rt = Runtime::new(EngineConfig::default());
        let app = super::app(rt);

        let req = Request::builder()
            .method(Method::GET)
            .uri("/state")
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
        let summary: StateSummary = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(summary.nodes, 0);
        assert_eq!(summary.edges, 0);
        assert_eq!(summary.tick, 0);
    }

    #[tokio::test]
    async fn test_health() {
        let rt = Runtime::new(EngineConfig::default());
        let app = super::app(rt);

        let req = Request::builder()
            .method(Method::GET)
            .uri("/health")
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
        let health: HealthResponse = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(health.status, "ok");
    }
}
