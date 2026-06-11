use dcma_core::types::Activation;
use dcma_core::types::EngineConfig;
use rust_decimal::Decimal;
/// Convergence: max_i |A_i(t+1) - A_i(t)| < ε
pub fn has_converged(max_diff: Decimal, threshold: Activation) -> bool {
    max_diff < threshold.get()
}

/// Convergence info after activation propagation
#[derive(Debug, Clone)]
pub struct ConvergenceInfo {
    pub converged: bool,
    pub iterations: usize,
    pub final_diff: Decimal,
}

/// Check convergence criterion per spec Theorem 3.2.
/// Returns true if the activation dynamics have reached a fixed point.
pub fn check_convergence(_prev_diff: Decimal, current_diff: Decimal, config: &EngineConfig) -> ConvergenceInfo {
    let threshold = config.convergence_threshold.get();
    let converged = current_diff < threshold;
    
    ConvergenceInfo {
        converged,
        iterations: 0, // caller tracks this
        final_diff: current_diff,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dcma_core::types::EngineConfig;

    #[test]
    fn test_has_converged_true() {
        assert!(has_converged(
            Decimal::from_str_exact("0.0000001").unwrap(),
            Activation::new(Decimal::from_str_exact("0.000001").unwrap())
        ));
    }

    #[test]
    fn test_has_converged_false() {
        assert!(!has_converged(
            Decimal::from_str_exact("0.001").unwrap(),
            Activation::new(Decimal::from_str_exact("0.000001").unwrap())
        ));
    }

    #[test]
    fn test_convergence_default_threshold() {
        let config = EngineConfig::default();
        let threshold = config.convergence_threshold.get();
        assert_eq!(threshold, Decimal::from_str_exact("0.000001").unwrap());
    }
}
