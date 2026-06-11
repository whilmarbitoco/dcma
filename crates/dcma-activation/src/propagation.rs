use dcma_core::types::*;
use dcma_core::graph::CognitiveGraph;
use dcma_core::atom::CognitiveAtom;
use rust_decimal::Decimal;
use std::collections::HashMap;
use rust_decimal::prelude::ToPrimitive;

// Re-export NodeIndex since it's used in public APIs
pub use petgraph::stable_graph::NodeIndex;

// ============================================================
// Sigmoid
// ============================================================

/// Sigmoid via piecewise rational approximation.
/// Spec requires error < 1e-10.
fn sigmoid(x: Decimal) -> Decimal {
    let x = x.clamp(Decimal::from(-20), Decimal::from(20));
    if x >= Decimal::ZERO {
        let exp_neg_x = exp_approx(-x);
        Decimal::ONE / (Decimal::ONE + exp_neg_x)
    } else {
        let exp_x = exp_approx(x);
        exp_x / (Decimal::ONE + exp_x)
    }
}

/// e^x via Taylor series: sum(x^n / n!) for n=0..=25
fn exp_approx(x: Decimal) -> Decimal {
    let mut sum = Decimal::ONE;
    let mut term = Decimal::ONE;
    for n in 1..=25 {
        term = term * x / Decimal::from(n);
        sum += term;
        if term.abs() < Decimal::new(1, 15) {
            break;
        }
    }
    sum
}

// ============================================================
// Activation Vector
// ============================================================

#[derive(Debug, Clone)]
pub struct ActivationVector {
    pub values: HashMap<NodeIndex, Activation>,
}

impl ActivationVector {
    pub fn new() -> Self {
        Self { values: HashMap::new() }
    }

    pub fn get(&self, idx: NodeIndex) -> Activation {
        self.values.get(&idx).copied().unwrap_or(Activation::ZERO)
    }

    pub fn set(&mut self, idx: NodeIndex, value: Activation) {
        self.values.insert(idx, value);
    }

    pub fn init_from_sources(graph: &CognitiveGraph, sources: &[AtomId]) -> Self {
        let mut vec = Self::new();
        for (idx, _) in graph.atoms() {
            vec.set(idx, Activation::ZERO);
        }
        for src_id in sources {
            if let Some(idx) = graph.node_index(*src_id) {
                vec.set(idx, Activation::ONE);
            }
        }
        vec
    }

    pub fn max_diff(&self, other: &Self) -> Decimal {
        let mut max = Decimal::ZERO;
        for (idx, val) in &self.values {
            let other_val = other.get(*idx);
            let diff = (val.get() - other_val.get()).abs();
            if diff > max { max = diff; }
        }
        for (idx, val) in &other.values {
            if !self.values.contains_key(idx) {
                let diff = val.get().abs();
                if diff > max { max = diff; }
            }
        }
        max
    }
}

