use super::types::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub id: u64,
    pub src: AtomId,
    pub dst: AtomId,
    pub relation_type: RelationType,
    pub weight: Weight,
    pub stability: Confidence,
    pub confidence: Confidence,
    pub t_created: u64,
}

impl Relation {
    pub fn new(id: u64, src: AtomId, dst: AtomId, relation_type: RelationType) -> Self {
        Self {
            id,
            src,
            dst,
            relation_type,
            weight: Weight::new(rust_decimal::Decimal::from_str_exact("0.5").unwrap()),
            stability: Confidence::ZERO,
            confidence: Confidence::ZERO,
            t_created: 0,
        }
    }

    /// Edge weight evolution per spec Section 2.3:
    /// w(t+1) = clamp(w(t) + η_r·r(e,t) - η_d·w(t) - η_c·c(e,t))
    pub fn evolve_weight(&mut self, reinforcement: rust_decimal::Decimal, contradiction: rust_decimal::Decimal, config: &EngineConfig) {
        let delta_reinforce = config.eta_r.get() * reinforcement;
        let delta_decay = config.eta_d.get() * self.weight.get();
        let delta_contradiction = config.eta_c.get() * contradiction;
        let new_weight = (self.weight.get() + delta_reinforce - delta_decay - delta_contradiction)
            .clamp(rust_decimal::Decimal::ZERO, rust_decimal::Decimal::ONE);
        self.weight = Weight::new(new_weight);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_relation_weight_is_half() {
        let src = AtomId::new();
        let dst = AtomId::new();
        let rel = Relation::new(1, src, dst, RelationType::Causes);
        assert_eq!(rel.weight.get(), rust_decimal::Decimal::from_str_exact("0.5").unwrap());
        assert_eq!(rel.stability.get(), rust_decimal::Decimal::ZERO);
        assert_eq!(rel.confidence.get(), rust_decimal::Decimal::ZERO);
    }

    #[test]
    fn test_evolve_weight_reinforcement() {
        let src = AtomId::new();
        let dst = AtomId::new();
        let mut rel = Relation::new(1, src, dst, RelationType::Causes);
        let config = EngineConfig::default();

        // Reinforcement only, no contradiction
        rel.evolve_weight(
            rust_decimal::Decimal::ONE,
            rust_decimal::Decimal::ZERO,
            &config,
        );

        // w = 0.5 + 0.05*1.0 - 0.1*0.5 - 0.15*0.0 = 0.5 + 0.05 - 0.05 = 0.5
        assert_eq!(rel.weight.get(), rust_decimal::Decimal::from_str_exact("0.5").unwrap());
    }

    #[test]
    fn test_evolve_weight_decay() {
        let src = AtomId::new();
        let dst = AtomId::new();
        let mut rel = Relation::new(1, src, dst, RelationType::Supports);
        let config = EngineConfig::default();

        // No reinforcement, no contradiction — pure decay
        rel.evolve_weight(
            rust_decimal::Decimal::ZERO,
            rust_decimal::Decimal::ZERO,
            &config,
        );

        // w = 0.5 + 0 - 0.1*0.5 - 0 = 0.45
        assert_eq!(rel.weight.get(), rust_decimal::Decimal::from_str_exact("0.45").unwrap());
    }

    #[test]
    fn test_evolve_weight_contradiction() {
        let src = AtomId::new();
        let dst = AtomId::new();
        let mut rel = Relation::new(1, src, dst, RelationType::Contradicts);
        let config = EngineConfig::default();

        // No reinforcement, contradiction = 1.0
        rel.evolve_weight(
            rust_decimal::Decimal::ZERO,
            rust_decimal::Decimal::ONE,
            &config,
        );

        // w = 0.5 + 0 - 0.1*0.5 - 0.15*1.0 = 0.5 - 0.05 - 0.15 = 0.3
        assert_eq!(rel.weight.get(), rust_decimal::Decimal::from_str_exact("0.3").unwrap());
    }

    #[test]
    fn test_evolve_weight_clamp_upper() {
        let src = AtomId::new();
        let dst = AtomId::new();
        let mut rel = Relation::new(1, src, dst, RelationType::Causes);
        let config = EngineConfig::default();

        // Set weight near max, apply strong reinforcement
        rel.weight = Weight::new(rust_decimal::Decimal::from_str_exact("0.99").unwrap());
        rel.evolve_weight(
            rust_decimal::Decimal::from(10),
            rust_decimal::Decimal::ZERO,
            &config,
        );

        assert!(rel.weight.get() <= rust_decimal::Decimal::ONE);
        assert_eq!(rel.weight.get(), rust_decimal::Decimal::ONE);
    }

    #[test]
    fn test_evolve_weight_clamp_lower() {
        let src = AtomId::new();
        let dst = AtomId::new();
        let mut rel = Relation::new(1, src, dst, RelationType::Contradicts);
        let config = EngineConfig::default();

        // Set weight near zero, apply strong contradiction
        rel.weight = Weight::new(rust_decimal::Decimal::from_str_exact("0.01").unwrap());
        rel.evolve_weight(
            rust_decimal::Decimal::ZERO,
            rust_decimal::Decimal::from(10),
            &config,
        );

        assert!(rel.weight.get() >= rust_decimal::Decimal::ZERO);
        assert_eq!(rel.weight.get(), rust_decimal::Decimal::ZERO);
    }
}
