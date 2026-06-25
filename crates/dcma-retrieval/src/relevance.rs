use dcma_core::graph::CognitiveGraph;
use dcma_core::types::*;
use crate::similarity::structural_similarity;
use petgraph::stable_graph::NodeIndex;
use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet, VecDeque};

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

fn bfs_distance(graph: &CognitiveGraph, from: AtomId, to: AtomId, allowed: &[NodeIndex]) -> usize {
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

const ALPHA: &str = "0.35";
const BETA: &str = "0.20";
const GAMMA: &str = "0.20";
const DELTA: &str = "0.15";
const LAMBDA: &str = "0.10";
const MU: &str = "0.01";

pub fn relevance(
    graph: &CognitiveGraph,
    subgraph_nodes: &[NodeIndex],
    query_ids: &[AtomId],
    activations: &HashMap<AtomId, Activation>,
    current_tick: u64,
    evidence_config: &EvidenceConfig,
) -> Decimal {
    if subgraph_nodes.is_empty() {
        return Decimal::ZERO;
    }
    let n = Decimal::from(subgraph_nodes.len());
    let alpha = Decimal::from_str_exact(ALPHA).unwrap();
    let beta = Decimal::from_str_exact(BETA).unwrap();
    let gamma = Decimal::from_str_exact(GAMMA).unwrap();
    let delta = Decimal::from_str_exact(DELTA).unwrap();
    let lambda_d = Decimal::from_str_exact(LAMBDA).unwrap();
    let mu = Decimal::from_str_exact(MU).unwrap();
    let nu = evidence_config.confidence_coefficient;

    let mut coherence = Decimal::ZERO;
    let mut salience = Decimal::ZERO;
    let mut activation_res = Decimal::ZERO;
    let mut temporal = Decimal::ZERO;
    let mut evidence_conf = Decimal::ZERO;

    for idx in subgraph_nodes {
        if let Some(atom) = graph.get_atom_by_index(*idx) {
            coherence += structural_similarity(graph, atom.id, query_ids, 2);
            salience += atom.salience.get();
            if let Some(act) = activations.get(&atom.id) {
                activation_res += act.get();
            }
            let ticks_since = current_tick.saturating_sub(atom.t_last_accessed);
            let recency = exp_approx(-mu * Decimal::from(ticks_since));
            temporal += recency;
            evidence_conf += atom.evidence_decayed_confidence(evidence_config.half_life_ticks, current_tick);
        }
    }
    coherence /= n;
    salience /= n;
    activation_res /= n;
    temporal /= n;
    evidence_conf /= n;

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

    alpha * coherence + beta * salience + gamma * activation_res + delta * temporal
        + nu * evidence_conf
        - lambda_d * traversal_cost
}

pub fn relevance_with_contradiction(
    graph: &CognitiveGraph,
    subgraph_nodes: &[NodeIndex],
    query_ids: &[AtomId],
    activations: &HashMap<AtomId, Activation>,
    contradictions: &HashMap<(AtomId, AtomId), Pressure>,
    current_tick: u64,
    evidence_config: &EvidenceConfig,
) -> Decimal {
    let base = relevance(graph, subgraph_nodes, query_ids, activations, current_tick, evidence_config);
    if subgraph_nodes.is_empty() {
        return Decimal::ZERO;
    }
    let mut total_pressure = Decimal::ZERO;
    let mut count = Decimal::ZERO;
    for idx in subgraph_nodes {
        if let Some(atom) = graph.get_atom_by_index(*idx) {
            let mut node_pressure = Decimal::ZERO;
            let mut rel_count = 0usize;
            for ((a, b), p) in contradictions {
                if *a == atom.id || *b == atom.id {
                    node_pressure += p.get();
                    rel_count += 1;
                }
            }
            if rel_count > 0 {
                total_pressure += node_pressure / Decimal::from(rel_count);
            }
            count += Decimal::ONE;
        }
    }
    let avg_pressure = if count > Decimal::ZERO {
        total_pressure / count
    } else {
        Decimal::ZERO
    };
    base * (Decimal::ONE - avg_pressure)
}

pub fn compute_contradiction_flags(
    subgraph_nodes: &[NodeIndex],
    graph: &CognitiveGraph,
    contradictions: &HashMap<(AtomId, AtomId), Pressure>,
) -> Vec<bool> {
    subgraph_nodes
        .iter()
        .map(|idx| {
            if let Some(atom) = graph.get_atom_by_index(*idx) {
                contradictions
                    .keys()
                    .any(|(a, b)| *a == atom.id || *b == atom.id)
            } else {
                false
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dcma_core::atom::CognitiveAtom;
    use dcma_core::graph::CognitiveGraph;

    fn make_test_graph() -> (CognitiveGraph, AtomId, AtomId, AtomId, AtomId, AtomId) {
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
    fn test_relevance_functional() {
        let (g, id_a, id_b, id_c, _, _) = make_test_graph();
        let idx_a = g.node_index(id_a).unwrap();
        let idx_b = g.node_index(id_b).unwrap();
        let idx_c = g.node_index(id_c).unwrap();
        let mut activations = HashMap::new();
        activations.insert(id_a, Activation::new(Decimal::from_str_exact("0.9").unwrap()));
        activations.insert(id_b, Activation::new(Decimal::from_str_exact("0.7").unwrap()));
        activations.insert(id_c, Activation::new(Decimal::from_str_exact("0.5").unwrap()));
        let ev_cfg = EvidenceConfig::default();
        let score = relevance(&g, &[idx_a, idx_b, idx_c], &[id_a], &activations, 10, &ev_cfg);
        assert!(score > Decimal::ZERO, "Relevance should be positive for connected graph");
        assert!(score <= Decimal::ONE, "Relevance should not exceed 1.0");
    }

    #[test]
    fn test_relevance_empty_subgraph() {
        let (g, _, _, _, _, _) = make_test_graph();
        let ev_cfg = EvidenceConfig::default();
        let score = relevance(&g, &[], &[AtomId::new()], &HashMap::new(), 0, &ev_cfg);
        assert_eq!(score, Decimal::ZERO);
    }

    #[test]
    fn test_contradiction_reduces_relevance() {
        let (g, id_a, id_b, id_c, _, _) = make_test_graph();
        let idx_a = g.node_index(id_a).unwrap();
        let idx_b = g.node_index(id_b).unwrap();
        let idx_c = g.node_index(id_c).unwrap();
        let mut activations = HashMap::new();
        activations.insert(id_a, Activation::new(Decimal::from_str_exact("0.9").unwrap()));
        activations.insert(id_b, Activation::new(Decimal::from_str_exact("0.7").unwrap()));
        activations.insert(id_c, Activation::new(Decimal::from_str_exact("0.5").unwrap()));
        let ev_cfg = EvidenceConfig::default();
        let base = relevance(&g, &[idx_a, idx_b, idx_c], &[id_a], &activations, 10, &ev_cfg);
        let mut contradictions = HashMap::new();
        contradictions.insert((id_a, id_b), Pressure::new(Decimal::from_str_exact("0.5").unwrap()));
        let contrad = relevance_with_contradiction(
            &g, &[idx_a, idx_b, idx_c], &[id_a], &activations, &contradictions, 10, &ev_cfg,
        );
        assert!(contrad < base, "Contradiction should reduce relevance");
    }
}
