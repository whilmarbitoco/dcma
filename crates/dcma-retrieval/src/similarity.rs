use dcma_core::graph::CognitiveGraph;
use dcma_core::types::AtomId;
use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet, VecDeque};

fn compute_path_weight(graph: &CognitiveGraph, from: AtomId, to: AtomId) -> Option<Decimal> {
    if from == to {
        return Some(Decimal::ONE);
    }
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let mut parent: HashMap<AtomId, (AtomId, Decimal)> = HashMap::new();
    visited.insert(from);
    queue.push_back(from);
    while let Some(current) = queue.pop_front() {
        if current == to {
            let mut weight = Decimal::ONE;
            let mut node = current;
            while let Some((prev, edge_w)) = parent.get(&node) {
                weight *= edge_w;
                node = *prev;
            }
            return Some(weight);
        }
        for (_, neighbor_atom) in graph.neighbors(current) {
            if !visited.contains(&neighbor_atom.id) {
                visited.insert(neighbor_atom.id);
                let mut max_w = Decimal::ZERO;
                for (_, rel) in graph.relations_between(current, neighbor_atom.id) {
                    if rel.weight.get() > max_w {
                        max_w = rel.weight.get();
                    }
                }
                for (_, rel) in graph.relations_between(neighbor_atom.id, current) {
                    if rel.weight.get() > max_w {
                        max_w = rel.weight.get();
                    }
                }
                parent.insert(neighbor_atom.id, (current, max_w));
                queue.push_back(neighbor_atom.id);
            }
        }
    }
    None
}

pub fn structural_similarity(
    graph: &CognitiveGraph,
    node_id: AtomId,
    query_ids: &[AtomId],
    k: usize,
) -> Decimal {
    if query_ids.is_empty() {
        return Decimal::ZERO;
    }
    if query_ids.contains(&node_id) {
        return Decimal::ONE;
    }
    let node_neighbors = graph.k_hop_neighbors(node_id, k);
    let node_set: HashSet<_> = node_neighbors.keys().collect();
    let mut query_union: HashMap<petgraph::stable_graph::NodeIndex, usize> = HashMap::new();
    for qid in query_ids {
        for (nidx, dist) in graph.k_hop_neighbors(*qid, k) {
            query_union.entry(nidx).or_insert(dist);
        }
    }
    if node_neighbors.is_empty() || query_union.is_empty() {
        return Decimal::ZERO;
    }
    let mut intersection_size = 0usize;
    for nidx in &node_set {
        if query_union.contains_key(nidx) {
            intersection_size += 1;
        }
    }
    let union_size = node_neighbors.len() + query_union.len() - intersection_size;
    if union_size == 0 {
        return Decimal::ZERO;
    }
    let jaccard = Decimal::from(intersection_size) / Decimal::from(union_size);
    let diameter = graph.diameter().unwrap_or(1);
    if diameter == 0 {
        return Decimal::ZERO;
    }
    let max_path_weight = query_ids
        .iter()
        .filter_map(|qid| compute_path_weight(graph, node_id, *qid))
        .fold(Decimal::ZERO, |a, b| a.max(b));
    jaccard * max_path_weight / Decimal::from(diameter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dcma_core::atom::CognitiveAtom;
    use dcma_core::graph::CognitiveGraph;
    use dcma_core::types::*;

    fn make_chain_graph() -> (CognitiveGraph, AtomId, AtomId, AtomId, AtomId, AtomId) {
        let mut g = CognitiveGraph::new();
        let a = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "a".to_string());
        let b = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "b".to_string());
        let c = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "c".to_string());
        let d = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "d".to_string());
        let e = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "e".to_string());
        let id_a = a.id;
        let id_b = b.id;
        let id_c = c.id;
        let id_d = d.id;
        let id_e = e.id;
        g.add_atom(a);
        g.add_atom(b);
        g.add_atom(c);
        g.add_atom(d);
        g.add_atom(e);
        g.add_relation_with_weight(id_a, id_b, RelationType::Causes,
            Weight::new(Decimal::from_str_exact("0.9").unwrap())).unwrap();
        g.add_relation_with_weight(id_b, id_c, RelationType::Causes,
            Weight::new(Decimal::from_str_exact("0.8").unwrap())).unwrap();
        g.add_relation_with_weight(id_c, id_d, RelationType::Causes,
            Weight::new(Decimal::from_str_exact("0.7").unwrap())).unwrap();
        g.add_relation_with_weight(id_d, id_e, RelationType::Causes,
            Weight::new(Decimal::from_str_exact("0.6").unwrap())).unwrap();
        (g, id_a, id_b, id_c, id_d, id_e)
    }

    #[test]
    fn test_structural_similarity_identical() {
        let (g, id_a, _, _, _, _) = make_chain_graph();
        let sim = structural_similarity(&g, id_a, &[id_a], 2);
        assert_eq!(sim, Decimal::ONE, "Same node queried against itself should be 1.0");
    }

    #[test]
    fn test_structural_similarity_disconnected() {
        let mut g = CognitiveGraph::new();
        let a = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "a".to_string());
        let b = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "b".to_string());
        let id_a = a.id;
        let id_b = b.id;
        g.add_atom(a);
        g.add_atom(b);
        let sim = structural_similarity(&g, id_a, &[id_b], 2);
        assert_eq!(sim, Decimal::ZERO, "Disconnected nodes should have similarity 0.0");
    }

    #[test]
    fn test_structural_similarity_direct_connection() {
        let (g, id_a, id_b, _, _, _) = make_chain_graph();
        let sim = structural_similarity(&g, id_a, &[id_b], 2);
        assert!(sim > Decimal::ZERO, "Direct connection should have non-zero similarity");
    }

    #[test]
    fn test_structural_similarity_empty_query() {
        let (g, id_a, _, _, _, _) = make_chain_graph();
        let sim = structural_similarity(&g, id_a, &[], 2);
        assert_eq!(sim, Decimal::ZERO);
    }
}
