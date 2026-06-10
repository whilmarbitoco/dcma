use dcma_core::types::*;
use dcma_core::graph::CognitiveGraph;
use petgraph::stable_graph::NodeIndex;
use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet, VecDeque};

/// Structural similarity between a node and a query context.
pub fn structural_similarity(
    graph: &CognitiveGraph,
    node_id: AtomId,
    query_ids: &[AtomId],
    k: usize,
) -> Decimal {
    if query_ids.is_empty() {
        return Decimal::ZERO;
    }

    let node_neighbors = graph.k_hop_neighbors(node_id, k);
    let mut query_union = HashMap::new();
    for qid in query_ids {
        for (nidx, dist) in graph.k_hop_neighbors(*qid, k) {
            query_union.entry(nidx).or_insert(dist);
        }
    }

    if node_neighbors.is_empty() || query_union.is_empty() {
        return Decimal::ZERO;
    }

    let mut intersection_size = 0usize;
    for (nidx, _) in &node_neighbors {
        if query_union.contains_key(nidx) {
            intersection_size += 1;
        }
    }

    let union_size = node_neighbors.len() + query_union.len() - intersection_size;
    if union_size == 0 {
        return Decimal::ZERO;
    }

    let jaccard = Decimal::from(intersection_size) / Decimal::from(union_size);

    // Path weight boost for direct connections
    let mut path_boost = Decimal::ZERO;
    for qid in query_ids {
        for (_, rel) in graph.relations_between(node_id, *qid) {
            if rel.weight.get() > path_boost {
                path_boost = rel.weight.get();
            }
        }
        for (_, rel) in graph.relations_between(*qid, node_id) {
            if rel.weight.get() > path_boost {
                path_boost = rel.weight.get();
            }
        }
    }

    let diameter = graph.diameter().unwrap_or(1);
    let diameter_factor = if diameter > 0 {
        Decimal::ONE / Decimal::from(diameter)
    } else {
        Decimal::ONE
    };

    jaccard * (Decimal::ONE + path_boost) * diameter_factor
}

/// Relevance functional R(G_q, S, q) with 5 terms.
pub fn relevance(
    graph: &CognitiveGraph,
    subgraph_nodes: &[NodeIndex],
    query_ids: &[AtomId],
    activations: &HashMap<AtomId, Activation>,
    current_tick: u64,
) -> Decimal {
    if subgraph_nodes.is_empty() {
        return Decimal::ZERO;
    }

    let n = Decimal::from(subgraph_nodes.len());

    // Term 1: Contextual coherence
    let mut coherence = Decimal::ZERO;
    for idx in subgraph_nodes {
        if let Some(atom) = graph.get_atom_by_index(*idx) {
            coherence += structural_similarity(graph, atom.id, query_ids, 2);
        }
    }
    coherence /= n;

    // Term 2: Salience
    let mut salience = Decimal::ZERO;
    for idx in subgraph_nodes {
        if let Some(atom) = graph.get_atom_by_index(*idx) {
            salience += atom.salience.get();
        }
    }
    salience /= n;

    // Term 3: Activation resonance
    let mut activation_res = Decimal::ZERO;
    for idx in subgraph_nodes {
        if let Some(atom) = graph.get_atom_by_index(*idx) {
            if let Some(act) = activations.get(&atom.id) {
                activation_res += act.get();
            }
        }
    }
    activation_res /= n;

    // Term 4: Temporal relevance
    let mut temporal = Decimal::ZERO;
    let decay_rate = Decimal::from_str_exact("0.01").unwrap();
    for idx in subgraph_nodes {
        if let Some(atom) = graph.get_atom_by_index(*idx) {
            let ticks_since = current_tick.saturating_sub(atom.t_last_accessed);
            let recency = exp_approx(-decay_rate * Decimal::from(ticks_since));
            temporal += recency;
        }
    }
    temporal /= n;

    // Term 5: Traversal cost
    let graph_diameter = graph.diameter().unwrap_or(1);
    let traversal_cost = if graph_diameter > 0 && subgraph_nodes.len() > 1 {
        let mut max_dist = 0usize;
        for (i, idx_a) in subgraph_nodes.iter().enumerate() {
            if let Some(atom_a) = graph.get_atom_by_index(*idx_a) {
                for idx_b in &subgraph_nodes[i + 1..] {
                    if let Some(atom_b) = graph.get_atom_by_index(*idx_b) {
                        let dist = bfs_distance(graph, atom_a.id, atom_b.id, subgraph_nodes);
                        if dist != usize::MAX && dist > max_dist {
                            max_dist = dist;
                        }
                    }
                }
            }
        }
        Decimal::from(max_dist) / Decimal::from(graph_diameter)
    } else {
        Decimal::ZERO
    };

    let alpha = Decimal::from_str_exact("0.35").unwrap();
    let beta = Decimal::from_str_exact("0.20").unwrap();
    let gamma = Decimal::from_str_exact("0.20").unwrap();
    let delta = Decimal::from_str_exact("0.15").unwrap();
    let lambda = Decimal::from_str_exact("0.10").unwrap();

    alpha * coherence + beta * salience + gamma * activation_res + delta * temporal
        - lambda * traversal_cost
}

