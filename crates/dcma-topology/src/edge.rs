use dcma_core::atom::CognitiveAtom;
use dcma_core::graph::CognitiveGraph;
use dcma_core::types::*;
use petgraph::stable_graph::EdgeIndex;
use petgraph::Direction;
use rust_decimal::Decimal;
use rust_decimal::MathematicalOps;
use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

#[derive(Debug, thiserror::Error)]
pub enum TopologyError {
    #[error("Graph has no nodes")]
    EmptyGraph,
    #[error("Invalid partition count: {0}")]
    InvalidPartitionCount(usize),
    #[error("Weight history not available for edge {0:?}")]
    NoWeightHistory(EdgeIndex),
}

const STABILIZATION_WINDOW: usize = 50;
const STABILITY_EPSILON: Decimal = Decimal::from_parts(1, 0, 0, false, 8);
const STABILITY_THRESHOLD: Decimal = Decimal::from_parts(75, 0, 0, false, 2);
const HUB_MIN_DEGREE: usize = 5;
const HUB_SUBNODE_DIVISOR: usize = 3;

#[derive(Debug, Clone, Serialize)]
pub struct WeightChangeReport {
    pub edges_updated: usize,
    pub total_reinforcement: Decimal,
    pub total_decay: Decimal,
    pub total_contradiction_suppression: Decimal,
}

#[derive(Debug, Clone, Serialize)]
pub struct StabilizationReport {
    pub newly_stabilized: Vec<EdgeIndex>,
    pub destabilized: Vec<EdgeIndex>,
    pub stable_count: usize,
}

#[derive(Debug, Clone)]
pub struct HubSplitResult {
    pub original: AtomId,
    pub new_nodes: Vec<AtomId>,
    pub edges_redistributed: usize,
}

fn weight_history() -> &'static Mutex<HashMap<EdgeIndex, VecDeque<Decimal>>> {
    static HISTORY: std::sync::OnceLock<Mutex<HashMap<EdgeIndex, VecDeque<Decimal>>>> =
        std::sync::OnceLock::new();
    HISTORY.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn clear_weight_history() {
    if let Ok(mut history) = weight_history().lock() {
        history.clear();
    }
}

fn record_weight(edge: EdgeIndex, weight: Decimal) {
    if let Ok(mut history) = weight_history().lock() {
        let entry = history
            .entry(edge)
            .or_insert_with(|| VecDeque::with_capacity(STABILIZATION_WINDOW + 1));
        entry.push_back(weight);
        if entry.len() > STABILIZATION_WINDOW {
            entry.pop_front();
        }
    }
}

fn get_weight_history(edge: EdgeIndex) -> Vec<Decimal> {
    if let Ok(history) = weight_history().lock() {
        history
            .get(&edge)
            .map(|h| h.iter().copied().collect())
            .unwrap_or_default()
    } else {
        Vec::new()
    }
}

pub fn evolve_weights(
    graph: &mut CognitiveGraph,
    config: &EngineConfig,
    tick: u64,
) -> WeightChangeReport {
    let mut report = WeightChangeReport {
        edges_updated: 0,
        total_reinforcement: Decimal::ZERO,
        total_decay: Decimal::ZERO,
        total_contradiction_suppression: Decimal::ZERO,
    };

    let edges: Vec<EdgeIndex> = graph.relations().map(|(idx, _)| idx).collect();

    for edge_idx in edges {
        let (reinforcement_signal, contradiction_signal) = {
            let rel = match graph.get_relation(edge_idx) {
                Some(r) => r,
                None => continue,
            };

            let r_signal = if let Some(atom) = graph.get_atom(rel.src) {
                let recent_count = atom
                    .reinforcement_history
                    .iter()
                    .filter(|&&t| tick.saturating_sub(t) <= config.reinforcement_window as u64)
                    .count();
                if recent_count > 0 {
                    Decimal::ONE
                } else {
                    Decimal::ZERO
                }
            } else {
                Decimal::ZERO
            };

            let c_signal = if rel.relation_type == RelationType::Contradicts {
                rel.weight.get()
            } else {
                Decimal::ZERO
            };

            (r_signal, c_signal)
        };

        let (delta_r, delta_d, delta_c) = {
            let rel = graph.get_relation_mut(edge_idx).unwrap();
            let old_weight = rel.weight.get();
            rel.evolve_weight(reinforcement_signal, contradiction_signal, config);

            if reinforcement_signal > Decimal::ZERO {
                rel.evidence.record_support();
            }
            if contradiction_signal > Decimal::ZERO {
                rel.evidence.record_oppose();
            }

            let new_weight = rel.weight.get();

            record_weight(edge_idx, new_weight);

            let dr = config.eta_r.get() * reinforcement_signal;
            let dd = config.eta_d.get() * old_weight;
            let dc = config.eta_c.get() * contradiction_signal;

            (dr, dd, dc)
        };

        report.edges_updated += 1;
        report.total_reinforcement += delta_r;
        report.total_decay += delta_d;
        report.total_contradiction_suppression += delta_c;
    }

    report
}