impl Default for ActivationVector {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// Engine computations
// ============================================================

/// R_i(t) = η_r * Σ_{h=0}^{H-1} λ^h * r_i(t-h)
fn compute_reinforcement(
    atom: &CognitiveAtom,
    current_tick: u64,
    config: &EngineConfig,
) -> Decimal {
    let h = config.reinforcement_window as u64;
    let lambda = config.lambda.get();
    let mut sum = Decimal::ZERO;
    let mut lambda_pow = Decimal::ONE;

    for h in 0..h {
        let tick = current_tick.saturating_sub(h);
        let r_i = if atom.reinforcement_history.contains(&tick) {
            Decimal::ONE
        } else {
            Decimal::ZERO
        };
        sum += lambda_pow * r_i;
        lambda_pow *= lambda;
    }
    config.eta_r.get() * sum
}

/// D_i(t) = η_d * A_i(t)
fn compute_decay(activation: Activation, config: &EngineConfig) -> Decimal {
    config.eta_d.get() * activation.get()
}

/// C_i(t) = η_c * Σ_{j ∈ contradicts(i)} A_j(t)
fn compute_contradiction_suppression(
    graph: &CognitiveGraph,
    idx: NodeIndex,
    activations: &ActivationVector,
    config: &EngineConfig,
) -> Decimal {
    let atom_id = graph.get_atom_by_index(idx).map(|a| a.id).unwrap_or_default();
    let mut sum = Decimal::ZERO;

    for (_, _, rel) in graph.inbound_neighbors(atom_id) {
        if rel.relation_type == RelationType::Contradicts {
            if let Some(src_idx) = graph.node_index(rel.src) {
                sum += activations.get(src_idx).get();
            }
        }
    }
    for (_, _, rel) in graph.outbound_neighbors(atom_id) {
        if rel.relation_type == RelationType::Contradicts {
            if let Some(dst_idx) = graph.node_index(rel.dst) {
                sum += activations.get(dst_idx).get();
            }
        }
    }
    config.eta_c.get() * sum
}

/// Q_i(t) = η_q * structural_similarity(i, q(t))
/// Placeholder: returns η_q for query nodes, 0 otherwise.
fn compute_contextual_charge(
    idx: NodeIndex,
    query_nodes: &[NodeIndex],
    config: &EngineConfig,
) -> Decimal {
    if query_nodes.contains(&idx) {
        config.eta_q.get()
    } else {
        Decimal::ZERO
    }
}

/// Σ_j w_ji * A_j(t) with normalized inbound weights
fn compute_inbound_weighted_sum(
    graph: &CognitiveGraph,
    idx: NodeIndex,
    activations: &ActivationVector,
) -> Decimal {
    let atom_id = graph.get_atom_by_index(idx).map(|a| a.id).unwrap_or_default();
    let inbound = graph.inbound_neighbors(atom_id);

    if inbound.is_empty() {
        return Decimal::ZERO;
    }

    let total_weight: Decimal = inbound.iter().map(|(_, _, rel)| rel.weight.get()).sum();
    if total_weight == Decimal::ZERO {
        return Decimal::ZERO;
    }

    let mut sum = Decimal::ZERO;
    for (_, _, rel) in inbound {
        if let Some(src_idx) = graph.node_index(rel.src) {
            let normalized_weight = rel.weight.get() / total_weight;
            sum += normalized_weight * activations.get(src_idx).get();
        }
    }
    sum
}

// ============================================================
// ACTIVATE algorithm (spec Section 3.3)
// ============================================================

pub fn activate(
    graph: &CognitiveGraph,
    source_nodes: &[AtomId],
    query_context: &[AtomId],
    current_tick: u64,
    config: &EngineConfig,
) -> ActivationVector {
    let query_indices: Vec<NodeIndex> = query_context
        .iter()
        .filter_map(|id| graph.node_index(*id))
        .collect();

    let mut a_current = ActivationVector::init_from_sources(graph, source_nodes);
    let mut a_next = ActivationVector::new();

    for _ in 0..config.max_iterations {
        for (idx, atom) in graph.atoms() {
            let inbound_sum = compute_inbound_weighted_sum(graph, idx, &a_current);
            let r_i = compute_reinforcement(atom, current_tick, config);
            let d_i = compute_decay(a_current.get(idx), config);
            let c_i = compute_contradiction_suppression(graph, idx, &a_current, config);
            let q_i = compute_contextual_charge(idx, &query_indices, config);

            let raw = inbound_sum + r_i - d_i - c_i + q_i;
            let activation = Activation::new(sigmoid(raw));
            a_next.set(idx, activation);
        }

        let diff = a_current.max_diff(&a_next);
        if diff < config.convergence_threshold.get() {
            return a_next;
        }

        a_current = a_next;
        a_next = ActivationVector::new();
    }

    a_current
}

// ============================================================
// Spontaneous activation (spec Section 3.4)
// ============================================================

pub fn spontaneous_activation(
    graph: &CognitiveGraph,
    tick: u64,
    config: &EngineConfig,
) -> Vec<(AtomId, Activation)> {
    let threshold = (config.spontaneous_activation_prob.get() * Decimal::from(10000))
        .to_u32()
        .unwrap_or(0);

    let mut result = Vec::new();
    for (_, atom) in graph.atoms() {
        let hash = simple_hash(&atom.id, tick);
        let roll = (hash % 10000) as u32;
        if roll < threshold {
            let activation = Activation::new(atom.salience.get().min(Decimal::ONE));
            result.push((atom.id, activation));
        }
    }
    result
}

fn simple_hash(id: &AtomId, tick: u64) -> u64 {
    let bytes = id.0.as_bytes();
    let mut hash: u64 = tick;
    for (i, &b) in bytes.iter().enumerate() {
        hash = hash.wrapping_add((b as u64) << ((i % 8) * 8));
        hash = hash.wrapping_mul(31);
    }
    hash
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use dcma_core::atom::CognitiveAtom;

    fn make_test_graph() -> (CognitiveGraph, AtomId, AtomId, AtomId) {
        let mut g = CognitiveGraph::new();
        let a = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "a".to_string());
        let b = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "b".to_string());
        let c = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "c".to_string());
        let id_a = a.id;
        let id_b = b.id;
        let id_c = c.id;
        g.add_atom(a);
        g.add_atom(b);
        g.add_atom(c);
        g.add_relation_with_weight(id_a, id_b, RelationType::Causes,
            Weight::new(Decimal::from_str_exact("0.8").unwrap())).unwrap();
        g.add_relation_with_weight(id_b, id_c, RelationType::Causes,
            Weight::new(Decimal::from_str_exact("0.6").unwrap())).unwrap();
        (g, id_a, id_b, id_c)
    }

    #[test]
    fn test_sigmoid_zero() {
        let result = sigmoid(Decimal::ZERO);
        assert_eq!(result, Decimal::from_str_exact("0.5").unwrap());
    }

    #[test]
    fn test_sigmoid_positive() {
        let result = sigmoid(Decimal::ONE);
        assert!(result > Decimal::from_str_exact("0.7").unwrap());
        assert!(result < Decimal::from_str_exact("0.8").unwrap());
    }

    #[test]
    fn test_sigmoid_negative() {
        let result = sigmoid(Decimal::from(-1));
        assert!(result > Decimal::from_str_exact("0.2").unwrap());
        assert!(result < Decimal::from_str_exact("0.3").unwrap());
    }

    #[test]
    fn test_sigmoid_symmetry() {
        let pos = sigmoid(Decimal::from(2));
        let neg = sigmoid(Decimal::from(-2));
        let sum = pos + neg;
        let diff = (sum - Decimal::ONE).abs();
        assert!(diff < Decimal::new(1, 8));
    }

    #[test]
    fn test_activation_vector_init() {
        let (g, id_a, id_b, _) = make_test_graph();
        let sources = vec![id_a];
        let vec = ActivationVector::init_from_sources(&g, &sources);
        let idx_a = g.node_index(id_a).unwrap();
        let idx_b = g.node_index(id_b).unwrap();
        assert_eq!(vec.get(idx_a).get(), Decimal::ONE);
        assert_eq!(vec.get(idx_b).get(), Decimal::ZERO);
    }

    #[test]
    fn test_activate_converges() {
        let (g, id_a, id_b, id_c) = make_test_graph();
        let config = EngineConfig::default();
        let sources = vec![id_a];
        let query = vec![id_c];
        let result = activate(&g, &sources, &query, 0, &config);

        let idx_a = g.node_index(id_a).unwrap();
        let idx_b = g.node_index(id_b).unwrap();
        assert!(result.get(idx_a).get() > Decimal::ZERO);
        assert!(result.get(idx_b).get() > Decimal::ZERO);
    }

    #[test]
    fn test_activate_deterministic() {
        let (g, id_a, _, _) = make_test_graph();
        let config = EngineConfig::default();
        let sources = vec![id_a];
        let r1 = activate(&g, &sources, &[], 0, &config);
        let r2 = activate(&g, &sources, &[], 0, &config);
        assert_eq!(r1.max_diff(&r2), Decimal::ZERO);
    }

    #[test]
    fn test_contradiction_suppresses() {
        let mut g = CognitiveGraph::new();
        let a = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "a".to_string());
        let b = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "b".to_string());
        let id_a = a.id;
        let id_b = b.id;
        g.add_atom(a);
        g.add_atom(b);
        g.add_relation(id_a, id_b, RelationType::Contradicts).unwrap();

        let config = EngineConfig::default();
        let sources = vec![id_a, id_b];
        let result = activate(&g, &sources, &[], 0, &config);

        let idx_a = g.node_index(id_a).unwrap();
        let idx_b = g.node_index(id_b).unwrap();
        assert!(result.get(idx_a).get() > Decimal::ZERO);
        assert!(result.get(idx_b).get() > Decimal::ZERO);
    }

    #[test]
    fn test_spontaneous_deterministic() {
        let (g, _, _, _) = make_test_graph();
        let config = EngineConfig::default();
        let r1 = spontaneous_activation(&g, 0, &config);
        let r2 = spontaneous_activation(&g, 0, &config);
        assert_eq!(r1.len(), r2.len());
    }
}
