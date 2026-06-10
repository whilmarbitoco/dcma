use super::atom::CognitiveAtom;
use super::relation::Relation;
use super::types::*;
use petgraph::stable_graph::{EdgeIndex, NodeIndex, StableGraph};
use petgraph::visit::{EdgeRef, VisitMap, Visitable};
use petgraph::Direction;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use rust_decimal::Decimal;

#[derive(Debug, thiserror::Error)]
pub enum GraphError {
    #[error("Source atom not found: {0:?}")]
    SourceNotFound(AtomId),
    #[error("Destination atom not found: {0:?}")]
    DestNotFound(AtomId),
    #[error("Edge not found")]
    EdgeNotFound,
    #[error("Node not found: {0:?}")]
    NodeNotFound(AtomId),
}

#[derive(Debug, Clone)]
pub struct CognitiveGraph {
    pub(crate) graph: StableGraph<CognitiveAtom, Relation>,
    pub(crate) id_map: HashMap<AtomId, NodeIndex>,
    pub(crate) next_relation_id: u64,
}

impl Serialize for CognitiveGraph {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("CognitiveGraph", 3)?;
        // Serialize atoms as a vec
        let atoms: Vec<&CognitiveAtom> = self.graph.node_indices()
            .filter_map(|idx| self.graph.node_weight(idx))
            .collect();
        state.serialize_field("atoms", &atoms)?;
        // Serialize relations as a vec (with src/dst AtomIds)
        let relations: Vec<&Relation> = self.graph.edge_indices()
            .filter_map(|idx| self.graph.edge_weight(idx))
            .collect();
        state.serialize_field("relations", &relations)?;
        state.serialize_field("next_relation_id", &self.next_relation_id)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for CognitiveGraph {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct GraphData {
            atoms: Vec<CognitiveAtom>,
            relations: Vec<Relation>,
            next_relation_id: u64,
        }
        let data = GraphData::deserialize(deserializer)?;
        let mut g = CognitiveGraph::new();
        for atom in data.atoms {
            g.add_atom(atom);
        }
        // Re-add relations — need to look up NodeIndex from AtomId
        for rel in data.relations {
            let src_idx = g.id_map.get(&rel.src).copied()
                .ok_or_else(|| serde::de::Error::custom(format!("Missing src atom {:?}", rel.src)))?;
            let dst_idx = g.id_map.get(&rel.dst).copied()
                .ok_or_else(|| serde::de::Error::custom(format!("Missing dst atom {:?}", rel.dst)))?;
            let id = g.next_relation_id;
            g.next_relation_id += 1;
            let new_rel = Relation {
                id,
                src: rel.src,
                dst: rel.dst,
                relation_type: rel.relation_type,
                weight: rel.weight,
                stability: rel.stability,
                confidence: rel.confidence,
                t_created: rel.t_created,
            };
            g.graph.add_edge(src_idx, dst_idx, new_rel);
        }
        g.next_relation_id = data.next_relation_id;
        Ok(g)
    }
}

impl CognitiveGraph {
    pub fn new() -> Self {
        Self {
            graph: StableGraph::new(),
            id_map: HashMap::new(),
            next_relation_id: 0,
        }
    }

    pub fn add_atom(&mut self, atom: CognitiveAtom) -> NodeIndex {
        let id = atom.id;
        let idx = self.graph.add_node(atom);
        self.id_map.insert(id, idx);
        idx
    }

    pub fn remove_atom(&mut self, id: AtomId) -> Option<CognitiveAtom> {
        let idx = self.id_map.remove(&id)?;
        self.graph.remove_node(idx)
    }

    pub fn get_atom(&self, id: AtomId) -> Option<&CognitiveAtom> {
        let idx = self.id_map.get(&id)?;
        self.graph.node_weight(*idx)
    }

    pub fn get_atom_mut(&mut self, id: AtomId) -> Option<&mut CognitiveAtom> {
        let idx = self.id_map.get(&id)?;
        self.graph.node_weight_mut(*idx)
    }

