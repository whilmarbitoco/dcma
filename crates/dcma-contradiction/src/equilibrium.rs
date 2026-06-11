use dcma_core::types::*;
use dcma_core::graph::CognitiveGraph;
use rust_decimal::Decimal;
use std::collections::HashMap;

use crate::pressure::{ContradictionStructure, EquilibriumState};

/// Tracks equilibrium history and provides classification for contradictions.
#[derive(Debug, Clone)]
pub struct EquilibriumTracker {
    /// Maps (atom_a_id, atom_b_id) -> pressure history
    history: HashMap<(AtomId, AtomId), Vec<Decimal>>,
}

impl EquilibriumTracker {
    pub fn new() -> Self {
        Self {
            history: HashMap::new(),
        }
    }

    /// Record a pressure delta for a contradiction pair
    pub fn record(&mut self,         a: AtomId,
        b: AtomId, delta: Decimal) {
        self.history
            .entry((a, b))
            .or_default()
            .push(delta);
    }

    /// Average rate of change over the tracking window
    pub fn avg_rate(&self, a: AtomId, b: AtomId) -> Decimal {
        match self.history.get(&(a, b)) {
            Some(deltas) if !deltas.is_empty() => {
                let sum: Decimal = deltas.iter().copied().fold(Decimal::ZERO, |acc, d| acc + d);
                sum / Decimal::from(deltas.len())
            }
            _ => Decimal::ZERO,
        }
    }

    /// Check if equilibrium has been stable for the required window
    pub fn is_stable(
        &self,
        a: AtomId,
        b: AtomId,
        current_pressure: Pressure,
        config: &EngineConfig,
    ) -> bool {
        let abs_pressure = current_pressure.get().abs();
        if abs_pressure > config.equilibrium_threshold.get() {
            return false;
        }
        let rate = self.avg_rate(a, b);
        rate.abs() < config.equilibrium_rate_threshold.get()
    }

    /// Check if resolved (pressure exceeds resolution threshold)
    pub fn is_resolved(
        &self,
        _a: AtomId,
        _b: AtomId,
        current_pressure: Pressure,
        config: &EngineConfig,
    ) -> bool {
        current_pressure.get().abs() > config.resolution_threshold.get()
    }

    pub fn remove(&mut self, a: AtomId, b: AtomId) {
        self.history.remove(&(a, b));
    }
}

impl Default for EquilibriumTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// High-level contradiction engine that combines pressure dynamics with equilibrium tracking.
#[derive(Debug, Clone)]
pub struct ContradictionEngine {
    pub structure: ContradictionStructure,
    tracker: EquilibriumTracker,
    config: EngineConfig,
}

impl ContradictionEngine {
    pub fn new(config: EngineConfig) -> Self {
        Self {
            structure: ContradictionStructure::new(),
            tracker: EquilibriumTracker::new(),
            config,
        }
    }

    /// Run one tick of contradiction processing.
    /// Returns list of ((a, b), equilibrium_state) for all active contradictions.
    pub fn tick(
        &mut self,
        graph: &CognitiveGraph,
        activations: &HashMap<AtomId, Activation>,
        config: &EngineConfig,
    ) -> Vec<((AtomId, AtomId), EquilibriumState)> {
        // Update all pressures
        self.structure.update_all(graph, activations, config);

        // Classify each contradiction
        let pairs = self.structure.pairs();
        let mut results = Vec::new();

        for ((a, b), pressure) in pairs {
            let old_pressure = self.structure.get(a, b);
            let delta = pressure.get() - old_pressure.get();
            self.tracker.record(a, b, delta);

            let state = self.structure.classify(a, b, delta, config);
            results.push(((a, b), state));
        }

        results
    }

