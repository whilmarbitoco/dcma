use dcma_core::graph::CognitiveGraph;
use dcma_core::types::*;
use petgraph::stable_graph::NodeIndex;
use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct Motif {
    pub pattern: Vec<NodeIndex>,
    pub size: usize,
    pub recurrence: usize,
    pub stability: Activation,
    pub canonical_form: String,
}

#[derive(Debug, Clone)]
pub struct MotifReport {
    pub motifs: Vec<Motif>,
    pub total_patterns_found: usize,
    pub total_recurring: usize,
}

pub fn detect_motifs(graph: &CognitiveGraph, config: &EngineConfig) -> MotifReport {
    let min_size = 3;
    let max_size = config.max_motif_size;
    let min_recurrence = config.crystallization_threshold;

    let mut canon_groups: HashMap<String, HashSet<Vec<NodeIndex>>> = HashMap::new();

    for node in graph.node_indices() {
        let mut current = Vec::new();
        let mut current_set = HashSet::new();
        current.push(node);
        current_set.insert(node);
        enumerate_recursive(
            graph,
            &mut current,
            &mut current_set,
            min_size,
            max_size,
            &mut canon_groups,
        );
    }

    let total_patterns_found = canon_groups.len();

    let mut motifs: Vec<Motif> = Vec::new();
    for (canon, instance_set) in &canon_groups {
        let recurrence = instance_set.len();
        if recurrence >= min_recurrence {
            let representative = instance_set.iter().next().unwrap().clone();
            let size = representative.len();
            let stability = compute_stability(graph, &representative);

            motifs.push(Motif {
                pattern: representative,
                size,
                recurrence,
                stability,
                canonical_form: canon.clone(),
            });
        }
    }

    motifs.sort_by(|a, b| b.recurrence.cmp(&a.recurrence));

    let total_recurring = motifs.len();

    MotifReport {
        motifs,
        total_patterns_found,
        total_recurring,
    }
}

fn enumerate_recursive(
    graph: &CognitiveGraph,
    current: &mut Vec<NodeIndex>,
    current_set: &mut HashSet<NodeIndex>,
    min_size: usize,
    max_size: usize,
    results: &mut HashMap<String, HashSet<Vec<NodeIndex>>>,
) {
    let size = current.len();

    if size >= min_size && size <= max_size {
        let canon = canonical_form(graph, current);
        let mut sorted = current.clone();
        sorted.sort_by_key(|n| n.index());
        results.entry(canon).or_default().insert(sorted);
    }

    if size >= max_size {
        return;
    }

    let mut extension = Vec::new();
    for &node in current.iter() {
        let atom = match graph.get_atom_by_index(node) {
            Some(a) => a,
            None => continue,
        };
        for (nbr_idx, _) in graph.neighbors(atom.id) {
            if !current_set.contains(&nbr_idx) && !extension.contains(&nbr_idx) {
                extension.push(nbr_idx);
            }
        }
    }
    extension.sort_by_key(|n| n.index());

    for &nbr in &extension {
        current.push(nbr);
        current_set.insert(nbr);
        enumerate_recursive(graph, current, current_set, min_size, max_size, results);
        current.pop();
        current_set.remove(&nbr);
    }
}

