use dcma_core::graph::CognitiveGraph;
use dcma_core::types::*;
use rust_decimal::Decimal;
use rust_decimal::MathematicalOps;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct Partition {
    pub id: usize,
    pub nodes: Vec<AtomId>,
    pub internal_edges: usize,
    pub external_edges: usize,
}

#[derive(Debug, Clone)]
pub struct PartitionReport {
    pub partitions: Vec<Partition>,
    pub cut_edges: usize,
    pub balance_score: Decimal,
}

fn mat_vec_mul(mat: &[Vec<Decimal>], v: &[Decimal], n: usize) -> Vec<Decimal> {
    let mut result = vec![Decimal::ZERO; n];
    for i in 0..n {
        for j in 0..n {
            result[i] += mat[i][j] * v[j];
        }
    }
    result
}

fn vec_norm_sq(v: &[Decimal]) -> Decimal {
    v.iter().map(|x| *x * *x).sum()
}

fn vec_norm(v: &[Decimal]) -> Decimal {
    vec_norm_sq(v).sqrt().unwrap_or(Decimal::ONE)
}

fn rayleigh_quotient(mat: &[Vec<Decimal>], v: &[Decimal], n: usize) -> Decimal {
    let mv = mat_vec_mul(mat, v, n);
    let mut num = Decimal::ZERO;
    for i in 0..n {
        num += v[i] * mv[i];
    }
    let denom = vec_norm_sq(v);
    if denom == Decimal::ZERO {
        Decimal::ZERO
    } else {
        num / denom
    }
}

fn power_iteration(
    mat: &[Vec<Decimal>],
    n: usize,
    max_iter: usize,
) -> (Vec<Decimal>, Decimal) {
    let init_val = Decimal::ONE / Decimal::from(n as u64).sqrt().unwrap();
    let mut v = vec![init_val; n];

    for _ in 0..max_iter {
        let w = mat_vec_mul(mat, &v, n);
            let norm = vec_norm(&w);
        let mut w_normalized = w;
        if norm > Decimal::ZERO {
            for x in &mut w_normalized {
                *x = *x / norm;
            }
        }
        v = w_normalized;
    }

    let lambda = rayleigh_quotient(mat, &v, n);
    (v, lambda)
}

fn deflate(
    mat: &[Vec<Decimal>],
    n: usize,
    eigenvector: &[Decimal],
    eigenvalue: Decimal,
) -> Vec<Vec<Decimal>> {
    let norm_sq = vec_norm_sq(eigenvector);
    if norm_sq == Decimal::ZERO {
        return mat.to_vec();
    }
    let mut m = mat.to_vec();
    for i in 0..n {
        for j in 0..n {
            m[i][j] = m[i][j] - eigenvalue * eigenvector[i] * eigenvector[j] / norm_sq;
        }
    }
    m
}

fn estimate_lambda_max(lap: &[Vec<Decimal>], n: usize) -> Decimal {
    let mut max_val = Decimal::ZERO;
    for i in 0..n {
        let row_sum: Decimal = lap[i].iter().map(|x| x.abs()).sum();
        if row_sum > max_val {
            max_val = row_sum;
        }
    }
    max_val
}

fn shifted_matrix(lap: &[Vec<Decimal>], n: usize, shift: Decimal) -> Vec<Vec<Decimal>> {
    let mut m = lap.to_vec();
    for i in 0..n {
        m[i][i] = shift - lap[i][i];
    }
    m
}

fn build_laplacian(graph: &CognitiveGraph) -> (Vec<Vec<Decimal>>, Vec<AtomId>) {
    let nodes: Vec<AtomId> = graph.atoms().map(|(_, atom)| atom.id).collect();
    let n = nodes.len();

    let mut adj = vec![vec![Decimal::ZERO; n]; n];
    let mut deg = vec![Decimal::ZERO; n];

    for (_, rel) in graph.relations() {
        let src_pos = match nodes.iter().position(|id| *id == rel.src) {
            Some(p) => p,
            None => continue,
        };
        let dst_pos = match nodes.iter().position(|id| *id == rel.dst) {
            Some(p) => p,
            None => continue,
        };
        let w = rel.weight.get();
        adj[src_pos][dst_pos] += w;
        if src_pos != dst_pos {
            adj[dst_pos][src_pos] += w;
        }
        deg[src_pos] += w;
        deg[dst_pos] += w;
    }

    let mut lap = vec![vec![Decimal::ZERO; n]; n];
    for i in 0..n {
        for j in 0..n {
            if i == j {
                lap[i][j] = deg[i];
            } else {
                lap[i][j] = -adj[i][j];
            }
        }
    }

    (lap, nodes)
}