pub fn stabilize_relations(
    graph: &mut CognitiveGraph,
    _config: &EngineConfig,
) -> StabilizationReport {
    let mut report = StabilizationReport {
        newly_stabilized: Vec::new(),
        destabilized: Vec::new(),
        stable_count: 0,
    };

    let edges: Vec<EdgeIndex> = graph.relations().map(|(idx, _)| idx).collect();

    let mut stability_results: Vec<(EdgeIndex, Decimal)> = Vec::new();

    for edge_idx in &edges {
        let history = get_weight_history(*edge_idx);
        if history.is_empty() {
            continue;
        }

        let n_val = Decimal::from(history.len() as u64);
        let sum: Decimal = history.iter().copied().sum();
        let mean = sum / n_val;

        let variance = if history.len() > 1 {
            let sum_sq_diff: Decimal = history
                .iter()
                .map(|w| {
                    let diff = *w - mean;
                    diff * diff
                })
                .sum();
            sum_sq_diff / n_val
        } else {
            Decimal::ZERO
        };

        let std_dev = variance.sqrt().unwrap_or(Decimal::ZERO);
        let cv = std_dev / (mean + STABILITY_EPSILON);
        let stability = (Decimal::ONE - cv).clamp(Decimal::ZERO, Decimal::ONE);

        stability_results.push((*edge_idx, stability));
    }

    for (edge_idx, stability) in &stability_results {
        let was_stable = graph
            .get_relation(*edge_idx)
            .map_or(false, |rel| rel.stability.get() >= STABILITY_THRESHOLD);

        if let Some(rel) = graph.get_relation_mut(*edge_idx) {
            rel.stability = Confidence::new(*stability);
        }

        let is_stable = *stability >= STABILITY_THRESHOLD;

        if is_stable && !was_stable {
            report.newly_stabilized.push(*edge_idx);
        } else if !is_stable && was_stable {
            report.destabilized.push(*edge_idx);
        }
        if is_stable {
            report.stable_count += 1;
        }
    }

    report
}