    pub fn add_relation(
        &mut self,
        src: AtomId,
        dst: AtomId,
        rel_type: RelationType,
    ) -> Result<EdgeIndex, GraphError> {
        self.add_relation_with_weight(
            src,
            dst,
            rel_type,
            Weight::new(Decimal::from_str_exact("0.5").unwrap()),
        )
    }

    pub fn add_relation_with_weight(
        &mut self,
        src: AtomId,
        dst: AtomId,
        rel_type: RelationType,
        weight: Weight,
    ) -> Result<EdgeIndex, GraphError> {
        let src_idx = *self.id_map.get(&src).ok_or(GraphError::SourceNotFound(src))?;
        let dst_idx = *self.id_map.get(&dst).ok_or(GraphError::DestNotFound(dst))?;
        let id = self.next_relation_id;
        self.next_relation_id += 1;
        let mut rel = Relation::new(id, src, dst, rel_type);
        rel.weight = weight;
        let edge = self.graph.add_edge(src_idx, dst_idx, rel);
        Ok(edge)
    }

    pub fn get_relation(&self, edge: EdgeIndex) -> Option<&Relation> {
        self.graph.edge_weight(edge)
    }

    pub fn get_relation_mut(&mut self, edge: EdgeIndex) -> Option<&mut Relation> {
        self.graph.edge_weight_mut(edge)
    }

    pub fn remove_relation(&mut self, src: AtomId, dst: AtomId, rel_type: RelationType) -> bool {
        let src_idx = match self.id_map.get(&src) {
            Some(idx) => *idx,
            None => return false,
        };
        let dst_idx = match self.id_map.get(&dst) {
            Some(idx) => *idx,
            None => return false,
        };
        if let Some(edge) = self.graph.find_edge(src_idx, dst_idx) {
            if let Some(rel) = self.graph.edge_weight(edge) {
                if rel.relation_type == rel_type {
                    self.graph.remove_edge(edge);
                    return true;
                }
            }
        }
        false
    }

    pub fn relations_between(&self, src: AtomId, dst: AtomId) -> Vec<(EdgeIndex, &Relation)> {
        let src_idx = match self.id_map.get(&src) {
            Some(idx) => *idx,
            None => return Vec::new(),
        };
        let dst_idx = match self.id_map.get(&dst) {
            Some(idx) => *idx,
            None => return Vec::new(),
        };
        self.graph
            .edges_connecting(src_idx, dst_idx)
            .map(|e| (e.id(), e.weight()))
            .collect()
    }

    pub fn atoms(&self) -> impl Iterator<Item = (NodeIndex, &CognitiveAtom)> {
        self.graph.node_indices().filter_map(move |idx| {
            self.graph.node_weight(idx).map(|atom| (idx, atom))
        })
    }

    pub fn atom_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn relations(&self) -> impl Iterator<Item = (EdgeIndex, &Relation)> {
        self.graph.edge_indices().filter_map(move |idx| {
            self.graph.edge_weight(idx).map(|rel| (idx, rel))
        })
    }

    pub fn relation_count(&self) -> usize {
        self.graph.edge_count()
    }

    pub fn node_indices(&self) -> Vec<NodeIndex> {
        self.graph.node_indices().collect()
    }

    pub fn node_index(&self, id: AtomId) -> Option<NodeIndex> {
        self.id_map.get(&id).copied()
    }

    pub fn inbound_neighbors(&self, id: AtomId) -> Vec<(NodeIndex, &CognitiveAtom, &Relation)> {
        let idx = match self.id_map.get(&id) {
            Some(i) => *i,
            None => return Vec::new(),
        };
        self.graph
            .edges_directed(idx, Direction::Incoming)
            .filter_map(|edge| {
                let src = edge.source();
                let atom = self.graph.node_weight(src)?;
                let rel = edge.weight();
                Some((src, atom, rel))
            })
            .collect()
    }

    pub fn outbound_neighbors(&self, id: AtomId) -> Vec<(NodeIndex, &CognitiveAtom, &Relation)> {
        let idx = match self.id_map.get(&id) {
            Some(i) => *i,
            None => return Vec::new(),
        };
        self.graph
            .edges_directed(idx, Direction::Outgoing)
            .filter_map(|edge| {
                let dst = edge.target();
                let atom = self.graph.node_weight(dst)?;
                let rel = edge.weight();
                Some((dst, atom, rel))
            })
            .collect()
    }

