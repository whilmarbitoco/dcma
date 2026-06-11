use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};
use uuid::Uuid;

// ============================================================
// Decimal Newtypes
// ============================================================

macro_rules! decimal_newtype {
    ($name:ident, $min:expr, $max:expr, $default:expr, $zero:expr, $one:expr) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub struct $name(Decimal);

        impl $name {
            pub const ZERO: Self = $name(Decimal::ZERO);
            pub const ONE: Self = $name(Decimal::ONE);

            pub fn new(value: Decimal) -> Self {
                Self(value.clamp($min, $max))
            }

            pub fn get(&self) -> Decimal {
                self.0
            }

            pub fn clamp(&self) -> Self {
                Self(self.0.clamp($min, $max))
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self($default)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }

        impl From<Decimal> for $name {
            fn from(value: Decimal) -> Self {
                Self::new(value)
            }
        }

        impl From<$name> for Decimal {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl Add for $name {
            type Output = Self;
            fn add(self, rhs: Self) -> Self::Output {
                Self::new(self.0 + rhs.0)
            }
        }
        impl Sub for $name {
            type Output = Self;
            fn sub(self, rhs: Self) -> Self::Output {
                Self::new(self.0 - rhs.0)
            }
        }
        impl Mul for $name {
            type Output = Self;
            fn mul(self, rhs: Self) -> Self::Output {
                Self::new(self.0 * rhs.0)
            }
        }
        impl Div for $name {
            type Output = Self;
            fn div(self, rhs: Self) -> Self::Output {
                if rhs.0 == Decimal::ZERO {
                    Self::new($min)
                } else {
                    Self::new(self.0 / rhs.0)
                }
            }
        }
        impl AddAssign for $name {
            fn add_assign(&mut self, rhs: Self) {
                *self = *self + rhs;
            }
        }
        impl SubAssign for $name {
            fn sub_assign(&mut self, rhs: Self) {
                *self = *self - rhs;
            }
        }
        impl MulAssign for $name {
            fn mul_assign(&mut self, rhs: Self) {
                *self = *self * rhs;
            }
        }
        impl DivAssign for $name {
            fn div_assign(&mut self, rhs: Self) {
                *self = *self / rhs;
            }
        }
    };
}

// Activation: [0, 1]
decimal_newtype!(
    Activation,
    Decimal::ZERO,
    Decimal::ONE,
    Decimal::ZERO,
    Decimal::ZERO,
    Decimal::ONE
);

// Weight: [0, 1]
decimal_newtype!(
    Weight,
    Decimal::ZERO,
    Decimal::ONE,
    Decimal::from_parts(5, 0, 0, false, 1), // 0.5
    Decimal::ZERO,
    Decimal::ONE
);

// Confidence: [0, 1]
decimal_newtype!(
    Confidence,
    Decimal::ZERO,
    Decimal::ONE,
    Decimal::ZERO,
    Decimal::ZERO,
    Decimal::ONE
);

// Salience: [0, ∞) — no upper bound, just non-negative
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Salience(Decimal);

impl Salience {
    pub const ZERO: Self = Salience(Decimal::ZERO);

    pub fn new(value: Decimal) -> Self {
        Self(value.max(Decimal::ZERO))
    }

    pub fn get(&self) -> Decimal {
        self.0
    }

    pub fn clamp(&self) -> Self {
        Self(self.0.max(Decimal::ZERO))
    }
}

impl Default for Salience {
    fn default() -> Self {
        Self(Decimal::ZERO)
    }
}

impl fmt::Debug for Salience {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Salience({})", self.0)
    }
}

impl From<Decimal> for Salience {
    fn from(value: Decimal) -> Self {
        Self::new(value)
    }
}

impl From<Salience> for Decimal {
    fn from(value: Salience) -> Self {
        value.0
    }
}

impl Add for Salience {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.0 + rhs.0)
    }
}
impl Sub for Salience {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.0 - rhs.0)
    }
}
impl Mul for Salience {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(self.0 * rhs.0)
    }
}
impl Div for Salience {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        if rhs.0 == Decimal::ZERO {
            Self::new(Decimal::ZERO)
        } else {
            Self::new(self.0 / rhs.0)
        }
    }
}

// Pressure: [0, ∞)
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Pressure(Decimal);

impl Pressure {
    pub const ZERO: Self = Pressure(Decimal::ZERO);

    pub fn new(value: Decimal) -> Self {
        Self(value.max(Decimal::ZERO))
    }

    pub fn get(&self) -> Decimal {
        self.0
    }

    pub fn clamp(&self) -> Self {
        Self(self.0.max(Decimal::ZERO))
    }
}