pub fn split_hubs(
    graph: &mut CognitiveGraph,
    _config: &EngineConfig,
) -> Vec<HubSplitResult> {
    let mut results = Vec::new();

    let total_atoms = graph.atom_count();
    let total_edges = graph.relation_count();
    if total_atoms == 0 {
        return results;
    }
    let avg_degree =
        Decimal::from(total_edges as u64) / Decimal::from(total_atoms as u64);
    let threshold = avg_degree * Decimal::from(HUB_MIN_DEGREE as u64);

    let candidate_ids: Vec<AtomId> = graph
        .atoms()
        .filter_map(|(_, atom)| {
            let deg = graph.degree(atom.id);
            if Decimal::from(deg as u64) > threshold && deg > HUB_MIN_DEGREE {
                Some(atom.id)
            } else {
                None
            }
        })
        .collect();

    for hub_id in candidate_ids {
        let degree = graph.degree(hub_id);
        if degree == 0 {
            continue;
        }

        let original_atom = match graph.get_atom(hub_id) {
            Some(a) => a.clone(),
            None => continue,
        };

        let num_subnodes = (degree + HUB_SUBNODE_DIVISOR - 1) / HUB_SUBNODE_DIVISOR;
        let mut subnode_ids = Vec::with_capacity(num_subnodes);

        let edges: Vec<(AtomId, RelationType, Weight, bool)> = graph
            .edges_of(hub_id)
            .iter()
            .map(|(_, dir, rel)| {
                let neighbor_id = match dir {
                    Direction::Incoming => rel.src,
                    Direction::Outgoing => rel.dst,
                };
                let is_outgoing = *dir == Direction::Outgoing;
                (neighbor_id, rel.relation_type, rel.weight, is_outgoing)
            })
            .collect();

        for (neighbor_id, rel_type, _, is_outgoing) in &edges {
            let (src, dst) = if *is_outgoing {
                (hub_id, *neighbor_id)
            } else {
                (*neighbor_id, hub_id)
            };
            graph.remove_relation(src, dst, *rel_type);
        }

        if let Some(atom) = graph.get_atom_mut(hub_id) {
            atom.state = AtomState::Dormant;
        }

        let mut edges_redistributed = 0;
        for i in 0..num_subnodes {
            let sub_name = format!("{}_sub_{}", original_atom.canonical, i);
            let sub_atom = CognitiveAtom::new(
                AtomId::new(),
                original_atom.atom_type,
                sub_name,
            );
            let sub_id = sub_atom.id;
            graph.add_atom(sub_atom);
            subnode_ids.push(sub_id);
        }

        for (idx, (neighbor_id, rel_type, weight, is_outgoing)) in
            edges.iter().enumerate()
        {
            let sub_idx = idx % num_subnodes;
            let sub_id = subnode_ids[sub_idx];
            if let Ok(edge_idx) = graph.add_relation_with_weight(
                if *is_outgoing { sub_id } else { *neighbor_id },
                if *is_outgoing { *neighbor_id } else { sub_id },
                *rel_type,
                *weight,
            ) {
                if let Some(rel) = graph.get_relation(edge_idx) {
                    record_weight(edge_idx, rel.weight.get());
                }
                edges_redistributed += 1;
            }
        }

        results.push(HubSplitResult {
            original: hub_id,
            new_nodes: subnode_ids,
            edges_redistributed,
        });
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use dcma_core::atom::CognitiveAtom;
    use serial_test::serial;

    fn make_graph_with_edge(
        rel_type: RelationType,
        weight: Decimal,
    ) -> (CognitiveGraph, EdgeIndex) {
        let mut graph = CognitiveGraph::new();
        let a = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "a".to_string());
        let b = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "b".to_string());
        let id_a = a.id;
        let id_b = b.id;
        graph.add_atom(a);
        graph.add_atom(b);
        let edge = graph
            .add_relation_with_weight(id_a, id_b, rel_type, Weight::new(weight))
            .unwrap();
        (graph, edge)
    }

    #[allow(dead_code)]
    fn make_graph() -> CognitiveGraph {
        let mut graph = CognitiveGraph::new();
        for i in 0..3 {
            let atom = CognitiveAtom::new(
                AtomId::new(),
                AtomType::Concept,
                format!("n{}", i),
            );
            graph.add_atom(atom);
        }
        let ids: Vec<AtomId> = graph.atoms().map(|(_, a)| a.id).collect();
        graph
            .add_relation(ids[0], ids[1], RelationType::Causes)
            .unwrap();
        graph
            .add_relation(ids[1], ids[2], RelationType::Supports)
            .unwrap();
        graph
    }

    #[test]
    #[serial]
    fn test_weight_reinforce() {
        let (mut graph, edge) =
            make_graph_with_edge(RelationType::Causes, Decimal::from_str_exact("0.5").unwrap());
        let config = EngineConfig::default();

        clear_weight_history();
        let a_id = graph.get_relation(edge).unwrap().src;
        if let Some(atom) = graph.get_atom_mut(a_id) {
            atom.record_reinforcement(1);
        }

        let report = evolve_weights(&mut graph, &config, 1);
        assert_eq!(report.edges_updated, 1);
        assert!(report.total_reinforcement > Decimal::ZERO);
    }

    #[test]
    #[serial]
    fn test_weight_decay() {
        let (mut graph, edge) =
            make_graph_with_edge(RelationType::Causes, Decimal::from_str_exact("0.5").unwrap());
        let config = EngineConfig::default();

        clear_weight_history();
        let old_w = graph.edge_weight(edge).unwrap();
        evolve_weights(&mut graph, &config, 0);
        let new_w = graph.edge_weight(edge).unwrap();
        assert!(new_w < old_w, "weight should decay: {:?} < {:?}", new_w, old_w);
    }

    #[test]
    #[serial]
    fn test_weight_clamped() {
        let (mut graph, edge) =
            make_graph_with_edge(RelationType::Causes, Decimal::from_str_exact("0.5").unwrap());
        let config = EngineConfig::default();

        clear_weight_history();
        let a_id = graph.get_relation(edge).unwrap().src;
        if let Some(atom) = graph.get_atom_mut(a_id) {
            for t in 1..=50 {
                atom.record_reinforcement(t);
            }
        }

        for t in 1..=50 {
            evolve_weights(&mut graph, &config, t);
            let w = graph.edge_weight(edge).unwrap();
            assert!(w >= Decimal::ZERO, "weight should be >= 0: {:?}", w);
            assert!(w <= Decimal::ONE, "weight should be <= 1: {:?}", w);
        }
    }

    #[test]
    fn test_weight_contradiction() {
        let (mut graph, edge) = make_graph_with_edge(
            RelationType::Contradicts,
            Decimal::from_str_exact("0.5").unwrap(),
        );
        let config = EngineConfig::default();

        clear_weight_history();
        let old_w = graph.edge_weight(edge).unwrap();
        evolve_weights(&mut graph, &config, 0);
        let new_w = graph.edge_weight(edge).unwrap();

        assert!(new_w < old_w, "contradiction should reduce weight");
    }

    #[test]
    #[serial]
    fn test_stability_constant() {
        let (mut graph, edge) =
            make_graph_with_edge(RelationType::Causes, Decimal::from_str_exact("0.5").unwrap());
        let config = EngineConfig::default();
        let constant_weight = Decimal::from_str_exact("0.5").unwrap();

        clear_weight_history();

        // Set weight to constant and record identical history — no reinforcement
        // on the source atom so evolve_weights doesn't change the weight
        graph.update_edge_weight(edge, Weight::new(constant_weight)).unwrap();
        for _ in 0..60 {
            record_weight(edge, constant_weight);
        }

        let _report = stabilize_relations(&mut graph, &config);
        let rel = graph.get_relation(edge).unwrap();
        assert!(
            rel.stability.get() > Decimal::from_str_exact("0.85").unwrap(),
            "constant weight should give stability near 1.0, got {:?}",
            rel.stability.get()
        );
    }

    #[test]
    #[serial]
    fn test_stability_varying() {
        let (mut graph, _edge) =
            make_graph_with_edge(RelationType::Causes, Decimal::from_str_exact("0.5").unwrap());
        let config = EngineConfig::default();

        clear_weight_history();
        let ids: Vec<AtomId> = graph.atoms().map(|(_, a)| a.id).collect();

        let edge_idx = graph
            .relations()
            .find(|(_, r)| r.src == ids[0] && r.dst == ids[1])
            .map(|(e, _)| e)
            .unwrap();

        for t in 1..=60 {
            let w = if t % 2 == 0 {
                Decimal::from_str_exact("0.1").unwrap()
            } else {
                Decimal::from_str_exact("0.9").unwrap()
            };
            graph
                .update_edge_weight(edge_idx, Weight::new(w))
                .unwrap();
            record_weight(edge_idx, w);
        }

        let _report = stabilize_relations(&mut graph, &config);
        let rel = graph.get_relation(edge_idx).unwrap();
        assert!(
            rel.stability.get() < Decimal::from_str_exact("0.9").unwrap(),
            "varying weight should give lower stability, got {:?}",
            rel.stability.get()
        );
    }

    #[test]
    #[serial]
    fn test_stability_threshold() {
        let (mut graph1, edge1) =
            make_graph_with_edge(RelationType::Causes, Decimal::from_str_exact("0.5").unwrap());
        let config = EngineConfig::default();
        let constant_weight = Decimal::from_str_exact("0.5").unwrap();

        clear_weight_history();

        // Set weight to constant and record identical history — no reinforcement
        graph1.update_edge_weight(edge1, Weight::new(constant_weight)).unwrap();
        for _ in 0..60 {
            record_weight(edge1, constant_weight);
        }

        stabilize_relations(&mut graph1, &config);
        assert!(
            graph1.get_relation(edge1).unwrap().stability.get() >= STABILITY_THRESHOLD,
            "stable edge should be above threshold"
        );
    }

    #[test]
    fn test_hub_split() {
        let mut graph = CognitiveGraph::new();
        let center = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "hub".to_string());
        let center_id = center.id;
        graph.add_atom(center);

        let mut leaf_ids = Vec::new();
        for i in 0..10 {
            let leaf = CognitiveAtom::new(
                AtomId::new(),
                AtomType::Concept,
                format!("leaf{}", i),
            );
            let leaf_id = leaf.id;
            graph.add_atom(leaf);
            graph
                .add_relation(center_id, leaf_id, RelationType::Causes)
                .unwrap();
            leaf_ids.push(leaf_id);
        }

        assert_eq!(graph.degree(center_id), 10);

        let config = EngineConfig::default();
        let results = split_hubs(&mut graph, &config);

        assert!(!results.is_empty(), "hub should be split");
        assert_eq!(results[0].original, center_id);
        assert_eq!(results[0].new_nodes.len(), 4);
        assert_eq!(results[0].edges_redistributed, 10);

        assert_eq!(
            graph.get_atom(center_id).unwrap().state,
            AtomState::Dormant
        );
    }

    #[test]
    fn test_hub_no_split_low_degree() {
        let mut graph = CognitiveGraph::new();
        let center = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "small".to_string());
        let center_id = center.id;
        graph.add_atom(center);

        for i in 0..2 {
            let leaf = CognitiveAtom::new(
                AtomId::new(),
                AtomType::Concept,
                format!("leaf{}", i),
            );
            let leaf_id = leaf.id;
            graph.add_atom(leaf);
            graph
                .add_relation(center_id, leaf_id, RelationType::Causes)
                .unwrap();
        }

        let config = EngineConfig::default();
        let results = split_hubs(&mut graph, &config);
        assert!(results.is_empty(), "low-degree node should not be split");
    }

    #[test]
    #[serial]
    fn test_evolve_deterministic() {
        clear_weight_history();
        let config = EngineConfig::default();

        let (mut graph1, edge1) =
            make_graph_with_edge(RelationType::Causes, Decimal::from_str_exact("0.5").unwrap());
        let a_id1 = graph1.get_relation(edge1).unwrap().src;
        if let Some(atom) = graph1.get_atom_mut(a_id1) {
            atom.record_reinforcement(1);
        }

        let (mut graph2, edge2) =
            make_graph_with_edge(RelationType::Causes, Decimal::from_str_exact("0.5").unwrap());
        let a_id2 = graph2.get_relation(edge2).unwrap().src;
        if let Some(atom) = graph2.get_atom_mut(a_id2) {
            atom.record_reinforcement(1);
        }

        clear_weight_history();
        evolve_weights(&mut graph1, &config, 1);
        clear_weight_history();
        evolve_weights(&mut graph2, &config, 1);

        let w1 = graph1.edge_weight(edge1).unwrap();
        let w2 = graph2.edge_weight(edge2).unwrap();
        assert_eq!(w1, w2, "deterministic evolution should give same result");
    }
}