    pub fn neighbors(&self, id: AtomId) -> Vec<(NodeIndex, &CognitiveAtom)> {
        let idx = match self.id_map.get(&id) {
            Some(i) => *i,
            None => return Vec::new(),
        };
        self.graph
            .neighbors(idx)
            .filter_map(|n| {
                let atom = self.graph.node_weight(n)?;
                Some((n, atom))
            })
            .collect()
    }

    pub fn k_hop_neighbors(&self, id: AtomId, k: usize) -> HashMap<NodeIndex, usize> {
        let start = match self.id_map.get(&id) {
            Some(i) => *i,
            None => return HashMap::new(),
        };
        let mut visited = HashMap::new();
        visited.insert(start, 0usize);
        let mut queue = VecDeque::new();
        queue.push_back((start, 0usize));

        while let Some((current, dist)) = queue.pop_front() {
            if dist >= k {
                continue;
            }
            for neighbor in self.graph.neighbors(current) {
                if !visited.contains_key(&neighbor) {
                    visited.insert(neighbor, dist + 1);
                    queue.push_back((neighbor, dist + 1));
                }
            }
        }
        visited
    }

    pub fn degree(&self, id: AtomId) -> usize {
        let idx = match self.id_map.get(&id) {
            Some(i) => *i,
            None => return 0,
        };
        self.graph.edges(idx).count()
    }

    pub fn in_degree(&self, id: AtomId) -> usize {
        let idx = match self.id_map.get(&id) {
            Some(i) => *i,
            None => return 0,
        };
        self.graph.edges_directed(idx, Direction::Incoming).count()
    }

    pub fn out_degree(&self, id: AtomId) -> usize {
        let idx = match self.id_map.get(&id) {
            Some(i) => *i,
            None => return 0,
        };
        self.graph.edges_directed(idx, Direction::Outgoing).count()
    }

    pub fn degree_distribution(&self) -> HashMap<usize, usize> {
        let mut dist = HashMap::new();
        for idx in self.graph.node_indices() {
            let d = self.graph.edges(idx).count();
            *dist.entry(d).or_insert(0) += 1;
        }
        dist
    }

    pub fn diameter(&self) -> Option<usize> {
        if self.graph.node_count() >= 1000 {
            return None;
        }
        let mut max_dist = 0;
        for start in self.graph.node_indices() {
            let mut visited = self.graph.visit_map();
            let mut queue = VecDeque::new();
            visited.visit(start);
            queue.push_back((start, 0usize));
            while let Some((current, dist)) = queue.pop_front() {
                max_dist = max_dist.max(dist);
                for neighbor in self.graph.neighbors(current) {
                    if !visited.is_visited(&neighbor) {
                        visited.visit(neighbor);
                        queue.push_back((neighbor, dist + 1));
                    }
                }
            }
        }
        Some(max_dist)
    }

    pub fn edge_weight(&self, edge: EdgeIndex) -> Option<Decimal> {
        self.graph.edge_weight(edge).map(|r| r.weight.get())
    }

    pub fn update_edge_weight(&mut self, edge: EdgeIndex, weight: Weight) -> Result<(), GraphError> {
        let rel = self.graph.edge_weight_mut(edge).ok_or(GraphError::EdgeNotFound)?;
        rel.weight = weight;
        Ok(())
    }

    pub fn subgraph(&self, nodes: &[NodeIndex]) -> CognitiveGraph {
        let node_set: HashSet<NodeIndex> = nodes.iter().copied().collect();
        let mut sub = CognitiveGraph::new();

        for &idx in nodes {
            if let Some(atom) = self.graph.node_weight(idx) {
                sub.add_atom(atom.clone());
            }
        }

        for edge in self.graph.edge_indices() {
            let endpoints = self.graph.edge_endpoints(edge);
            if let Some((src, dst)) = endpoints {
                if node_set.contains(&src) && node_set.contains(&dst) {
                    if let Some(rel) = self.graph.edge_weight(edge) {
                        let new_id = sub.next_relation_id;
                        sub.next_relation_id += 1;
                        let new_rel = Relation {
                            id: new_id,
                            src: rel.src,
                            dst: rel.dst,
                            relation_type: rel.relation_type,
                            weight: rel.weight,
                            stability: rel.stability,
                            confidence: rel.confidence,
                            t_created: rel.t_created,
                        };
                        let new_src = sub.id_map[&rel.src];
                        let new_dst = sub.id_map[&rel.dst];
                        sub.graph.add_edge(new_src, new_dst, new_rel);
                    }
                }
            }
        }
        sub
    }

