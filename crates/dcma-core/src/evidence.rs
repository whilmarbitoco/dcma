use crate::types::EvidenceCounter;
use rust_decimal::Decimal;

/// Compute average confidence across a slice of evidence counters.
pub fn average_confidence(counters: &[EvidenceCounter]) -> Decimal {
    if counters.is_empty() {
        return Decimal::ZERO;
    }
    let sum: Decimal = counters.iter().map(|c| c.confidence()).sum();
    sum / Decimal::from(counters.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_average() {
        assert_eq!(average_confidence(&[]), Decimal::ZERO);
    }

    #[test]
    fn test_confidence_formula() {
        let mut c = EvidenceCounter::new(0, 0);
        assert_eq!(c.confidence(), Decimal::ZERO); // 0/(0+0+1) = 0

        c.record_support();
        c.record_oppose();
        // 1/(1+1+1) = 1/3 ≈ 0.333...
        assert_eq!(c.confidence(), Decimal::ONE / Decimal::from(3u64));
    }

    #[test]
    fn test_record_support() {
        let mut c = EvidenceCounter::new(3, 2);
        c.record_support();
        assert_eq!(c.support, 4);
        assert_eq!(c.oppose, 2);
    }

    #[test]
    fn test_record_oppose() {
        let mut c = EvidenceCounter::new(5, 1);
        c.record_oppose();
        assert_eq!(c.support, 5);
        assert_eq!(c.oppose, 2);
    }
}
