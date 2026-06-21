use dcma_core::types::*;
use rust_decimal::Decimal;
use std::collections::HashMap;

/// Episodic compression: collapse consecutive identical events into groups.
///
/// Given a sequence of atom references, groups consecutive occurrences of the
/// same atom together when they repeat. Returns groups of indices into `events`
/// where each group is a consecutive run of the same atom.
pub fn episodic_compress(
    events: &[AtomId],
    _threshold: Decimal,
) -> Vec<Vec<usize>> {
    if events.is_empty() {
        return Vec::new();
    }

    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut current_run: Vec<usize> = vec![0];

    for i in 1..events.len() {
        if events[i] == events[i - 1] {
            current_run.push(i);
        } else {
            groups.push(current_run);
            current_run = vec![i];
        }
    }
    groups.push(current_run);
    groups
}

/// Motif fusion: merge motifs that share significant atom overlap.
///
/// For motifs whose atom overlap exceeds `threshold`, the most central atom
/// (appearing across the most motifs) is returned as the fused representative.
/// Returns one representative atom per input motif group.
pub fn motif_fuse(
    motifs: &[Vec<AtomId>],
    threshold: Decimal,
) -> Vec<AtomId> {
    if motifs.is_empty() {
        return Vec::new();
    }

    // Count atom frequency across all motifs (centrality measure)
    let mut atom_freq: HashMap<AtomId, usize> = HashMap::new();
    for motif in motifs {
        for atom in motif {
            *atom_freq.entry(*atom).or_default() += 1;
        }
    }

    // Build overlap graph: edges between motifs sharing >= threshold atoms
    let n = motifs.len();
    let mut merged: Vec<Option<usize>> = (0..n).map(Some).collect();

    for i in 0..n {
        for j in (i + 1)..n {
            let intersection: usize = motifs[i]
                .iter()
                .filter(|a| motifs[j].contains(a))
                .count();
            let min_size = motifs[i].len().min(motifs[j].len()).max(1);
            let overlap = Decimal::from(intersection) / Decimal::from(min_size);
            if overlap >= threshold {
                // Merge j into i
                merged[j] = None;
            }
        }
    }

    // For each merged group, pick the most central atom
    let mut representatives = Vec::new();
    for i in 0..n {
        if merged[i].is_some() {
            let rep = motifs[i]
                .iter()
                .max_by_key(|a| atom_freq.get(a).copied().unwrap_or(0))
                .copied()
                .unwrap_or_else(|| {
                    motifs[i].first().copied().unwrap_or_else(AtomId::new)
                });
            representatives.push(rep);
        }
    }

    representatives
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_episodic_compress_empty() {
        let result = episodic_compress(&[], Decimal::ONE);
        assert!(result.is_empty());
    }

    #[test]
    fn test_episodic_compress_no_repeats() {
        let a = AtomId::new();
        let b = AtomId::new();
        let c = AtomId::new();
        let result = episodic_compress(&[a, b, c], Decimal::ONE);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], vec![0]);
        assert_eq!(result[1], vec![1]);
        assert_eq!(result[2], vec![2]);
    }

    #[test]
    fn test_episodic_compress_with_repeats() {
        let a = AtomId::new();
        let b = AtomId::new();
        let result = episodic_compress(&[a, a, a, b, b, a], Decimal::ONE);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], vec![0, 1, 2]);
        assert_eq!(result[1], vec![3, 4]);
        assert_eq!(result[2], vec![5]);
    }

    #[test]
    fn test_motif_fuse() {
        let a = AtomId::new();
        let b = AtomId::new();
        let fused = motif_fuse(&[vec![a, b]], Decimal::ONE);
        assert_eq!(fused.len(), 1);
        assert!(fused[0] == a || fused[0] == b);
    }

    #[test]
    fn test_motif_fuse_overlap() {
        let a = AtomId::new();
        let b = AtomId::new();
        let c = AtomId::new();
        // Two motifs sharing atom 'a' — should fuse with threshold >= 0.5
        let fused = motif_fuse(&[vec![a, b], vec![a, c]], Decimal::from_str_exact("0.5").unwrap());
        assert_eq!(fused.len(), 1);
        assert_eq!(fused[0], a); // 'a' appears in both motifs
    }

    #[test]
    fn test_motif_fuse_no_overlap() {
        let a = AtomId::new();
        let b = AtomId::new();
        let c = AtomId::new();
        let d = AtomId::new();
        let fused = motif_fuse(&[vec![a, b], vec![c, d]], Decimal::ONE);
        assert_eq!(fused.len(), 2);
    }
}
