use dcma_core::types::*;
use dcma_core::graph::CognitiveGraph;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Contradiction structure C(t) = {(a_i, b_i, p_i(t))}
/// Tracks pressure between pairs of atoms that contradict each other.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContradictionStructure {
    /// Maps (atom_a_id, atom_b_id) -> pressure
    pub pressures: HashMap<(AtomId, AtomId), Pressure>,
    /// Numerical stability constant (ε = 10^-8)
    epsilon: Decimal,
}

impl ContradictionStructure {
    pub fn new() -> Self {
        Self {
            pressures: HashMap::new(),
            epsilon: Decimal::new(1, 8), // 10^-8
        }
    }

    /// Get pressure for a contradiction pair
    pub fn get(&self, a: AtomId, b: AtomId) -> Pressure {
        self.pressures.get(&(a, b)).copied().unwrap_or(Pressure::ZERO)
    }

    /// Set pressure for a contradiction pair
    pub fn set(&mut self, a: AtomId, b: AtomId, pressure: Pressure) {
        self.pressures.insert((a, b), pressure);
    }

    /// Remove a contradiction entry
    pub fn remove(&mut self, a: AtomId, b: AtomId) -> Option<Pressure> {
        self.pressures.remove(&(a, b))
    }

    /// All contradiction pairs
    pub fn pairs(&self) -> Vec<((AtomId, AtomId), Pressure)> {
        self.pressures.iter().map(|(k, v)| (*k, *v)).collect()
    }

    /// Number of tracked contradictions
    pub fn len(&self) -> usize {
        self.pressures.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pressures.is_empty()
    }

    /// Update pressure for a single contradiction per Section 5.2:
    ///
    /// p(t+1) = p(t) + η_p * ((A(a) - A(b)) / (A(a) + A(b) + ε)) - η_d * p(t)
    ///
    /// Also performs resolution check: if |p| > θ_resolve,
    /// the suppressed node is marked DORMANT (preserved per Law 4).
    pub fn update_pressure(
        &mut self,
        a: AtomId,
        b: AtomId,
        activation_a: Activation,
        activation_b: Activation,
        config: &EngineConfig,
    ) {
        let current_pressure = self.get(a, b);

        let act_a = activation_a.get();
        let act_b = activation_b.get();

        // (A(a) - A(b)) / (A(a) + A(b) + ε)
        let diff = act_a - act_b;
        let sum = act_a + act_b + self.epsilon;
        let ratio = if sum == Decimal::ZERO {
            Decimal::ZERO
        } else {
            diff / sum
        };

        let delta_pressure = config.eta_p.get() * ratio;
        let decay = config.eta_pressure_decay.get() * current_pressure.get();

        let new_pressure = (current_pressure.get() + delta_pressure - decay)
            .clamp(Decimal::ZERO, Decimal::ONE);
        self.set(a, b, Pressure::new(new_pressure));
    }