fn canonical_form(graph: &CognitiveGraph, nodes: &[NodeIndex]) -> String {
    let node_set: HashSet<NodeIndex> = nodes.iter().copied().collect();

    let mut degrees: Vec<usize> = nodes
        .iter()
        .map(|&n| {
            let atom = match graph.get_atom_by_index(n) {
                Some(a) => a,
                None => return 0,
            };
            let mut deg = 0;
            for (nbr_idx, _) in graph.neighbors(atom.id) {
                if node_set.contains(&nbr_idx) {
                    deg += 1;
                }
            }
            deg
        })
        .collect();
    degrees.sort();

    let mut edge_pairs: Vec<(String, String)> = Vec::new();
    for (i, &src) in nodes.iter().enumerate() {
        let src_atom = match graph.get_atom_by_index(src) {
            Some(a) => a,
            None => continue,
        };
        for &dst in nodes.iter().skip(i + 1) {
            let dst_atom = match graph.get_atom_by_index(dst) {
                Some(a) => a,
                None => continue,
            };
            for (_, rel) in graph.relations_between(src_atom.id, dst_atom.id) {
                edge_pairs.push((
                    format!("{:?}", rel.relation_type),
                    format!("{}", rel.weight.get()),
                ));
            }
            for (_, rel) in graph.relations_between(dst_atom.id, src_atom.id) {
                edge_pairs.push((
                    format!("{:?}", rel.relation_type),
                    format!("{}", rel.weight.get()),
                ));
            }
        }
    }
    edge_pairs.sort();

    format!("{:?}|{:?}", degrees, edge_pairs)
}

pub(crate) fn compute_stability(graph: &CognitiveGraph, nodes: &[NodeIndex]) -> Activation {
    let _node_set: HashSet<NodeIndex> = nodes.iter().copied().collect();

    let mut weights = Vec::new();
    for (i, &src) in nodes.iter().enumerate() {
        let src_atom = match graph.get_atom_by_index(src) {
            Some(a) => a,
            None => continue,
        };
        for &dst in nodes.iter().skip(i + 1) {
            let dst_atom = match graph.get_atom_by_index(dst) {
                Some(a) => a,
                None => continue,
            };
            for (_, rel) in graph.relations_between(src_atom.id, dst_atom.id) {
                weights.push(rel.weight.get());
            }
            for (_, rel) in graph.relations_between(dst_atom.id, src_atom.id) {
                weights.push(rel.weight.get());
            }
        }
    }

    if weights.is_empty() {
        return Activation::ONE;
    }

    let n = Decimal::from(weights.len());
    let sum: Decimal = weights.iter().sum();
    let mean = sum / n;

    if mean == Decimal::ZERO {
        return Activation::ONE;
    }

    let variance: Decimal = weights
        .iter()
        .map(|w| {
            let diff = *w - mean;
            diff * diff
        })
        .sum::<Decimal>()
        / n;

    let stability = Decimal::ONE - (variance / mean);
    Activation::new(stability)
}