    pub fn edges_of(&self, id: AtomId) -> Vec<(EdgeIndex, Direction, &Relation)> {
        let idx = match self.id_map.get(&id) {
            Some(i) => *i,
            None => return Vec::new(),
        };
        let mut result = Vec::new();
        for edge in self.graph.edges_directed(idx, Direction::Incoming) {
            result.push((edge.id(), Direction::Incoming, edge.weight()));
        }
        for edge in self.graph.edges_directed(idx, Direction::Outgoing) {
            result.push((edge.id(), Direction::Outgoing, edge.weight()));
        }
        result
    }

    /// Get atom by NodeIndex (for cross-crate access)
    pub fn get_atom_by_index(&self, idx: NodeIndex) -> Option<&CognitiveAtom> {
        self.graph.node_weight(idx)
    }

    /// Get mutable atom by NodeIndex (for cross-crate access)
    pub fn get_atom_by_index_mut(&mut self, idx: NodeIndex) -> Option<&mut CognitiveAtom> {
        self.graph.node_weight_mut(idx)
    }

    pub fn total_inbound_weight(&self, id: AtomId) -> Decimal {
        let idx = match self.id_map.get(&id) {
            Some(i) => *i,
            None => return Decimal::ZERO,
        };
        self.graph
            .edges_directed(idx, Direction::Incoming)
            .map(|e| e.weight().weight.get())
            .fold(Decimal::ZERO, |acc, w| acc + w)
    }

