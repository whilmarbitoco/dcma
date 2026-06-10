use dcma_core::types::*;
use rust_decimal::Decimal;

/// Episodic compression: collapse sequences of repetitive events.
pub fn episodic_compress(
    events: &[AtomId],
    _threshold: Decimal,
) -> Vec<Vec<usize>> {
    if events.is_empty() {
        return Vec::new();
    }
    vec![(0..events.len()).collect()]
}

/// Motif fusion: merge recurring subgraphs into abstraction nodes.
pub fn motif_fuse(
    motifs: &[Vec<AtomId>],
    _threshold: Decimal,
) -> Vec<AtomId> {
    let mut representatives = Vec::new();
    for motif in motifs {
        if !motif.is_empty() {
            representatives.push(motif[0]);
        }
    }
    representatives
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_episodic_compress_empty() {
        let result = episodic_compress(&[], Decimal::ONE);
        assert!(result.is_empty());
    }

    #[test]
    fn test_episodic_compress_single() {
        let a = AtomId::new();
        let result = episodic_compress(&[a], Decimal::ONE);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_motif_fuse() {
        let a = AtomId::new();
        let b = AtomId::new();
        let fused = motif_fuse(&[vec![a, b]], Decimal::ONE);
        assert_eq!(fused.len(), 1);
        assert_eq!(fused[0], a);
    }
}