fn bfs_distance(
    graph: &CognitiveGraph,
    from: AtomId,
    to: AtomId,
    allowed: &[NodeIndex],
) -> usize {
    if from == to {
        return 0;
    }
    let allowed_set: HashSet<NodeIndex> = allowed.iter().copied().collect();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back((from, 0usize));
    visited.insert(from);

    while let Some((current, dist)) = queue.pop_front() {
        for (nidx, neighbor_atom) in graph.neighbors(current) {
            if !allowed_set.contains(&nidx) || visited.contains(&neighbor_atom.id) {
                continue;
            }
            if neighbor_atom.id == to {
                return dist + 1;
            }
            visited.insert(neighbor_atom.id);
            queue.push_back((neighbor_atom.id, dist + 1));
        }
    }
    usize::MAX
}

fn exp_approx(x: Decimal) -> Decimal {
    let x = x.clamp(Decimal::from(-10), Decimal::from(10));
    let mut sum = Decimal::ONE;
    let mut term = Decimal::ONE;
    for n in 1..=20 {
        term = term * x / Decimal::from(n);
        sum += term;
        if term.abs() < Decimal::new(1, 12) {
            break;
        }
    }
    sum.max(Decimal::ZERO)
}

/// Bounded beam search for optimal subgraph retrieval.
pub fn retrieve(
    graph: &CognitiveGraph,
    query_ids: &[AtomId],
    activations: &HashMap<AtomId, Activation>,
    current_tick: u64,
    config: &EngineConfig,
) -> RetrievalResult {
    let d_max = config.max_retrieval_depth;
    let n_max = config.max_subgraph_size;
    let beam_width = config.beam_width;

    let query_indices: Vec<NodeIndex> = query_ids
        .iter()
        .filter_map(|id| graph.node_index(*id))
        .collect();

    if query_indices.is_empty() {
        return RetrievalResult {
            nodes: Vec::new(),
            relevance_score: Decimal::ZERO,
            metadata: RetrievalMetadata::default(),
        };
    }

    let mut frontier: Vec<(Vec<NodeIndex>, Decimal)> = query_indices
        .iter()
        .map(|idx| (vec![*idx], Decimal::ZERO))
        .collect();

    let mut best_subgraph = frontier[0].0.clone();
    let mut best_score = Decimal::ZERO;

    for _depth in 0..d_max {
        let mut candidates: Vec<(Vec<NodeIndex>, Decimal)> = Vec::new();

        for (subgraph_nodes, _) in &frontier {
            let mut neighbor_set = HashSet::new();
            for idx in subgraph_nodes {
                if let Some(atom) = graph.get_atom_by_index(*idx) {
                    for (nidx, _) in graph.neighbors(atom.id) {
                        if !subgraph_nodes.contains(&nidx) {
                            neighbor_set.insert(nidx);
                        }
                    }
                }
            }

            for nidx in neighbor_set {
                if subgraph_nodes.len() >= n_max {
                    continue;
                }
                let mut new_subgraph = subgraph_nodes.clone();
                new_subgraph.push(nidx);

                let score = relevance(graph, &new_subgraph, query_ids, activations, current_tick);
                candidates.push((new_subgraph, score));
            }
        }

        if candidates.is_empty() {
            break;
        }

        candidates.sort_by(|a, b| b.1.cmp(&a.1));
        candidates.truncate(beam_width);

        if let Some((nodes, score)) = candidates.first() {
            if *score > best_score {
                best_score = *score;
                best_subgraph = nodes.clone();
            }
        }

        frontier = candidates;
    }

    RetrievalResult {
        nodes: best_subgraph,
        relevance_score: best_score,
        metadata: RetrievalMetadata {
            depth_searched: d_max,
            beam_width,
            query_count: query_ids.len(),
        },
    }
}

#[derive(Debug, Clone)]
pub struct RetrievalResult {
    pub nodes: Vec<NodeIndex>,
    pub relevance_score: Decimal,
    pub metadata: RetrievalMetadata,
}

#[derive(Debug, Clone, Default)]
pub struct RetrievalMetadata {
    pub depth_searched: usize,
    pub beam_width: usize,
    pub query_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use dcma_core::atom::CognitiveAtom;