    pub fn contradiction_edges(&self) -> Vec<(AtomId, AtomId, Decimal)> {
        self.graph
            .edge_indices()
            .filter_map(|edge| {
                let rel = self.graph.edge_weight(edge)?;
                if rel.relation_type == RelationType::Contradicts {
                    Some((rel.src, rel.dst, rel.weight.get()))
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Default for CognitiveGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_atom(name: &str) -> CognitiveAtom {
        CognitiveAtom::new(AtomId::new(), AtomType::Concept, name.to_string())
    }

    #[test]
    fn test_add_and_get_atom() {
        let mut g = CognitiveGraph::new();
        let atom = make_atom("test");
        let id = atom.id;
        g.add_atom(atom);
        assert!(g.get_atom(id).is_some());
        assert_eq!(g.get_atom(id).unwrap().canonical, "test");
    }

    #[test]
    fn test_add_relation() {
        let mut g = CognitiveGraph::new();
        let a = make_atom("a");
        let b = make_atom("b");
        let id_a = a.id;
        let id_b = b.id;
        g.add_atom(a);
        g.add_atom(b);
        let edge = g.add_relation(id_a, id_b, RelationType::Causes).unwrap();
        let rel = g.get_relation(edge).unwrap();
        assert_eq!(rel.relation_type, RelationType::Causes);
        assert_eq!(rel.src, id_a);
        assert_eq!(rel.dst, id_b);
    }

    #[test]
    fn test_remove_atom() {
        let mut g = CognitiveGraph::new();
        let a = make_atom("a");
        let b = make_atom("b");
        let id_a = a.id;
        let id_b = b.id;
        g.add_atom(a);
        g.add_atom(b);
        g.add_relation(id_a, id_b, RelationType::Causes).unwrap();
        g.remove_atom(id_a);
        assert!(g.get_atom(id_a).is_none());
        assert_eq!(g.relation_count(), 0);
    }

    #[test]
    fn test_k_hop_neighbors() {
        let mut g = CognitiveGraph::new();
        let atoms: Vec<_> = (0..5).map(|i| make_atom(&format!("n{}", i))).collect();
        let ids: Vec<_> = atoms.iter().map(|a| a.id).collect();
        for a in atoms {
            g.add_atom(a);
        }
        for i in 0..4 {
            g.add_relation(ids[i], ids[i + 1], RelationType::Causes).unwrap();
        }
        let neighbors = g.k_hop_neighbors(ids[0], 2);
        assert_eq!(neighbors.len(), 3);
        assert_eq!(neighbors[&g.node_index(ids[0]).unwrap()], 0);
        assert_eq!(neighbors[&g.node_index(ids[1]).unwrap()], 1);
        assert_eq!(neighbors[&g.node_index(ids[2]).unwrap()], 2);
    }

    #[test]
    fn test_degree_distribution() {
        let mut g = CognitiveGraph::new();
        let center = make_atom("center");
        let center_id = center.id;
        g.add_atom(center);
        for i in 0..4 {
            let leaf = make_atom(&format!("leaf{}", i));
            let leaf_id = leaf.id;
            g.add_atom(leaf);
            g.add_relation(center_id, leaf_id, RelationType::Causes).unwrap();
        }
        let dist = g.degree_distribution();
        // In a directed graph, edges() returns outgoing edges only
        // center has 4 outgoing, each leaf has 0 outgoing
        assert_eq!(dist.get(&4), Some(&1)); // center
        assert_eq!(dist.get(&0), Some(&4)); // 4 leaves
    }
    #[test]
    fn test_subgraph() {
        let mut g = CognitiveGraph::new();
        let atoms: Vec<_> = (0..6).map(|i| make_atom(&format!("n{}", i))).collect();
        let ids: Vec<_> = atoms.iter().map(|a| a.id).collect();
        for a in atoms {
            g.add_atom(a);
        }
        for i in 0..5 {
            g.add_relation(ids[i], ids[i + 1], RelationType::Causes).unwrap();
        }
        let sub_nodes: Vec<_> = (1..=3).map(|i| g.node_index(ids[i]).unwrap()).collect();
        let sub = g.subgraph(&sub_nodes);
        assert_eq!(sub.atom_count(), 3);
        assert_eq!(sub.relation_count(), 2);
    }

    #[test]
    fn test_contradiction_edges() {
        let mut g = CognitiveGraph::new();
        let a = make_atom("a");
        let b = make_atom("b");
        let id_a = a.id;
        let id_b = b.id;
        g.add_atom(a);
        g.add_atom(b);
        g.add_relation(id_a, id_b, RelationType::Contradicts).unwrap();
        let contradictions = g.contradiction_edges();
        assert_eq!(contradictions.len(), 1);
        assert_eq!(contradictions[0].0, id_a);
        assert_eq!(contradictions[0].1, id_b);
    }

    #[test]
    fn test_edge_weight_evolution() {
        let mut g = CognitiveGraph::new();
        let a = make_atom("a");
        let b = make_atom("b");
        let id_a = a.id;
        let id_b = b.id;
        g.add_atom(a);
        g.add_atom(b);
        let edge = g.add_relation(id_a, id_b, RelationType::Causes).unwrap();
        let config = EngineConfig::default();
        {
            let rel = g.get_relation_mut(edge).unwrap();
            rel.evolve_weight(Decimal::ONE, Decimal::ZERO, &config);
        }
        let weight = g.edge_weight(edge).unwrap();
        assert_eq!(weight, Decimal::from_str_exact("0.5").unwrap());
    }

    #[test]
    fn test_edges_of() {
        let mut g = CognitiveGraph::new();
        let a = make_atom("a");
        let b = make_atom("b");
        let c = make_atom("c");
        let id_a = a.id;
        let id_b = b.id;
        let id_c = c.id;
        g.add_atom(a);
        g.add_atom(b);
        g.add_atom(c);
        g.add_relation(id_a, id_b, RelationType::Causes).unwrap();
        g.add_relation(id_c, id_a, RelationType::Supports).unwrap();
        let edges = g.edges_of(id_a);
        assert_eq!(edges.len(), 2);
    }
}
