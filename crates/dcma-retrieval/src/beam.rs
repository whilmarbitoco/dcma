use dcma_core::graph::CognitiveGraph;
use dcma_core::types::*;
use crate::relevance::{relevance, relevance_with_contradiction, compute_contradiction_flags};
use serde::{Deserialize, Serialize};

use petgraph::stable_graph::NodeIndex;
use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveField {
    pub nodes: Vec<AtomId>,
    pub relevance_score: Decimal,
    pub salience_score: Decimal,
    pub activation_score: Decimal,
    pub contradiction_flags: Vec<bool>,
    pub metadata: RetrievalMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalMetadata {
    pub traversal_depth: usize,
    pub beam_width_used: usize,
    pub query_nodes: Vec<AtomId>,
}

fn compute_field_scores(
    graph: &CognitiveGraph,
    subgraph_nodes: &[NodeIndex],
    activations: &HashMap<AtomId, Activation>,
) -> (Decimal, Decimal) {
    let n = Decimal::from(subgraph_nodes.len().max(1));
    let mut salience = Decimal::ZERO;
    let mut activation_res = Decimal::ZERO;
    for idx in subgraph_nodes {
        if let Some(atom) = graph.get_atom_by_index(*idx) {
            salience += atom.salience.get();
            if let Some(act) = activations.get(&atom.id) {
                activation_res += act.get();
            }
        }
    }
    (salience / n, activation_res / n)
}

pub fn retrieve_beam(
    graph: &CognitiveGraph,
    query_ids: &[AtomId],
    activations: &HashMap<AtomId, Activation>,
    current_tick: u64,
    config: &EngineConfig,
) -> CognitiveField {
    let d_max = config.max_retrieval_depth;
    let n_max = config.max_subgraph_size;
    let beam_width = config.beam_width;

    let query_indices: Vec<NodeIndex> = query_ids
        .iter()
        .filter_map(|id| graph.node_index(*id))
        .collect();

    if query_indices.is_empty() {
        return CognitiveField {
            nodes: Vec::new(),
            relevance_score: Decimal::ZERO,
            salience_score: Decimal::ZERO,
            activation_score: Decimal::ZERO,
            contradiction_flags: Vec::new(),
            metadata: RetrievalMetadata {
                traversal_depth: 0,
                beam_width_used: beam_width,
                query_nodes: query_ids.to_vec(),
            },
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

        candidates.sort_by(|a, b| {
            b.1.cmp(&a.1).then_with(|| {
                let a_ids: Vec<AtomId> = a.0.iter()
                    .filter_map(|idx| graph.get_atom_by_index(*idx).map(|a| a.id))
                    .collect();
                let b_ids: Vec<AtomId> = b.0.iter()
                    .filter_map(|idx| graph.get_atom_by_index(*idx).map(|a| a.id))
                    .collect();
                let a_uuids: Vec<_> = a_ids.iter().map(|id| id.0).collect();
                let b_uuids: Vec<_> = b_ids.iter().map(|id| id.0).collect();
                b_uuids.cmp(&a_uuids)
            })
        });
        candidates.truncate(beam_width);

        if let Some((nodes, score)) = candidates.first() {
            if *score > best_score {
                best_score = *score;
                best_subgraph = nodes.clone();
            }
        }

        frontier = candidates;
    }

    let nodes_atom_ids: Vec<AtomId> = best_subgraph
        .iter()
        .filter_map(|idx| graph.get_atom_by_index(*idx).map(|a| a.id))
        .collect();

    let (salience_score, activation_score) =
        compute_field_scores(graph, &best_subgraph, activations);

    CognitiveField {
        nodes: nodes_atom_ids,
        relevance_score: best_score,
        salience_score,
        activation_score,
        contradiction_flags: vec![false; best_subgraph.len()],
        metadata: RetrievalMetadata {
            traversal_depth: d_max,
            beam_width_used: beam_width,
            query_nodes: query_ids.to_vec(),
        },
    }
}

pub fn retrieve_beam_with_contradiction(
    graph: &CognitiveGraph,
    query_ids: &[AtomId],
    activations: &HashMap<AtomId, Activation>,
    contradictions: &HashMap<(AtomId, AtomId), Pressure>,
    current_tick: u64,
    config: &EngineConfig,
) -> CognitiveField {
    let d_max = config.max_retrieval_depth;
    let n_max = config.max_subgraph_size;
    let beam_width = config.beam_width;

    let query_indices: Vec<NodeIndex> = query_ids
        .iter()
        .filter_map(|id| graph.node_index(*id))
        .collect();

    if query_indices.is_empty() {
        return CognitiveField {
            nodes: Vec::new(),
            relevance_score: Decimal::ZERO,
            salience_score: Decimal::ZERO,
            activation_score: Decimal::ZERO,
            contradiction_flags: Vec::new(),
            metadata: RetrievalMetadata {
                traversal_depth: 0,
                beam_width_used: beam_width,
                query_nodes: query_ids.to_vec(),
            },
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

                let score = relevance_with_contradiction(
                    graph, &new_subgraph, query_ids, activations, contradictions, current_tick,
                );
                candidates.push((new_subgraph, score));
            }
        }

        if candidates.is_empty() {
            break;
        }

        candidates.sort_by(|a, b| {
            b.1.cmp(&a.1).then_with(|| {
                let a_ids: Vec<AtomId> = a.0.iter()
                    .filter_map(|idx| graph.get_atom_by_index(*idx).map(|a| a.id))
                    .collect();
                let b_ids: Vec<AtomId> = b.0.iter()
                    .filter_map(|idx| graph.get_atom_by_index(*idx).map(|a| a.id))
                    .collect();
                let a_uuids: Vec<_> = a_ids.iter().map(|id| id.0).collect();
                let b_uuids: Vec<_> = b_ids.iter().map(|id| id.0).collect();
                b_uuids.cmp(&a_uuids)
            })
        });
        candidates.truncate(beam_width);

        if let Some((nodes, score)) = candidates.first() {
            if *score > best_score {
                best_score = *score;
                best_subgraph = nodes.clone();
            }
        }

        frontier = candidates;
    }

    let nodes_atom_ids: Vec<AtomId> = best_subgraph
        .iter()
        .filter_map(|idx| graph.get_atom_by_index(*idx).map(|a| a.id))
        .collect();

    let (salience_score, activation_score) =
        compute_field_scores(graph, &best_subgraph, activations);

    let contradiction_flags = compute_contradiction_flags(&best_subgraph, graph, contradictions);

    CognitiveField {
        nodes: nodes_atom_ids,
        relevance_score: best_score,
        salience_score,
        activation_score,
        contradiction_flags,
        metadata: RetrievalMetadata {
            traversal_depth: d_max,
            beam_width_used: beam_width,
            query_nodes: query_ids.to_vec(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dcma_core::atom::CognitiveAtom;
    use std::collections::VecDeque;

    fn make_test_graph() -> (CognitiveGraph, AtomId, AtomId, AtomId) {
        let mut g = CognitiveGraph::new();
        let a = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "postgres".to_string());
        let b = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "database".to_string());
        let c = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "storage".to_string());
        let id_a = a.id;
        let id_b = b.id;
        let id_c = c.id;
        g.add_atom(a);
        g.add_atom(b);
        g.add_atom(c);
        g.add_relation_with_weight(id_a, id_b, RelationType::InstanceOf,
            Weight::new(Decimal::from_str_exact("0.9").unwrap())).unwrap();
        g.add_relation_with_weight(id_b, id_c, RelationType::Contains,
            Weight::new(Decimal::from_str_exact("0.8").unwrap())).unwrap();
        (g, id_a, id_b, id_c)
    }

    #[test]
    fn test_beam_search_finds_query() {
        let (g, id_a, _, _) = make_test_graph();
        let config = EngineConfig::default();
        let mut activations = HashMap::new();
        activations.insert(id_a, Activation::new(Decimal::ONE));
        let field = retrieve_beam(&g, &[id_a], &activations, 0, &config);
        assert!(field.nodes.contains(&id_a), "Query node should be in result");
    }

    #[test]
    fn test_beam_search_respects_n_max() {
        let (g, id_a, id_b, _) = make_test_graph();
        let mut config = EngineConfig::default();
        config.max_subgraph_size = 2;
        let mut activations = HashMap::new();
        activations.insert(id_a, Activation::new(Decimal::ONE));
        activations.insert(id_b, Activation::new(Decimal::from_str_exact("0.5").unwrap()));
        let field = retrieve_beam(&g, &[id_a], &activations, 0, &config);
        assert!(field.nodes.len() <= 2, "Result should not exceed n_max, got {}", field.nodes.len());
    }

    #[test]
    fn test_beam_search_respects_d_max() {
        let mut g = CognitiveGraph::new();
        let atoms: Vec<_> = (0..6).map(|i| CognitiveAtom::new(AtomId::new(), AtomType::Concept, format!("n{}", i))).collect();
        let ids: Vec<_> = atoms.iter().map(|a| a.id).collect();
        for a in atoms { g.add_atom(a); }
        for i in 0..5 {
            g.add_relation(ids[i], ids[i + 1], RelationType::Causes).unwrap();
        }
        let mut config = EngineConfig::default();
        config.max_retrieval_depth = 2;
        config.max_subgraph_size = 10;
        let mut activations = HashMap::new();
        activations.insert(ids[0], Activation::new(Decimal::ONE));
        let field = retrieve_beam(&g, &[ids[0]], &activations, 0, &config);
        let q_idx = g.node_index(ids[0]).unwrap();
        for node_id in &field.nodes {
            if let Some(idx) = g.node_index(*node_id) {
                let dist = bfs_distance_nodes(&g, q_idx, idx);
                assert!(dist <= 2, "Node {:?} is {} hops from query, max is 2", node_id, dist);
            }
        }
    }

    fn bfs_distance_nodes(graph: &CognitiveGraph, from: NodeIndex, to: NodeIndex) -> usize {
        if from == to {
            return 0;
        }
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back((from, 0usize));
        visited.insert(from);
        while let Some((current, dist)) = queue.pop_front() {
            for (nidx, _) in graph.neighbors(
                graph.get_atom_by_index(current).map(|a| a.id).unwrap()
            ) {
                if !visited.contains(&nidx) {
                    if nidx == to {
                        return dist + 1;
                    }
                    visited.insert(nidx);
                    queue.push_back((nidx, dist + 1));
                }
            }
        }
        usize::MAX
    }

    #[test]
    fn test_beam_search_deterministic() {
        let (g, id_a, _, _) = make_test_graph();
        let config = EngineConfig::default();
        let mut activations = HashMap::new();
        activations.insert(id_a, Activation::new(Decimal::ONE));
        let field1 = retrieve_beam(&g, &[id_a], &activations, 0, &config);
        let field2 = retrieve_beam(&g, &[id_a], &activations, 0, &config);
        assert_eq!(field1.nodes, field2.nodes, "Same query should produce same result");
        assert_eq!(field1.relevance_score, field2.relevance_score);
    }

    #[test]
    fn test_retrieve_empty_query() {
        let (g, _, _, _) = make_test_graph();
        let config = EngineConfig::default();
        let field = retrieve_beam(&g, &[], &HashMap::new(), 0, &config);
        assert!(field.nodes.is_empty(), "Empty query should return empty result");
    }

    #[test]
    fn test_contradiction_reduces_relevance_beam() {
        let mut g = CognitiveGraph::new();
        let a = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "a".to_string());
        let b = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "b".to_string());
        let id_a = a.id;
        let id_b = b.id;
        g.add_atom(a);
        g.add_atom(b);
        g.add_relation(id_a, id_b, RelationType::Contradicts).unwrap();
        let mut config = EngineConfig::default();
        config.max_subgraph_size = 10;
        let mut activations = HashMap::new();
        activations.insert(id_a, Activation::new(Decimal::from_str_exact("0.9").unwrap()));
        activations.insert(id_b, Activation::new(Decimal::from_str_exact("0.9").unwrap()));
        let field_no_contradiction = retrieve_beam(&g, &[id_a], &activations, 0, &config);
        let mut contradictions = HashMap::new();
        contradictions.insert((id_a, id_b), Pressure::new(Decimal::from_str_exact("0.8").unwrap()));
        let field_with_contradiction = retrieve_beam_with_contradiction(
            &g, &[id_a], &activations, &contradictions, 0, &config,
        );
        assert!(
            field_with_contradiction.relevance_score <= field_no_contradiction.relevance_score,
            "Contradiction should not increase relevance"
        );
    }

    #[test]
    fn test_retrieve_single_node() {
        let (g, id_a, _, _) = make_test_graph();
        let config = EngineConfig::default();
        let mut activations = HashMap::new();
        activations.insert(id_a, Activation::new(Decimal::ONE));
        let field = retrieve_beam(&g, &[id_a], &activations, 0, &config);
        assert!(field.nodes.contains(&id_a), "Single query node should be in result");
    }
}