fn fiedler_vector(lap: &[Vec<Decimal>], n: usize, max_iter: usize) -> Vec<Decimal> {
    let lambda_max = estimate_lambda_max(lap, n);
    let shift = lambda_max + Decimal::ONE;
    let m = shifted_matrix(lap, n, shift);

    let (v1, lambda1) = power_iteration(&m, n, max_iter);
    let m2 = deflate(&m, n, &v1, lambda1);
    let (v2, _) = power_iteration(&m2, n, max_iter);
    v2
}

fn smallest_eigenvectors(
    lap: &[Vec<Decimal>],
    n: usize,
    num_vectors: usize,
    max_iter: usize,
) -> Vec<Vec<Decimal>> {
    let lambda_max = estimate_lambda_max(lap, n);
    let shift = lambda_max + Decimal::ONE;
    let mut m = shifted_matrix(lap, n, shift);

    let mut eigenvectors = Vec::new();
    for _ in 0..num_vectors {
        let (v, lambda) = power_iteration(&m, n, max_iter);
        eigenvectors.push(v.clone());
        m = deflate(&m, n, &eigenvectors.last().unwrap(), lambda);
    }
    eigenvectors
}

fn assign_by_sign(vector: &[Decimal]) -> Vec<usize> {
    let n = vector.len();
    let mut sorted = vector.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = sorted[n / 2];
    vector
        .iter()
        .map(|v| if *v >= median { 0 } else { 1 })
        .collect()
}