impl Default for Pressure {
    fn default() -> Self {
        Self(Decimal::ZERO)
    }
}

impl fmt::Debug for Pressure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Pressure({})", self.0)
    }
}

impl From<Decimal> for Pressure {
    fn from(value: Decimal) -> Self {
        Self::new(value)
    }
}

impl From<Pressure> for Decimal {
    fn from(value: Pressure) -> Self {
        value.0
    }
}

impl Add for Pressure {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.0 + rhs.0)
    }
}
impl Sub for Pressure {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.0 - rhs.0)
    }
}
impl Mul for Pressure {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(self.0 * rhs.0)
    }
}
impl Div for Pressure {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        if rhs.0 == Decimal::ZERO {
            Self::new(Decimal::ZERO)
        } else {
            Self::new(self.0 / rhs.0)
        }
    }
}

// LearningRate: [0, ∞)
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LearningRate(Decimal);

impl LearningRate {
    pub const ZERO: Self = LearningRate(Decimal::ZERO);

    pub fn new(value: Decimal) -> Self {
        Self(value.max(Decimal::ZERO))
    }

    pub fn get(&self) -> Decimal {
        self.0
    }

    pub fn clamp(&self) -> Self {
        Self(self.0.max(Decimal::ZERO))
    }
}

impl Default for LearningRate {
    fn default() -> Self {
        Self(Decimal::ZERO)
    }
}

impl fmt::Debug for LearningRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LearningRate({})", self.0)
    }
}

impl From<Decimal> for LearningRate {
    fn from(value: Decimal) -> Self {
        Self::new(value)
    }
}

impl From<LearningRate> for Decimal {
    fn from(value: LearningRate) -> Self {
        value.0
    }
}

impl Add for LearningRate {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.0 + rhs.0)
    }
}
impl Sub for LearningRate {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.0 - rhs.0)
    }
}
impl Mul for LearningRate {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(self.0 * rhs.0)
    }
}
impl Div for LearningRate {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        if rhs.0 == Decimal::ZERO {
            Self::new(Decimal::ZERO)
        } else {
            Self::new(self.0 / rhs.0)
        }
    }
}

// ============================================================
// AtomId
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AtomId(pub Uuid);

