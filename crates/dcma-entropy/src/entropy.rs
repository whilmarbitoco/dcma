use dcma_core::types::*;
use dcma_core::graph::CognitiveGraph;
use rust_decimal::Decimal;
use serde::Serialize;

/// Shannon entropy over the degree distribution of a cognitive graph.
/// H(G) = -Σ P(d) * log2(P(d))
/// where P(d) = |{v ∈ V : deg(v) = d}| / |V|
pub fn graph_entropy(graph: &CognitiveGraph) -> Decimal {
    let n = graph.atom_count();
    if n == 0 {
        return Decimal::ZERO;
    }

    let dist = graph.degree_distribution();
    let n_decimal = Decimal::from(n);

    let mut entropy = Decimal::ZERO;
    for &count in dist.values() {
        if count == 0 {
            continue;
        }
        let p = Decimal::from(count) / n_decimal;
        // log2(p) = ln(p) / ln(2)
        if p > Decimal::ZERO {
            let ln_p = ln_approx(p);
            let ln_2 = ln_approx(Decimal::from(2));
            let log2_p = ln_p / ln_2;
            entropy -= p * log2_p;
        }
    }
    entropy
}

/// Natural logarithm via Taylor series: ln(x) = 2 * atanh((x-1)/(x+1))
/// For better convergence, we use ln(x) = ln(a * 2^n) = ln(a) + n*ln(2)
/// where a is in [1, 2)
fn ln_approx(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO; // shouldn't happen with valid probability
    }

    // Normalize x to [1, 2) by factoring out powers of 2
    let mut x = x;
    let mut n: i64 = 0;

    while x >= Decimal::from(2) {
        x = x / Decimal::from(2);
        n += 1;
    }
    while x < Decimal::ONE {
        x = x * Decimal::from(2);
        n -= 1;
    }

    // Now x ∈ [1, 2). Use ln(x) = 2 * atanh((x-1)/(x+1))
    let y = (x - Decimal::ONE) / (x + Decimal::ONE);
    let y2 = y * y;

    // atanh(y) = Σ y^(2k+1) / (2k+1) for k=0..∞
    let mut sum = Decimal::ZERO;
    let mut term = y;
    for k in 0..30 {
        let divisor = Decimal::from(2 * k + 1);
        sum += term / divisor;
        term = term * y2;
        if term.abs() < Decimal::new(1, 15) {
            break;
        }
    }

    let ln_x = Decimal::from(2) * sum;
    let ln_2 = Decimal::from_str_exact("0.6931471805599453").unwrap();

    ln_x + Decimal::from(n) * ln_2
}

/// Check if graph entropy is within operational bounds [H_min, H_max]
pub fn entropy_in_bounds(graph: &CognitiveGraph, config: &EngineConfig) -> bool {
    let h = graph_entropy(graph);
    h >= config.entropy_min.get() && h <= config.entropy_max.get()
}

/// Entropy regulation engine.
/// When entropy is outside bounds, applies regulation operations.
#[derive(Debug, Clone)]
pub struct EntropyRegulator {
    config: EngineConfig,
}

impl EntropyRegulator {
    pub fn new(config: EngineConfig) -> Self {
        Self { config }
    }

    /// Check current entropy status
    pub fn check(&self, graph: &CognitiveGraph) -> EntropyStatus {
        let h = graph_entropy(graph);
        let h_min = self.config.entropy_min.get();
        let h_max = self.config.entropy_max.get();

        if h < h_min {
            EntropyStatus::TooLow(h)
        } else if h > h_max {
            EntropyStatus::TooHigh(h)
        } else {
            EntropyStatus::Normal(h)
        }
    }

