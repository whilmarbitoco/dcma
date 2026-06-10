use super::graph::CognitiveGraph;
use super::types::*;
use petgraph::Direction;
use unicode_normalization::UnicodeNormalization;

/// Canonicalize a raw symbol string per the pipeline:
/// 1. Lowercase
/// 2. Unicode normalization (NFC)
/// 3. Trim whitespace
/// 4. Collapse internal whitespace to single spaces
pub fn canonicalize(raw: &str) -> String {
    let s = raw.to_lowercase();
    let s = s.nfc().collect::<String>();
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Identity merge (spec Section 8.2):
/// Merge source atom into destination atom.
/// - Edges from source are moved to dest (max weight if duplicate)
/// - Salience: max of both
/// - Reinforcement histories: merged
/// - Source is marked DORMANT (never deleted — Law 4)
pub fn merge_identifiers(
    graph: &mut CognitiveGraph,
    source_id: AtomId,
    dest_id: AtomId,
) -> Result<(), crate::graph::GraphError> {
    if graph.node_index(source_id).is_none() {
        return Err(crate::graph::GraphError::NodeNotFound(source_id));
    }
    if graph.node_index(dest_id).is_none() {
        return Err(crate::graph::GraphError::NodeNotFound(dest_id));
    }

    // 1. Inbound edges of source: (u, source)
    let inbound: Vec<_> = graph.edges_of(source_id)
        .into_iter()
        .filter(|(_, dir, _)| *dir == Direction::Incoming)
        .map(|(edge_idx, _, rel)| {
            let src_id = rel.src;
            let rel_type = rel.relation_type;
            let weight = rel.weight;
            (edge_idx, src_id, rel_type, weight)
        })
        .collect();

    // We need to collect edge removals separately since we can't modify while iterating
    let mut edges_to_remove = Vec::new();
    let mut edges_to_add = Vec::new();

    for (edge_idx, src_id, rel_type, weight) in inbound {
        edges_to_remove.push(edge_idx);
        // Check if (src, dest) already exists
        let existing = graph.relations_between(src_id, dest_id)
            .into_iter()
            .find(|(_, rel)| rel.relation_type == rel_type);

        if let Some((existing_edge, existing_rel)) = existing {
            // Take max weight
            if existing_rel.weight.get() < weight.get() {
                edges_to_remove.push(existing_edge);
                edges_to_add.push((src_id, dest_id, rel_type, weight));
            }
        } else {
            edges_to_add.push((src_id, dest_id, rel_type, weight));
        }
    }

    // 2. Outbound edges of source: (source, u)
    let outbound: Vec<_> = graph.edges_of(source_id)
        .into_iter()
        .filter(|(_, dir, _)| *dir == Direction::Outgoing)
        .map(|(edge_idx, _, rel)| {
            let dst_id = rel.dst;
            let rel_type = rel.relation_type;
            let weight = rel.weight;
            (edge_idx, dst_id, rel_type, weight)
        })
        .collect();

    for (edge_idx, dst_id, rel_type, weight) in outbound {
        edges_to_remove.push(edge_idx);
        // Check if (dest, dst) already exists
        let existing = graph.relations_between(dest_id, dst_id)
            .into_iter()
            .find(|(_, rel)| rel.relation_type == rel_type);

        if let Some((existing_edge, existing_rel)) = existing {
            if existing_rel.weight.get() < weight.get() {
                edges_to_remove.push(existing_edge);
                edges_to_add.push((dest_id, dst_id, rel_type, weight));
            }
        } else {
            edges_to_add.push((dest_id, dst_id, rel_type, weight));
        }
    }

    // Remove old edges
    for edge_idx in edges_to_remove {
        graph.graph.remove_edge(edge_idx);
    }

    // Add new edges
    for (src, dst, rel_type, weight) in edges_to_add {
        let id = graph.next_relation_id;
        graph.next_relation_id += 1;
        let mut rel = super::relation::Relation::new(id, src, dst, rel_type);
        rel.weight = weight;
        let src_idx = graph.id_map[&src];
        let dst_idx = graph.id_map[&dst];
        graph.graph.add_edge(src_idx, dst_idx, rel);
    }

    // Merge salience: dest.salience = max(dest.salience, source.salience)
    let source_salience = graph.get_atom(source_id).map(|a| a.salience).unwrap_or(Salience::ZERO);
    let source_reinforcement = graph.get_atom(source_id).map(|a| a.reinforcement_history.clone()).unwrap_or_default();
    if let Some(dest_atom) = graph.get_atom_mut(dest_id) {
        if dest_atom.salience.get() < source_salience.get() {
            dest_atom.salience = source_salience;
        }
        // Merge reinforcement histories
        dest_atom.reinforcement_history.extend(source_reinforcement);
        dest_atom.reinforcement_history.sort_unstable();
        dest_atom.reinforcement_history.dedup();
    }

    // Mark source as DORMANT (preserve — Law 4)
    if let Some(source_atom) = graph.get_atom_mut(source_id) {
        source_atom.mark_dormant();
    }

    Ok(())
}

/// Check if two symbols are approximate matches (edit distance threshold)
pub fn is_approximate_match(a: &str, b: &str, threshold: usize) -> bool {
    levenshtein_distance(a, b) <= threshold
}

/// Levenshtein distance using standard DP
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let n = a_chars.len();
    let m = b_chars.len();

    if n == 0 {
        return m;
    }
    if m == 0 {
        return n;
    }

    let mut prev: Vec<usize> = (0..=m).collect();
    let mut curr = vec![0usize; m + 1];

    for i in 1..=n {
        curr[0] = i;
        for j in 1..=m {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            curr[j] = (curr[j - 1] + 1)
                .min(prev[j] + 1)
                .min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[m]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atom::CognitiveAtom;
    use rust_decimal::Decimal;

    #[test]
    fn test_canonicalize_lowercase() {
        assert_eq!(canonicalize("Hello World"), "hello world");
    }

    #[test]
    fn test_canonicalize_unicode() {
        // é composed (U+00E9) vs e + combining accent (U+0065 U+0301)
        let composed = "caf\u{00E9}";
        let decomposed = "cafe\u{0301}";
        assert_eq!(canonicalize(composed), canonicalize(decomposed));
    }

    #[test]
    fn test_canonicalize_whitespace() {
        assert_eq!(canonicalize("  foo   bar  "), "foo bar");
        assert_eq!(canonicalize("single"), "single");
        assert_eq!(canonicalize(""), "");
    }

    #[test]
    fn test_merge_identifiers() {
        let mut g = CognitiveGraph::new();

        // Create 3 atoms: a -> source -> b, and a -> dest
        let a = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "a".to_string());
        let source = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "source".to_string());
        let dest = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "dest".to_string());
        let b = CognitiveAtom::new(AtomId::new(), AtomType::Concept, "b".to_string());

        let id_a = a.id;
        let id_source = source.id;
        let id_dest = dest.id;
        let id_b = b.id;

        g.add_atom(a);
        g.add_atom(source);
        g.add_atom(dest);
        g.add_atom(b);

        // a -> source (Causes, w=0.7)
        g.add_relation_with_weight(id_a, id_source, RelationType::Causes, Weight::new(Decimal::from_str_exact("0.7").unwrap())).unwrap();
        // source -> b (Causes, w=0.6)
        g.add_relation_with_weight(id_source, id_b, RelationType::Causes, Weight::new(Decimal::from_str_exact("0.6").unwrap())).unwrap();
        // a -> dest (Supports, w=0.3)
        g.add_relation_with_weight(id_a, id_dest, RelationType::Supports, Weight::new(Decimal::from_str_exact("0.3").unwrap())).unwrap();

        // Set salience on source > dest
        g.get_atom_mut(id_source).unwrap().salience = Salience::new(Decimal::from(10));
        g.get_atom_mut(id_dest).unwrap().salience = Salience::new(Decimal::from(5));

        // Record reinforcement on source
        g.get_atom_mut(id_source).unwrap().record_reinforcement(42);

        // Merge source into dest
        merge_identifiers(&mut g, id_source, id_dest).unwrap();

        // Source should be DORMANT, not deleted
        assert_eq!(g.get_atom(id_source).unwrap().state, AtomState::Dormant);
        assert!(g.get_atom(id_source).is_some()); // Law 4: preserved

        // Dest should now have inbound edges from a (Causes AND Supports)
        let dest_inbound = g.inbound_neighbors(id_dest);
        let rel_types: Vec<_> = dest_inbound.iter().map(|(_, _, rel)| rel.relation_type).collect();
        assert!(rel_types.contains(&RelationType::Causes));
        assert!(rel_types.contains(&RelationType::Supports));

        // Dest should have outbound edge to b
        let dest_outbound = g.outbound_neighbors(id_dest);
        assert_eq!(dest_outbound.len(), 1);
        assert_eq!(dest_outbound[0].1.id, id_b);

        // Dest salience should be max(5, 10) = 10
        assert_eq!(g.get_atom(id_dest).unwrap().salience.get(), Decimal::from(10));

        // Dest should have reinforcement history from source
        assert!(g.get_atom(id_dest).unwrap().reinforcement_history.contains(&42));
    }

    #[test]
    fn test_approximate_match() {
        assert!(is_approximate_match("hello", "hello", 0));
        assert!(is_approximate_match("hello", "hallo", 1));
        assert!(is_approximate_match("kitten", "sitting", 3));
        assert!(!is_approximate_match("kitten", "sitting", 2));
        assert!(is_approximate_match("", "", 0));
        assert!(is_approximate_match("a", "", 1));
        assert!(!is_approximate_match("abc", "xyz", 2));
    }
}