    fn make_test_graph() -> (CognitiveGraph, AtomId, AtomId, AtomId) {
        let mut g = CognitiveGraph::new();

        let postgres = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "postgres".to_string());
        let database = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "database".to_string());
        let storage = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "storage".to_string());
        let indexing = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "indexing".to_string());

        let id_pg = postgres.id;
        let id_db = database.id;
        let id_st = storage.id;
        let id_ix = indexing.id;

        g.add_atom(postgres);
        g.add_atom(database);
        g.add_atom(storage);
        g.add_atom(indexing);

        g.add_relation_with_weight(
            id_pg, id_db, RelationType::InstanceOf,
            Weight::new(Decimal::from_str_exact("0.9").unwrap()),
        ).unwrap();
        g.add_relation_with_weight(
            id_db, id_st, RelationType::Contains,
            Weight::new(Decimal::from_str_exact("0.8").unwrap()),
        ).unwrap();
        g.add_relation_with_weight(
            id_db, id_ix, RelationType::Contains,
            Weight::new(Decimal::from_str_exact("0.7").unwrap()),
        ).unwrap();

        (g, id_pg, id_db, id_st)
    }

    #[test]
    fn test_structural_similarity_direct_connection() {
        let (g, id_pg, id_db, _) = make_test_graph();
        let sim = structural_similarity(&g, id_pg, &[id_db], 2);
        assert!(sim > Decimal::ZERO, "Direct connection should have non-zero similarity");
    }

    #[test]
    fn test_structural_similarity_self() {
        let (g, id_pg, _, _) = make_test_graph();
        let sim = structural_similarity(&g, id_pg, &[id_pg], 2);
        assert!(sim > Decimal::ZERO, "Self-similarity should be non-zero");
    }

    #[test]
    fn test_structural_similarity_empty_query() {
        let (g, id_pg, _, _) = make_test_graph();
        let sim = structural_similarity(&g, id_pg, &[], 2);
        assert_eq!(sim, Decimal::ZERO);
    }

    #[test]
    fn test_relevance_nonzero() {
        let (g, id_pg, id_db, _) = make_test_graph();

        let pg_idx = g.node_index(id_pg).unwrap();
        let db_idx = g.node_index(id_db).unwrap();

        let mut activations = HashMap::new();
        activations.insert(id_pg, Activation::new(Decimal::ONE));
        activations.insert(id_db, Activation::new(Decimal::from_str_exact("0.8").unwrap()));

        let score = relevance(&g, &[pg_idx, db_idx], &[id_pg], &activations, 0);
        assert!(score > Decimal::ZERO, "Relevance should be non-zero for connected activated nodes");
    }

    #[test]
    fn test_relevance_empty_subgraph() {
        let (g, _, _, _) = make_test_graph();
        let score = relevance(&g, &[], &[AtomId::new()], &HashMap::new(), 0);
        assert_eq!(score, Decimal::ZERO);
    }

    #[test]
    fn test_retrieve_basic() {
        let (g, id_pg, id_db, _) = make_test_graph();
        let config = EngineConfig::default();

        let mut activations = HashMap::new();
        activations.insert(id_pg, Activation::new(Decimal::ONE));
        activations.insert(id_db, Activation::new(Decimal::from_str_exact("0.8").unwrap()));

        let result = retrieve(&g, &[id_pg], &activations, 0, &config);

        assert!(!result.nodes.is_empty(), "Should retrieve at least the query node");
        assert!(result.relevance_score > Decimal::ZERO, "Should have non-zero relevance");

        let pg_idx = g.node_index(id_pg).unwrap();
        assert!(result.nodes.contains(&pg_idx), "Query node should be in result");
    }

    #[test]
    fn test_retrieve_expands_from_query() {
        let (g, id_pg, id_db, id_st) = make_test_graph();
        let config = EngineConfig::default();

        let mut activations = HashMap::new();
        activations.insert(id_pg, Activation::new(Decimal::ONE));
        activations.insert(id_db, Activation::new(Decimal::from_str_exact("0.9").unwrap()));
        activations.insert(id_st, Activation::new(Decimal::from_str_exact("0.7").unwrap()));

        let result = retrieve(&g, &[id_pg], &activations, 0, &config);
        assert!(result.nodes.len() >= 2, "Should expand to connected nodes, got {}", result.nodes.len());
    }

    #[test]
    fn test_retrieve_empty_query() {
        let (g, _, _, _) = make_test_graph();
        let config = EngineConfig::default();
        let result = retrieve(&g, &[], &HashMap::new(), 0, &config);
        assert!(result.nodes.is_empty());
    }

    #[test]
    fn test_exp_approx() {
        let result = exp_approx(Decimal::ZERO);
        assert!((result - Decimal::ONE).abs() < Decimal::from_str_exact("0.001").unwrap());

        let result = exp_approx(Decimal::from(-1));
        assert!(result > Decimal::from_str_exact("0.3").unwrap());
        assert!(result < Decimal::from_str_exact("0.4").unwrap());
    }
}