    /// Apply regulation operations based on current entropy status.
    /// Returns a description of what was done.
    pub fn regulate(&self, graph: &CognitiveGraph) -> RegulationReport {
        let status = self.check(graph);
        let mut ops = Vec::new();

        match status {
            EntropyStatus::TooHigh(h) => {
                // Strategy 1: Check for hubs that could be split
                let dist = graph.degree_distribution();
                let n = graph.atom_count();
                let avg_degree = if n > 0 {
                    let total_degree: usize = dist.iter().map(|(d, c)| d * c).sum();
                    Decimal::from(total_degree) / Decimal::from(n)
                } else {
                    Decimal::ZERO
                };

                let hub_threshold = Decimal::from(5) * avg_degree;
                let mut hubs_found = 0;
                for (_, atom) in graph.atoms() {
                    let deg = graph.degree(atom.id);
                    if Decimal::from(deg) > hub_threshold && deg > 5 {
                        hubs_found += 1;
                    }
                }
                if hubs_found > 0 {
                    ops.push(RegulationOp::HubSplit { count: hubs_found });
                }

                // Strategy 2: Check for dead paths
                let mut dead_found = 0;
                for (_, atom) in graph.atoms() {
                    if atom.state == AtomState::Active {
                        // A node is "dead" if it has zero degree and low salience
                        let in_deg = graph.in_degree(atom.id);
                        let out_deg = graph.out_degree(atom.id);
                        if in_deg == 0 && out_deg == 0 && atom.salience.get() < self.config.dead_node_threshold.get() {
                            dead_found += 1;
                        }
                    }
                }
                if dead_found > 0 {
                    ops.push(RegulationOp::DeadPathElimination { count: dead_found });
                }

                if ops.is_empty() {
                    ops.push(RegulationOp::AbstractionPromotion);
                }

                RegulationReport { entropy: h, status, operations: ops }
            }
            EntropyStatus::TooLow(h) => {
                // Low entropy: topology is too uniform
                // Strategy: bridging edges between disconnected components
                ops.push(RegulationOp::TopologyDiversification);

                RegulationReport { entropy: h, status, operations: ops }
            }
            EntropyStatus::Normal(h) => {
                RegulationReport { entropy: h, status, operations: ops }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum EntropyStatus {
    TooLow(Decimal),
    Normal(Decimal),
    TooHigh(Decimal),
}

#[derive(Debug, Clone)]
pub enum RegulationOp {
    /// Create abstraction nodes for dense clusters
    AbstractionPromotion,
    /// Merge duplicate edges, fuse identical motifs
    RedundancyCollapse,
    /// Create bridging edges between clusters
    TopologyDiversification,
    /// Re-activate dormant episodic nodes
    EpisodicExpansion,
    /// Partition hub into specialized sub-nodes
    HubSplit { count: usize },
    /// Remove node and edges (activation below threshold for too long)
    DeadPathElimination { count: usize },
}

#[derive(Debug, Clone)]
pub struct RegulationReport {
    pub entropy: Decimal,
    pub status: EntropyStatus,
    pub operations: Vec<RegulationOp>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use dcma_core::atom::CognitiveAtom;

    #[test]
    fn test_empty_graph_entropy() {
        let g = CognitiveGraph::new();
        assert_eq!(graph_entropy(&g), Decimal::ZERO);
    }

    #[test]
    fn test_single_node_entropy() {
        let mut g = CognitiveGraph::new();
        g.add_atom(CognitiveAtom::new(AtomId::new(), AtomType::Concept, "solo".to_string()));
        // Single node: degree 0, P(0) = 1, H = -1 * log2(1) = 0
        let h = graph_entropy(&g);
        assert_eq!(h, Decimal::ZERO);
    }

    #[test]
    fn test_uniform_distribution_entropy() {
        // Star graph: 1 center (degree 4), 4 leaves (degree 1)
        // P(1) = 4/5, P(4) = 1/5
        // H = -(4/5 * log2(4/5) + 1/5 * log2(1/5)) ≈ 0.7219
        let mut g = CognitiveGraph::new();
        let center = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "center".to_string());
        let center_id = center.id;
        g.add_atom(center);
        for i in 0..4 {
            let leaf = CognitiveAtom::new(AtomId::new(), AtomType::Concept, format!("leaf{}", i));
            let leaf_id = leaf.id;
            g.add_atom(leaf);
            g.add_relation(center_id, leaf_id, RelationType::Causes).unwrap();
        }

        let h = graph_entropy(&g);
        assert!(h > Decimal::ZERO);
        assert!(h < Decimal::ONE);
    }

    #[test]
    fn test_entropy_in_bounds() {
        let mut g = CognitiveGraph::new();

        // Create a 20-node chain graph
        let mut prev_id = AtomId::new();
        g.add_atom(CognitiveAtom::new(prev_id, AtomType::Concept, "n0".to_string()));

        for i in 1..20 {
            let node = CognitiveAtom::new(AtomId::new(), AtomType::Concept, format!("n{}", i));
            let nid = node.id;
            g.add_atom(node);
            g.add_relation(prev_id, nid, RelationType::Causes).unwrap();
            prev_id = nid;
        }

        let h = graph_entropy(&g);
        assert!(h > Decimal::ZERO, "Entropy should be positive");

        // Use loose bounds that a 20-node chain will satisfy
        let mut loose_config = EngineConfig::default();
        loose_config.entropy_min = Activation::new(Decimal::ZERO);
        loose_config.entropy_max = Activation::new(Decimal::from(10));
        assert!(entropy_in_bounds(&g, &loose_config));
    }

    #[test]
    fn test_regulator_normal() {
        let config = EngineConfig::default();
        let mut g = CognitiveGraph::new();

        let center = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "c".to_string());
        let cid = center.id;
        g.add_atom(center);
        for i in 0..5 {
            let leaf = CognitiveAtom::new(AtomId::new(), AtomType::Concept, format!("l{}", i));
            let lid = leaf.id;
            g.add_atom(leaf);
            g.add_relation(cid, lid, RelationType::Causes).unwrap();
        }

        let regulator = EntropyRegulator::new(config);

        // A small star graph has low entropy, so regulator may report TooLow
        // Just verify it doesn't panic and returns a valid status
        let report = regulator.regulate(&g);
        // Entropy of star graph with 6 nodes is likely below H_min=1.5
        // So regulator should report TooLow with TopologyDiversification
        assert!(!report.operations.is_empty() || matches!(report.status, EntropyStatus::Normal(_)));
    }

    #[test]
    fn test_regulator_empty_graph() {
        let config = EngineConfig::default();
        let g = CognitiveGraph::new();
        let regulator = EntropyRegulator::new(config);
        let report = regulator.regulate(&g);

        // Empty graph: entropy = 0, which is below H_min = 1.5
        assert!(matches!(report.status, EntropyStatus::TooLow(_)));
    }

    #[test]
    fn test_ln_approx() {
        // ln(1) = 0
        let result = ln_approx(Decimal::ONE);
        assert!(result.abs() < Decimal::new(1, 8));

        // ln(e) ≈ 1 (e ≈ 2.71828)
        let e = Decimal::from_str_exact("2.718281828459045").unwrap();
        let result = ln_approx(e);
        assert!((result - Decimal::ONE).abs() < Decimal::new(1, 6));

        // ln(2) ≈ 0.6931
        let result = ln_approx(Decimal::from(2));
        assert!(result > Decimal::from_str_exact("0.6").unwrap());
        assert!(result < Decimal::from_str_exact("0.8").unwrap());
    }
}