fn assign_by_largest_component(
    eigenvectors: &[Vec<Decimal>],
    n: usize,
) -> Vec<usize> {
    let k = eigenvectors.len();
    if k == 0 {
        return vec![0; n];
    }
    (0..n)
        .map(|i| {
            let (best_j, _) = eigenvectors
                .iter()
                .enumerate()
                .map(|(j, ev)| (j, ev[i].abs()))
                .max_by(|a, b| {
                    a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap_or((0, Decimal::ZERO));
            best_j
        })
        .collect()
}

fn count_internal_edges(graph: &CognitiveGraph, partition_nodes: &[AtomId]) -> usize {
    let node_set: HashSet<AtomId> = partition_nodes.iter().copied().collect();
    let mut count = 0;
    for (_, rel) in graph.relations() {
        if node_set.contains(&rel.src) && node_set.contains(&rel.dst) {
            count += 1;
        }
    }
    count
}

fn count_external_edges(graph: &CognitiveGraph, partition_nodes: &[AtomId]) -> usize {
    let node_set: HashSet<AtomId> = partition_nodes.iter().copied().collect();
    let mut count = 0;
    for (_, rel) in graph.relations() {
        let src_in = node_set.contains(&rel.src);
        let dst_in = node_set.contains(&rel.dst);
        if src_in != dst_in {
            count += 1;
        }
    }
    count
}

pub fn partition(graph: &CognitiveGraph, k: usize) -> PartitionReport {
    let (lap, nodes) = build_laplacian(graph);
    let n = nodes.len();

    if n == 0 {
        return PartitionReport {
            partitions: Vec::new(),
            cut_edges: 0,
            balance_score: Decimal::ZERO,
        };
    }

    if k <= 1 || n <= 1 {
        let internal = count_internal_edges(graph, &nodes);
        let external = count_external_edges(graph, &nodes);
        let part = Partition {
            id: 0,
            nodes: nodes.clone(),
            internal_edges: internal,
            external_edges: external,
        };
        return PartitionReport {
            partitions: vec![part],
            cut_edges: 0,
            balance_score: Decimal::ONE,
        };
    }

    if k >= n {
        let partitions: Vec<Partition> = nodes
            .iter()
            .enumerate()
            .map(|(i, id)| Partition {
                id: i,
                nodes: vec![*id],
                internal_edges: 0,
                external_edges: count_external_edges(graph, &[*id]),
            })
            .collect();
        let total_cut: usize = partitions.iter().map(|p| p.external_edges).sum::<usize>() / 2;
        let balance = if k > 0 {
            Decimal::ONE / Decimal::from(k as u64)
        } else {
            Decimal::ZERO
        };
        return PartitionReport {
            partitions,
            cut_edges: total_cut,
            balance_score: balance,
        };
    }

    let max_iter = 100;
    let assignments: Vec<usize> = if k == 2 {
        let fv = fiedler_vector(&lap, n, max_iter);
        assign_by_sign(&fv)
    } else {
        let evs = smallest_eigenvectors(&lap, n, k, max_iter);
        assign_by_largest_component(&evs, n)
    };

    let mut partition_map: HashMap<usize, Vec<AtomId>> = HashMap::new();
    for (node_idx, &part_id) in assignments.iter().enumerate() {
        partition_map.entry(part_id).or_default().push(nodes[node_idx]);
    }

    let mut partitions: Vec<Partition> = partition_map
        .into_iter()
        .map(|(part_id, part_nodes)| {
            let internal = count_internal_edges(graph, &part_nodes);
            let external = count_external_edges(graph, &part_nodes);
            Partition {
                id: part_id,
                nodes: part_nodes,
                internal_edges: internal,
                external_edges: external,
            }
        })
        .collect();
    partitions.sort_by_key(|p| p.id);

    let cut_edges: usize = partitions.iter().map(|p| p.external_edges).sum::<usize>() / 2;

    let sizes: Vec<usize> = partitions.iter().map(|p| p.nodes.len()).collect();
    let min_size = sizes.iter().copied().min().unwrap_or(0);
    let max_size = sizes.iter().copied().max().unwrap_or(0);
    let balance_score = if max_size > 0 && k > 0 {
        Decimal::from(min_size as u64) / Decimal::from(max_size as u64)
    } else {
        Decimal::ZERO
    };

    PartitionReport {
        partitions,
        cut_edges,
        balance_score,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dcma_core::atom::CognitiveAtom;

    fn make_test_graph(num_nodes: usize) -> CognitiveGraph {
        let mut graph = CognitiveGraph::new();
        let atoms: Vec<CognitiveAtom> = (0..num_nodes)
            .map(|i| {
                CognitiveAtom::new(AtomId::new(), AtomType::Concept, format!("n{}", i))
            })
            .collect();
        let ids: Vec<AtomId> = atoms.iter().map(|a| a.id).collect();
        for a in atoms {
            graph.add_atom(a);
        }
        // Create a chain graph: 0-1-2-3-...-(num_nodes-1)
        for i in 0..num_nodes - 1 {
            graph
                .add_relation(ids[i], ids[i + 1], RelationType::Causes)
                .unwrap();
        }
        graph
    }

    #[test]
    fn test_partition_count() {
        let graph = make_test_graph(10);
        let k = 3;
        let report = partition(&graph, k);
        assert_eq!(
            report.partitions.len(),
            k,
            "should produce {} partitions, got {}",
            k,
            report.partitions.len()
        );
    }

    #[test]
    fn test_partition_coverage() {
        let graph = make_test_graph(10);
        let k = 3;
        let report = partition(&graph, k);
        let mut assigned: HashSet<AtomId> = HashSet::new();
        for p in &report.partitions {
            for id in &p.nodes {
                assert!(
                    assigned.insert(*id),
                    "node {:?} assigned to multiple partitions",
                    id
                );
            }
        }
        let total_nodes: usize = graph.atoms().count();
        assert_eq!(
            assigned.len(),
            total_nodes,
            "all {} nodes should be assigned, got {}",
            total_nodes,
            assigned.len()
        );
    }

    #[test]
    fn test_partition_balance() {
        let graph = make_test_graph(12);
        let k = 3;
        let report = partition(&graph, k);
        let sizes: Vec<usize> = report.partitions.iter().map(|p| p.nodes.len()).collect();
        let min_size = sizes.iter().copied().min().unwrap_or(0);
        let max_size = sizes.iter().copied().max().unwrap_or(0);
        assert!(
            max_size <= min_size * 2 || min_size == 0,
            "no partition should have >2x nodes of another: sizes={:?}",
            sizes
        );
    }

    #[test]
    fn test_partition_single_node() {
        let graph = make_test_graph(1);
        let report = partition(&graph, 2);
        assert_eq!(report.partitions.len(), 1);
        assert_eq!(report.partitions[0].nodes.len(), 1);
    }

    #[test]
    fn test_partition_empty_graph() {
        let graph = CognitiveGraph::new();
        let report = partition(&graph, 3);
        assert!(report.partitions.is_empty());
    }

    #[test]
    fn test_partition_k_one() {
        let graph = make_test_graph(10);
        let report = partition(&graph, 1);
        assert_eq!(report.partitions.len(), 1);
        assert_eq!(report.partitions[0].nodes.len(), 10);
        assert_eq!(report.balance_score, Decimal::ONE);
    }

    #[test]
    fn test_partition_k_equals_n() {
        let graph = make_test_graph(5);
        let report = partition(&graph, 5);
        assert_eq!(report.partitions.len(), 5);
        for p in &report.partitions {
            assert_eq!(p.nodes.len(), 1);
        }
    }
}