impl AtomId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for AtomId {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Uuid> for AtomId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl fmt::Display for AtomId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================
// Enums
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AtomType {
    Concept,
    Event,
    Entity,
    Procedure,
    Abstraction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AtomState {
    Active,
    Dormant,
    Frozen,
    Consolidated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelationType {
    // Causal
    Causes,
    Prevents,
    Enables,
    Triggers,
    DerivesFrom,
    // Structural
    PartOf,
    InstanceOf,
    Specializes,
    Generalizes,
    Contains,
    AbstractionOf,
    // Logical
    Implies,
    Negates,
    Supports,
    Contradicts,
    // Temporal
    Precedes,
    Follows,
    CoOccursWith,
    PersistsWith,
    // Cognitive
    Reinforces,
    Inhibits,
    Activates,
    Suppresses,
    Focuses,
}

// ============================================================
// EngineConfig
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    // Activation parameters
    pub eta_d: LearningRate,
    pub eta_r: LearningRate,
    pub eta_c: LearningRate,
    pub eta_q: LearningRate,
    pub lambda: LearningRate,
    pub reinforcement_window: usize,
    pub convergence_threshold: Activation,
    pub max_iterations: usize,
    // Contradiction parameters
    pub eta_p: LearningRate,
    pub eta_pressure_decay: LearningRate,
    pub equilibrium_threshold: Pressure,
    pub equilibrium_rate_threshold: Pressure,
    pub equilibrium_window: usize,
    pub resolution_threshold: Pressure,
    // Consolidation parameters
    pub crystallization_threshold: usize,
    pub stability_threshold: Activation,
    // Entropy parameters
    pub entropy_min: Activation,
    pub entropy_max: Activation,
    pub dead_node_threshold: Activation,
    pub dead_node_window: usize,
    // Spontaneous activation
    pub spontaneous_activation_prob: Activation,
    // Topology parameters
    pub max_motif_size: usize,
    pub max_abstraction_depth: usize,
    // Retrieval parameters
    pub beam_width: usize,
    pub max_retrieval_depth: usize,
    pub max_subgraph_size: usize,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            eta_d: LearningRate::new(Decimal::from_str_exact("0.1").unwrap()),
            eta_r: LearningRate::new(Decimal::from_str_exact("0.05").unwrap()),
            eta_c: LearningRate::new(Decimal::from_str_exact("0.15").unwrap()),
            eta_q: LearningRate::new(Decimal::from_str_exact("0.1").unwrap()),
            lambda: LearningRate::new(Decimal::from_str_exact("0.9").unwrap()),
            reinforcement_window: 10,
            convergence_threshold: Activation::new(Decimal::from_str_exact("0.000001").unwrap()),
            max_iterations: 100,
            // Contradiction defaults (spec Section 5)
            eta_p: LearningRate::new(Decimal::from_str_exact("0.1").unwrap()),
            eta_pressure_decay: LearningRate::new(Decimal::from_str_exact("0.05").unwrap()),
            equilibrium_threshold: Pressure::new(Decimal::from_str_exact("0.05").unwrap()),
            equilibrium_rate_threshold: Pressure::new(Decimal::from_str_exact("0.01").unwrap()),
            equilibrium_window: 100,
            resolution_threshold: Pressure::new(Decimal::from_str_exact("0.95").unwrap()),
            crystallization_threshold: 5,
            stability_threshold: Activation::new(Decimal::from_str_exact("0.7").unwrap()),
            entropy_min: Activation::new(Decimal::from_str_exact("1.5").unwrap()),
            entropy_max: Activation::new(Decimal::from_str_exact("5.0").unwrap()),
            dead_node_threshold: Activation::new(Decimal::from_str_exact("0.000001").unwrap()),
            dead_node_window: 10000,
            spontaneous_activation_prob: Activation::new(Decimal::from_str_exact("0.0001").unwrap()),
            max_motif_size: 5,
            max_abstraction_depth: 5,
            beam_width: 20,
            max_retrieval_depth: 4,
            max_subgraph_size: 500,
        }
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activation_clamp() {
        let a = Activation::new(Decimal::from(2));
        assert_eq!(a.get(), Decimal::ONE);
        let b = Activation::new(Decimal::from(-1));
        assert_eq!(b.get(), Decimal::ZERO);
        let c = Activation::new(Decimal::from_str_exact("0.5").unwrap());
        assert_eq!(c.get(), Decimal::from_str_exact("0.5").unwrap());
    }

    #[test]
    fn test_weight_clamp() {
        let w = Weight::new(Decimal::from(5));
        assert_eq!(w.get(), Decimal::ONE);
        let w2 = Weight::new(Decimal::from(-3));
        assert_eq!(w2.get(), Decimal::ZERO);
    }

    #[test]
    fn test_weight_default_is_half() {
        let w: Weight = Default::default();
        assert_eq!(w.get(), Decimal::from_str_exact("0.5").unwrap());
    }

    #[test]
    fn test_decimal_arithmetic() {
        let a = Activation::new(Decimal::from_str_exact("0.3").unwrap());
        let b = Activation::new(Decimal::from_str_exact("0.4").unwrap());
        let c = a + b;
        assert_eq!(c.get(), Decimal::from_str_exact("0.7").unwrap());
    }

    #[test]
    fn test_engine_config_defaults() {
        let cfg = EngineConfig::default();
        assert_eq!(cfg.eta_d.get(), Decimal::from_str_exact("0.1").unwrap());
        assert_eq!(cfg.eta_r.get(), Decimal::from_str_exact("0.05").unwrap());
        assert_eq!(cfg.max_iterations, 100);
        assert_eq!(cfg.beam_width, 20);
        assert_eq!(cfg.max_retrieval_depth, 4);
    }

    #[test]
    fn test_atom_id_unique() {
        let a = AtomId::new();
        let b = AtomId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn test_relation_type_variants() {
        // Just verify they exist and are distinct
        let causal = RelationType::Causes;
        let structural = RelationType::PartOf;
        let logical = RelationType::Contradicts;
        let temporal = RelationType::Precedes;
        let cognitive = RelationType::Reinforces;
        assert_ne!(causal, structural);
        assert_ne!(logical, temporal);
        assert_ne!(cognitive, causal);
    }

    #[test]
    fn test_atom_state_variants() {
        assert_ne!(AtomState::Active, AtomState::Dormant);
        assert_ne!(AtomState::Frozen, AtomState::Consolidated);
    }

    #[test]
    fn test_salience_non_negative() {
        let s = Salience::new(Decimal::from(-5));
        assert_eq!(s.get(), Decimal::ZERO);
        let s2 = Salience::new(Decimal::from(42));
        assert_eq!(s2.get(), Decimal::from(42));
    }

    #[test]
    fn test_pressure_non_negative() {
        let p = Pressure::new(Decimal::from(-1));
        assert_eq!(p.get(), Decimal::ZERO);
    }

    #[test]
    fn test_learning_rate_non_negative() {
        let lr = LearningRate::new(Decimal::from(-10));
        assert_eq!(lr.get(), Decimal::ZERO);
    }
}