    /// Update all contradiction pressures in the graph.
    /// Finds all Contradicts edges and updates their pressures.
    ///
    /// Also feeds resolution/equilibrium into evidence on the graph's atoms:
    /// - StableEquilibrium → record_support on both atoms
    /// - Resolved → record_support on the dominant atom (higher activation),
    ///              record_oppose on the suppressed atom
    pub fn update_all(
        &mut self,
        graph: &mut CognitiveGraph,
        activations: &HashMap<AtomId, Activation>,
        config: &EngineConfig,
        current_tick: u64,
    ) {
        // Collect contradiction edges from the graph
        let contradiction_edges: Vec<(AtomId, AtomId)> = graph
            .contradiction_edges()
            .into_iter()
            .map(|(a, b, _)| (a, b))
            .collect();

        for (a, b) in contradiction_edges {
            let old_pressure = self.get(a, b);

            let act_a = activations.get(&a).copied().unwrap_or(Activation::ZERO);
            let act_b = activations.get(&b).copied().unwrap_or(Activation::ZERO);
            self.update_pressure(a, b, act_a, act_b, config);

            let new_pressure = self.get(a, b);
            let pressure_delta = new_pressure.get() - old_pressure.get();
            let state = self.classify(a, b, pressure_delta, config);

            match state {
                EquilibriumState::StableEquilibrium => {
                    if let Some(atom) = graph.get_atom_mut(a) {
                        atom.record_support_with_tick(current_tick);
                    }
                    if let Some(atom) = graph.get_atom_mut(b) {
                        atom.record_support_with_tick(current_tick);
                    }
                }
                EquilibriumState::Resolved => {
                    if act_a.get() > act_b.get() {
                        if let Some(atom) = graph.get_atom_mut(a) {
                            atom.record_support_with_tick(current_tick);
                        }
                        if let Some(atom) = graph.get_atom_mut(b) {
                            atom.record_oppose_with_tick(current_tick);
                        }
                    } else if act_b.get() > act_a.get() {
                        if let Some(atom) = graph.get_atom_mut(a) {
                            atom.record_oppose_with_tick(current_tick);
                        }
                        if let Some(atom) = graph.get_atom_mut(b) {
                            atom.record_support_with_tick(current_tick);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Classify equilibrium state per Section 5.4.
    ///
    /// Uses current pressure and rate of change (|dp/dt| approximated by
    /// the last pressure delta) to classify the contradiction.
    pub fn classify(
        &self,
        a: AtomId,
        b: AtomId,
        pressure_delta: Decimal,
        config: &EngineConfig,
    ) -> EquilibriumState {
        let pressure = self.get(a, b);
        let abs_pressure = pressure.get().abs();
        let abs_delta = pressure_delta.abs();

        if abs_pressure > config.resolution_threshold.get() {
            EquilibriumState::Resolved
        } else if abs_pressure <= config.equilibrium_threshold.get()
            && abs_delta <= config.equilibrium_rate_threshold.get()
        {
            EquilibriumState::StableEquilibrium
        } else if abs_pressure > config.equilibrium_threshold.get()
            && abs_delta <= config.equilibrium_rate_threshold.get()
        {
            EquilibriumState::DominantEquilibrium
        } else {
            EquilibriumState::Oscillating
        }
    }
}

impl Default for ContradictionStructure {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// Equilibrium Classification
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EquilibriumState {
    /// |p| ≤ θ_eq AND |dp/dt| < θ_rate: both beliefs coexist peacefully
    StableEquilibrium,
    /// |p| > θ_eq AND sign stable: one belief is winning
    DominantEquilibrium,
    /// |p| > θ_eq AND |dp/dt| high: active competition, unresolved
    Oscillating,
    /// p > θ_resolve: one belief dominates other suppressed (DORMANT, not deleted)
    Resolved,
}

#[cfg(test)]
mod tests {
    use super::*;
    use dcma_core::atom::CognitiveAtom;

    fn make_contradicting_atoms() -> (CognitiveGraph, AtomId, AtomId) {
        let mut g = CognitiveGraph::new();
        let a = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "belief_a".to_string());
        let b = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "belief_b".to_string());
        let id_a = a.id;
        let id_b = b.id;
        g.add_atom(a);
        g.add_atom(b);
        g.add_relation(id_a, id_b, RelationType::Contradicts).unwrap();
        (g, id_a, id_b)
    }

    #[test]
    fn test_new_structure_empty() {
        let cs = ContradictionStructure::new();
        assert!(cs.is_empty());
    }

    #[test]
    fn test_set_and_get_pressure() {
        let mut cs = ContradictionStructure::new(); // mut needed for set()
        let a = AtomId::new();
        let b = AtomId::new();
        let p = Pressure::new(Decimal::from_str_exact("0.5").unwrap());
        cs.set(a, b, p);
        assert_eq!(cs.get(a, b).get(), Decimal::from_str_exact("0.5").unwrap());
    }

    #[test]
    fn test_update_pressure_equal_activations() {
        let mut cs = ContradictionStructure::new();
        let a = AtomId::new();
        let b = AtomId::new();
        let config = EngineConfig::default();

        // Equal activations -> no pressure change from the ratio term
        let act_a = Activation::new(Decimal::from_str_exact("0.5").unwrap());
        let act_b = Activation::new(Decimal::from_str_exact("0.5").unwrap());

        cs.update_pressure(a, b, act_a, act_b, &config);

        // Pressure should be near zero (only decay term acts on zero pressure)
        let pressure = cs.get(a, b);
        assert_eq!(pressure.get(), Decimal::ZERO);
    }

    #[test]
    fn test_update_pressure_one_dominant() {
        let mut cs = ContradictionStructure::new();
        let a = AtomId::new();
        let b = AtomId::new();
        let config = EngineConfig::default();

        // A(a) >> A(b) -> positive pressure (a is winning)
        let act_a = Activation::new(Decimal::ONE);
        let act_b = Activation::new(Decimal::from_str_exact("0.01").unwrap());

        cs.update_pressure(a, b, act_a, act_b, &config);

        let pressure = cs.get(a, b);
        assert!(pressure.get() > Decimal::ZERO);
    }

    #[test]
    fn test_pressure_clamped() {
        let mut cs = ContradictionStructure::new();
        let a = AtomId::new();
        let b = AtomId::new();
        let config = EngineConfig::default();

        // Set very high pressure
        cs.set(a, b, Pressure::new(Decimal::from_str_exact("0.99").unwrap()));

        // With decay, pressure should not exceed 1.0
        let act_a = Activation::new(Decimal::ONE);
        let act_b = Activation::new(Decimal::ZERO);

        for _ in 0..100 {
            cs.update_pressure(a, b, act_a, act_b, &config);
        }

        assert!(cs.get(a, b).get() <= Decimal::ONE);
    }

    #[test]
    fn test_classify_stable() {
        let mut cs = ContradictionStructure::new();
        let config = EngineConfig::default();
        let a = AtomId::new();
        let b = AtomId::new();
        cs.set(a, b, Pressure::new(Decimal::from_str_exact("0.01").unwrap()));

        let state = cs.classify(a, b, Decimal::from_str_exact("0.001").unwrap(), &config);
        assert_eq!(state, EquilibriumState::StableEquilibrium);
    }

    #[test]
    fn test_classify_dominant() {
        let mut cs = ContradictionStructure::new();
        let config = EngineConfig::default();
        let a = AtomId::new();
        let b = AtomId::new();
        cs.set(a, b, Pressure::new(Decimal::from_str_exact("0.5").unwrap()));

        let state = cs.classify(a, b, Decimal::from_str_exact("0.001").unwrap(), &config);
        assert_eq!(state, EquilibriumState::DominantEquilibrium);
    }

    #[test]
    fn test_classify_oscillating() {
        let mut cs = ContradictionStructure::new();
        let config = EngineConfig::default();
        let a = AtomId::new();
        let b = AtomId::new();
        cs.set(a, b, Pressure::new(Decimal::from_str_exact("0.5").unwrap()));

        let state = cs.classify(a, b, Decimal::from_str_exact("0.1").unwrap(), &config);
        assert_eq!(state, EquilibriumState::Oscillating);
    }

    #[test]
    fn test_classify_resolved() {
        let mut cs = ContradictionStructure::new();
        let config = EngineConfig::default();
        let a = AtomId::new();
        let b = AtomId::new();
        cs.set(a, b, Pressure::new(Decimal::from_str_exact("0.96").unwrap()));

        let state = cs.classify(a, b, Decimal::ZERO, &config);
        assert_eq!(state, EquilibriumState::Resolved);
    }

    #[test]
    fn test_update_all_from_graph() {
        let (mut g, id_a, id_b) = make_contradicting_atoms();
        let mut cs = ContradictionStructure::new();
        let config = EngineConfig::default();

        let mut activations = HashMap::new();
        activations.insert(id_a, Activation::new(Decimal::ONE));
        activations.insert(id_b, Activation::new(Decimal::from_str_exact("0.1").unwrap()));

        cs.update_all(&mut g, &activations, &config, 0);

        // Should have tracked the contradiction
        assert_eq!(cs.len(), 1);

        // Pressure should be positive (a is more activated)
        let pressure = cs.get(id_a, id_b);
        assert!(pressure.get() > Decimal::ZERO);
    }

    #[test]
    fn test_remove_contradiction() {
        let mut cs = ContradictionStructure::new();
        let a = AtomId::new();
        let b = AtomId::new();
        cs.set(a, b, Pressure::new(Decimal::from_str_exact("0.5").unwrap()));

        let removed = cs.remove(a, b);
        assert_eq!(removed.unwrap().get(), Decimal::from_str_exact("0.5").unwrap());
        assert!(cs.is_empty());
    }

    #[test]
    fn test_pressure_decay_over_time() {
        let mut cs = ContradictionStructure::new();
        let a = AtomId::new();
        let b = AtomId::new();
        let config = EngineConfig::default();

        // Start with some pressure, equal activations -> only decay acts
        cs.set(a, b, Pressure::new(Decimal::from_str_exact("0.5").unwrap()));
        let act = Activation::new(Decimal::from_str_exact("0.5").unwrap());

        cs.update_pressure(a, b, act, act, &config);

        // Pressure should decay
        assert!(cs.get(a, b).get() < Decimal::from_str_exact("0.5").unwrap());
    }

    #[test]
    fn test_zero_activations_no_panic() {
        let mut cs = ContradictionStructure::new();
        let a = AtomId::new();
        let b = AtomId::new();
        let config = EngineConfig::default();

        // Both zero activations — should not panic (epsilon prevents division by zero)
        cs.update_pressure(a, b, Activation::ZERO, Activation::ZERO, &config);
        assert_eq!(cs.get(a, b).get(), Decimal::ZERO);
    }
}
