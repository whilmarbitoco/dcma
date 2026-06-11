use dcma_core::atom::CognitiveAtom;
use dcma_core::state::CognitiveState;
use dcma_core::types::*;
use rust_decimal::Decimal;
use std::collections::HashSet;

pub fn merge_edge_weights(w1: Decimal, w2: Decimal) -> Decimal {
    w1.max(w2)
}

pub fn merge_atoms(atom1: &CognitiveAtom, atom2: &CognitiveAtom) -> CognitiveAtom {
    let mut merged = atom1.clone();
    for (key, value) in &atom2.attributes {
        merged.attributes.insert(key.clone(), value.clone());
    }
    let conf1 = atom1.confidence.get();
    let conf2 = atom2.confidence.get();
    merged.confidence = Confidence::new(conf1.max(conf2));
    let sal1 = atom1.salience.get();
    let sal2 = atom2.salience.get();
    merged.salience = Salience::new(sal1.max(sal2));
    merged.t_created = atom1.t_created.min(atom2.t_created);
    merged.t_last_accessed = atom1.t_last_accessed.max(atom2.t_last_accessed);
    merged.state = if atom1.state == AtomState::Active || atom2.state == AtomState::Active {
        AtomState::Active
    } else {
        atom1.state
    };
    merged
}

pub fn merge_states(state1: &CognitiveState, state2: &CognitiveState) -> CognitiveState {
    let config = state1.config.clone();
    let mut merged = CognitiveState::new(config);
    let mut seen_atoms: HashSet<AtomId> = HashSet::new();
    let mut seen_edges: HashSet<(AtomId, AtomId, RelationType)> = HashSet::new();

    for (_, atom) in state1.graph.atoms() {
        merged.graph.add_atom(atom.clone());
        seen_atoms.insert(atom.id);
    }
    for (_, atom) in state2.graph.atoms() {
        if seen_atoms.contains(&atom.id) {
            if let Some(mut existing) = merged.graph.get_atom(atom.id).cloned() {
                let merged_atom = merge_atoms(&existing, atom);
                // Remove old and re-add to update in place
                merged.graph.remove_atom(atom.id);
                merged.graph.add_atom(merged_atom);
            }
        } else {
            merged.graph.add_atom(atom.clone());
            seen_atoms.insert(atom.id);
        }
    }

    for (_, rel) in state1.graph.relations() {
        let key = (rel.src, rel.dst, rel.relation_type);
        if !seen_edges.contains(&key) {
            if let Ok(_) = merged.graph.add_relation_with_weight(rel.src, rel.dst, rel.relation_type, rel.weight) {
                seen_edges.insert(key);
            }
        }
    }
    for (_, rel) in state2.graph.relations() {
        let key = (rel.src, rel.dst, rel.relation_type);
        if seen_edges.contains(&key) {
            if let Some(existing_rel) = merged.graph.relations_between(rel.src, rel.dst).iter().find(|(_, r)| r.relation_type == rel.relation_type) {
                let existing_weight = existing_rel.1.weight.get();
                let new_weight = merge_edge_weights(existing_weight, rel.weight.get());
                if let Ok(()) = merged.graph.update_edge_weight(existing_rel.0, Weight::new(new_weight)) {
                }
            }
        } else {
            if let Ok(_) = merged.graph.add_relation_with_weight(rel.src, rel.dst, rel.relation_type, rel.weight) {
                seen_edges.insert(key);
            }
        }
    }

    for ((a, b), p) in &state1.contradictions {
        merged.contradictions.insert((*a, *b), *p);
    }
    for ((a, b), p) in &state2.contradictions {
        let existing = merged.contradictions.get(&(*a, *b)).copied().unwrap_or(Pressure::ZERO);
        merged.contradictions.insert((*a, *b), Pressure::new(existing.get().max(p.get())));
    }

    let tick1 = state1.tick;
    let tick2 = state2.tick;
    merged.tick = tick1.max(tick2);

    let mut history: Vec<u64> = state1.history.clone();
    history.extend(&state2.history);
    history.sort();
    history.dedup();
    merged.history = history;

    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use dcma_core::atom::CognitiveAtom;
    use dcma_core::types::*;
    use rust_decimal::Decimal;
    use std::collections::HashMap;

    fn make_atom(name: &str) -> CognitiveAtom {
        CognitiveAtom::new(AtomId::new(), AtomType::Concept, name.to_string())
    }

    #[test]
    fn test_merge_edge_weights() {
        let w1 = Decimal::from_str_exact("0.3").unwrap();
        let w2 = Decimal::from_str_exact("0.7").unwrap();
        assert_eq!(merge_edge_weights(w1, w2), Decimal::from_str_exact("0.7").unwrap());
    }

    #[test]
    fn test_merge_edge_weights_commutative() {
        let w1 = Decimal::from_str_exact("0.3").unwrap();
        let w2 = Decimal::from_str_exact("0.7").unwrap();
        assert_eq!(merge_edge_weights(w1, w2), merge_edge_weights(w2, w1));
    }

    #[test]
    fn test_merge_edge_weights_idempotent() {
        let w = Decimal::from_str_exact("0.5").unwrap();
        assert_eq!(merge_edge_weights(w, w), w);
    }

    #[test]
    fn test_merge_atoms() {
        let mut atom1 = make_atom("test");
        let mut atom2 = make_atom("test");
        atom1.confidence = Confidence::new(Decimal::from_str_exact("0.5").unwrap());
        atom2.confidence = Confidence::new(Decimal::from_str_exact("0.8").unwrap());
        atom1.salience = Salience::new(Decimal::from_str_exact("0.3").unwrap());
        atom2.salience = Salience::new(Decimal::from_str_exact("0.6").unwrap());
        atom1.t_created = 100;
        atom2.t_created = 200;
        atom1.t_last_accessed = 50;
        atom2.t_last_accessed = 300;
        atom1.attributes.insert("color".to_string(), "red".to_string());
        atom2.attributes.insert("size".to_string(), "big".to_string());

        let merged = merge_atoms(&atom1, &atom2);

        assert_eq!(merged.confidence.get(), Decimal::from_str_exact("0.8").unwrap());
        assert_eq!(merged.salience.get(), Decimal::from_str_exact("0.6").unwrap());
        assert_eq!(merged.t_created, 100);
        assert_eq!(merged.t_last_accessed, 300);
        assert_eq!(merged.attributes.get("color").unwrap(), "red");
        assert_eq!(merged.attributes.get("size").unwrap(), "big");
    }

    #[test]
    fn test_merge_states() {
        let config = EngineConfig::default();
        let mut state1 = CognitiveState::new(config.clone());
        let mut state2 = CognitiveState::new(config);

        let atom1 = make_atom("a");
        let id1 = atom1.id;
        let atom2 = make_atom("b");
        let id2 = atom2.id;
        let atom3 = make_atom("c");
        let id3 = atom3.id;

        state1.graph.add_atom(atom1);
        state1.graph.add_atom(atom2);
        state2.graph.add_atom(atom3);

        state1.graph.add_relation(id1, id2, RelationType::Causes).unwrap();

        let merged = merge_states(&state1, &state2);
        assert_eq!(merged.graph.atom_count(), 3);
        assert_eq!(merged.graph.relation_count(), 1);
    }

    #[test]
    fn test_merge_commutative() {
        let config = EngineConfig::default();
        let mut state_a = CognitiveState::new(config.clone());
        let mut state_b = CognitiveState::new(config);

        let atom1 = make_atom("x");
        let id1 = atom1.id;
        let atom2 = make_atom("y");
        let id2 = atom2.id;
        state_a.graph.add_atom(atom1.clone());
        state_a.graph.add_atom(atom2.clone());
        state_b.graph.add_atom(atom1);
        state_b.graph.add_atom(atom2);
        state_a.graph.add_relation(id1, id2, RelationType::Supports).unwrap();
        state_b.graph.add_relation(id1, id2, RelationType::Supports).unwrap();

        let merged_ab = merge_states(&state_a, &state_b);
        let merged_ba = merge_states(&state_b, &state_a);

        assert_eq!(merged_ab.graph.atom_count(), merged_ba.graph.atom_count());
        assert_eq!(merged_ab.graph.relation_count(), merged_ba.graph.relation_count());
    }

    #[test]
    fn test_merge_associative() {
        let config = EngineConfig::default();
        let mut state_a = CognitiveState::new(config.clone());
        let mut state_b = CognitiveState::new(config.clone());
        let mut state_c = CognitiveState::new(config);

        let a1 = make_atom("1");
        let a2 = make_atom("2");
        let a3 = make_atom("3");
        let id1 = a1.id;
        let id2 = a2.id;
        let id3 = a3.id;

        state_a.graph.add_atom(a1);
        state_a.graph.add_atom(a2.clone());
        state_b.graph.add_atom(a2.clone());
        state_b.graph.add_atom(a3.clone());
        state_c.graph.add_atom(a2);
        state_c.graph.add_atom(a3);

        state_a.graph.add_relation(id1, id2, RelationType::Causes).unwrap();
        state_b.graph.add_relation(id2, id3, RelationType::Supports).unwrap();

        let ab = merge_states(&state_a, &state_b);
        let ab_c = merge_states(&ab, &state_c);

        let bc = merge_states(&state_b, &state_c);
        let a_bc = merge_states(&state_a, &bc);

        assert_eq!(ab_c.graph.atom_count(), a_bc.graph.atom_count());
        assert_eq!(ab_c.graph.relation_count(), a_bc.graph.relation_count());
    }
}