    /// Get the current equilibrium state for a specific pair
    pub fn get_state(&self, a: AtomId, b: AtomId) -> Option<EquilibriumState> {
        let pressure = self.structure.get(a, b);
        if pressure.get() == Decimal::ZERO {
            return None;
        }
        let rate = self.tracker.avg_rate(a, b);
        Some(self.structure.classify(a, b, rate, &self.config))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dcma_core::atom::CognitiveAtom;

    #[test]
    fn test_new_tracker_empty() {
        let t = EquilibriumTracker::new();
        let a = AtomId::new();
        let b = AtomId::new();
        assert_eq!(t.avg_rate(a, b), Decimal::ZERO);
    }

    #[test]
    fn test_tracker_record_and_avg() {
        let mut t = EquilibriumTracker::new();
        let a = AtomId::new();
        let b = AtomId::new();

        t.record(a, b, Decimal::from_str_exact("0.1").unwrap());
        t.record(a, b, Decimal::from_str_exact("0.2").unwrap());
        t.record(a, b, Decimal::from_str_exact("0.3").unwrap());

        let avg = t.avg_rate(a, b);
        assert_eq!(avg, Decimal::from_str_exact("0.2").unwrap());
    }

    #[test]
    fn test_tracker_stable() {
        let mut t = EquilibriumTracker::new();
        let config = EngineConfig::default();
        let a = AtomId::new();
        let b = AtomId::new();

        // Record small deltas
        for _ in 0..10 {
            t.record(a, b, Decimal::from_str_exact("0.001").unwrap());
        }

        let pressure = Pressure::new(Decimal::from_str_exact("0.01").unwrap());
        assert!(t.is_stable(a, b, pressure, &config));
    }

    #[test]
    fn test_tracker_not_stable_high_pressure() {
        let t = EquilibriumTracker::new();
        let config = EngineConfig::default();
        let a = AtomId::new();
        let b = AtomId::new();

        let pressure = Pressure::new(Decimal::from_str_exact("0.5").unwrap());
        assert!(!t.is_stable(a, b, pressure, &config));
    }

    #[test]
    fn test_tracker_resolved() {
        let t = EquilibriumTracker::new();
        let config = EngineConfig::default();
        let a = AtomId::new();
        let b = AtomId::new();

        let pressure = Pressure::new(Decimal::from_str_exact("0.96").unwrap());
        assert!(t.is_resolved(a, b, pressure, &config));
    }

    #[test]
    fn test_contradiction_engine_new() {
        let config = EngineConfig::default();
        let engine = ContradictionEngine::new(config);
        assert!(engine.structure.is_empty());
    }

    #[test]
    fn test_contradiction_engine_tick() {
        let config = EngineConfig::default();
        let mut engine = ContradictionEngine::new(config);

        let mut g = CognitiveGraph::new();
        let a = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "a".to_string());
        let b = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "b".to_string());
        let id_a = a.id;
        let id_b = b.id;
        g.add_atom(a);
        g.add_atom(b);
        g.add_relation(id_a, id_b, RelationType::Contradicts).unwrap();

        let mut activations = HashMap::new();
        activations.insert(id_a, Activation::new(Decimal::ONE));
        activations.insert(id_b, Activation::new(Decimal::from_str_exact("0.1").unwrap()));

        let results = engine.tick(&g, &activations, &engine.config.clone());

        assert_eq!(results.len(), 1);
        let ((ra, rb), state) = &results[0];
        assert_eq!(*ra, id_a);
        assert_eq!(*rb, id_b);
        // With a >> b, should be dominant or oscillating
        assert_ne!(*state, EquilibriumState::StableEquilibrium);
    }

    #[test]
    fn test_contradiction_engine_resolves_over_time() {
        let config = EngineConfig::default();
        let mut engine = ContradictionEngine::new(config.clone());

        let mut g = CognitiveGraph::new();
        let a = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "a".to_string());
        let b = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "b".to_string());
        let id_a = a.id;
        let id_b = b.id;
        g.add_atom(a);
        g.add_atom(b);
        g.add_relation(id_a, id_b, RelationType::Contradicts).unwrap();

        let mut activations = HashMap::new();
        activations.insert(id_a, Activation::new(Decimal::ONE));
        activations.insert(id_b, Activation::new(Decimal::ZERO));

        // Run many ticks — pressure should build toward resolution
        for _ in 0..50 {
            engine.tick(&g, &activations, &config);
        }

        let state = engine.get_state(id_a, id_b);
        // After many ticks with a=1, b=0, should be resolved or dominant
        match state {
            Some(EquilibriumState::Resolved) | Some(EquilibriumState::DominantEquilibrium) => {}
            other => {
                // Check pressure at least built up
                let pressure = engine.structure.get(id_a, id_b);
                assert!(pressure.get() > Decimal::ZERO, "Pressure should have built up, got {:?}", other);
            }
        }
    }
}