pub(crate) fn mean_edge_weight(graph: &CognitiveGraph, nodes: &[NodeIndex]) -> Decimal {
    let _node_set: HashSet<NodeIndex> = nodes.iter().copied().collect();

    let mut sum = Decimal::ZERO;
    let mut count = 0usize;

    for (i, &src) in nodes.iter().enumerate() {
        let src_atom = match graph.get_atom_by_index(src) {
            Some(a) => a,
            None => continue,
        };
        for &dst in nodes.iter().skip(i + 1) {
            let dst_atom = match graph.get_atom_by_index(dst) {
                Some(a) => a,
                None => continue,
            };
            for (_, rel) in graph.relations_between(src_atom.id, dst_atom.id) {
                sum += rel.weight.get();
                count += 1;
            }
            for (_, rel) in graph.relations_between(dst_atom.id, src_atom.id) {
                sum += rel.weight.get();
                count += 1;
            }
        }
    }

    if count == 0 {
        Decimal::ZERO
    } else {
        sum / Decimal::from(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dcma_core::atom::CognitiveAtom;
    use dcma_core::types::{AtomType, EngineConfig, RelationType};

    fn make_atom(name: &str) -> CognitiveAtom {
        CognitiveAtom::new(AtomId::new(), AtomType::Concept, name.to_string())
    }

    fn add_edge(
        graph: &mut CognitiveGraph,
        src: AtomId,
        dst: AtomId,
        rel_type: RelationType,
        weight_str: &str,
    ) {
        graph
            .add_relation_with_weight(
                src,
                dst,
                rel_type,
                Weight::new(rust_decimal::Decimal::from_str_exact(weight_str).unwrap()),
            )
            .unwrap();
    }

    #[test]
    fn test_detect_single_motif() {
        let mut graph = CognitiveGraph::new();

        // Create 5 identical triangle motifs (recurrence >= threshold of 5)
        for group in 0..5 {
            let a = make_atom(&format!("a{}", group));
            let b = make_atom(&format!("b{}", group));
            let c = make_atom(&format!("c{}", group));
            let id_a = a.id;
            let id_b = b.id;
            let id_c = c.id;
            graph.add_atom(a);
            graph.add_atom(b);
            graph.add_atom(c);
            add_edge(&mut graph, id_a, id_b, RelationType::Causes, "0.5");
            add_edge(&mut graph, id_b, id_c, RelationType::Causes, "0.5");
            add_edge(&mut graph, id_c, id_a, RelationType::Causes, "0.5");
        }

        let config = EngineConfig::default();
        let report = detect_motifs(&graph, &config);

        assert!(report.total_patterns_found > 0);
        assert_eq!(report.total_recurring, 1);
        assert_eq!(report.motifs.len(), 1);
        assert_eq!(report.motifs[0].recurrence, 5);
        assert_eq!(report.motifs[0].size, 3);
    }

    #[test]
    fn test_no_motifs_below_threshold() {
        let mut graph = CognitiveGraph::new();

        // Create 4 identical triangles (below threshold of 5)
        for group in 0..4 {
            let a = make_atom(&format!("a{}", group));
            let b = make_atom(&format!("b{}", group));
            let c = make_atom(&format!("c{}", group));
            let id_a = a.id;
            let id_b = b.id;
            let id_c = c.id;
            graph.add_atom(a);
            graph.add_atom(b);
            graph.add_atom(c);
            add_edge(&mut graph, id_a, id_b, RelationType::Causes, "0.5");
            add_edge(&mut graph, id_b, id_c, RelationType::Causes, "0.5");
            add_edge(&mut graph, id_c, id_a, RelationType::Causes, "0.5");
        }

        let config = EngineConfig::default();
        let report = detect_motifs(&graph, &config);

        assert_eq!(report.motifs.len(), 0);
        assert_eq!(report.total_recurring, 0);
    }

    #[test]
    fn test_motif_stability() {
        let mut graph = CognitiveGraph::new();

        // Triangle with uniform weights (stability = 1.0)
        let a = make_atom("a");
        let b = make_atom("b");
        let c = make_atom("c");
        let id_a = a.id;
        let id_b = b.id;
        let id_c = c.id;
        graph.add_atom(a);
        graph.add_atom(b);
        graph.add_atom(c);
        add_edge(&mut graph, id_a, id_b, RelationType::Causes, "0.5");
        add_edge(&mut graph, id_b, id_c, RelationType::Causes, "0.5");
        add_edge(&mut graph, id_c, id_a, RelationType::Causes, "0.5");

        let nodes: Vec<NodeIndex> = graph.node_indices();
        let stability = compute_stability(&graph, &nodes);
        assert_eq!(stability.get(), rust_decimal::Decimal::ONE);
    }

    #[test]
    fn test_empty_graph_no_motifs() {
        let graph = CognitiveGraph::new();
        let config = EngineConfig::default();
        let report = detect_motifs(&graph, &config);

        assert_eq!(report.motifs.len(), 0);
        assert_eq!(report.total_patterns_found, 0);
        assert_eq!(report.total_recurring, 0);
    }

    #[test]
    fn test_single_node_no_motifs() {
        let mut graph = CognitiveGraph::new();
        graph.add_atom(make_atom("solo"));

        let config = EngineConfig::default();
        let report = detect_motifs(&graph, &config);

        assert_eq!(report.motifs.len(), 0);
        assert_eq!(report.total_patterns_found, 0);
        assert_eq!(report.total_recurring, 0);
    }
}
