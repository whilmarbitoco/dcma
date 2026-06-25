use super::types::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveAtom {
    pub id: AtomId,
    pub atom_type: AtomType,
    pub canonical: String,
    pub state: AtomState,
    pub attributes: HashMap<String, String>,
    pub t_created: u64,
    pub t_last_accessed: u64,
    pub salience: Salience,
    pub confidence: Confidence,
    pub origin: String,
    pub reinforcement_history: Vec<u64>,
    pub activation_history: Vec<Activation>,
    pub contradiction_indices: Vec<usize>,
    pub evidence: EvidenceCounter,
    pub t_last_evidence: u64,
}

impl CognitiveAtom {
    pub fn new(id: AtomId, atom_type: AtomType, canonical: String) -> Self {
        Self {
            id,
            atom_type,
            canonical,
            state: AtomState::Active,
            attributes: HashMap::new(),
            t_created: 0,
            t_last_accessed: 0,
            salience: Salience::ZERO,
            confidence: Confidence::ZERO,
            origin: String::new(),
            reinforcement_history: Vec::new(),
            activation_history: Vec::new(),
            contradiction_indices: Vec::new(),
            evidence: EvidenceCounter::default(),
            t_last_evidence: 0,
        }
    }

    pub fn is_active(&self) -> bool {
        self.state == AtomState::Active
    }

    pub fn mark_dormant(&mut self) {
        self.state = AtomState::Dormant;
    }

    pub fn record_activation(&mut self, activation: Activation) {
        self.activation_history.push(activation);
        // Keep last 100 activations to bound memory
        if self.activation_history.len() > 100 {
            self.activation_history.remove(0);
        }
    }

    pub fn record_reinforcement(&mut self, tick: u64) {
        self.reinforcement_history.push(tick);
        self.t_last_accessed = tick;
    }

    /// Compute evidence confidence with linear decay over half_life ticks of inactivity.
    pub fn evidence_decayed_confidence(&self, half_life: u64, current_tick: u64) -> Decimal {
        if half_life == 0 {
            return self.evidence.confidence();
        }
        let elapsed = current_tick.saturating_sub(self.t_last_evidence);
        if elapsed == 0 {
            return self.evidence.confidence();
        }
        let decay_num = Decimal::from(half_life.saturating_sub(elapsed));
        let decay_den = Decimal::from(half_life);
        let decay = if decay_den == Decimal::ZERO {
            Decimal::ONE
        } else {
            (decay_num / decay_den).max(Decimal::ZERO)
        };
        let s = Decimal::from(self.evidence.support) * decay;
        let o = Decimal::from(self.evidence.oppose) * decay;
        s / (s + o + Decimal::ONE)
    }

    pub fn record_support_with_tick(&mut self, tick: u64) {
        self.evidence.record_support();
        self.t_last_evidence = tick;
    }

    pub fn record_oppose_with_tick(&mut self, tick: u64) {
        self.evidence.record_oppose();
        self.t_last_evidence = tick;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_atom() {
        let id = AtomId::new();
        let atom = CognitiveAtom::new(id, AtomType::Concept, "test_concept".to_string());
        assert_eq!(atom.id, id);
        assert_eq!(atom.atom_type, AtomType::Concept);
        assert_eq!(atom.canonical, "test_concept");
        assert!(atom.is_active());
        assert_eq!(atom.state, AtomState::Active);
    }

    #[test]
    fn test_mark_dormant() {
        let mut atom = CognitiveAtom::new(AtomId::new(), AtomType::Entity, "foo".to_string());
        assert!(atom.is_active());
        atom.mark_dormant();
        assert!(!atom.is_active());
        assert_eq!(atom.state, AtomState::Dormant);
    }

    #[test]
    fn test_record_activation() {
        let mut atom = CognitiveAtom::new(AtomId::new(), AtomType::Event, "bar".to_string());
        let a = Activation::new(rust_decimal::Decimal::from_str_exact("0.75").unwrap());
        atom.record_activation(a);
        assert_eq!(atom.activation_history.len(), 1);
        assert_eq!(atom.activation_history[0].get(), rust_decimal::Decimal::from_str_exact("0.75").unwrap());
    }

    #[test]
    fn test_record_activation_bounds() {
        let mut atom = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "baz".to_string());
        for i in 0..150 {
            let a = Activation::new(rust_decimal::Decimal::from(i % 10) / rust_decimal::Decimal::from(10));
            atom.record_activation(a);
        }
        assert_eq!(atom.activation_history.len(), 100);
    }

    #[test]
    fn test_record_reinforcement() {
        let mut atom = CognitiveAtom::new(AtomId::new(), AtomType::Procedure, "qux".to_string());
        atom.record_reinforcement(42);
        assert_eq!(atom.reinforcement_history, vec![42]);
        assert_eq!(atom.t_last_accessed, 42);
        atom.record_reinforcement(100);
        assert_eq!(atom.reinforcement_history, vec![42, 100]);
        assert_eq!(atom.t_last_accessed, 100);
    }
}
