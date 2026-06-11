pub mod relevance;
pub mod similarity;
pub mod beam;

pub use beam::{CognitiveField, RetrievalMetadata};
pub use relevance::{relevance, relevance_with_contradiction};
pub use similarity::structural_similarity;

use dcma_core::graph::CognitiveGraph;
use dcma_core::types::*;
use std::collections::HashMap;

pub fn retrieve(
    graph: &CognitiveGraph,
    query_nodes: &[AtomId],
    activations: &HashMap<AtomId, Activation>,
    tick: u64,
    config: &EngineConfig,
) -> CognitiveField {
    beam::retrieve_beam(graph, query_nodes, activations, tick, config)
}

pub fn retrieve_with_contradiction(
    graph: &CognitiveGraph,
    query_nodes: &[AtomId],
    activations: &HashMap<AtomId, Activation>,
    contradictions: &HashMap<(AtomId, AtomId), Pressure>,
    tick: u64,
    config: &EngineConfig,
) -> CognitiveField {
    beam::retrieve_beam_with_contradiction(graph, query_nodes, activations, contradictions, tick, config)
}
