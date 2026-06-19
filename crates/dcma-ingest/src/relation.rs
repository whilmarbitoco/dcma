use dcma_core::types::RelationType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtractedRelation {
    pub src_idx: usize,
    pub dst_idx: usize,
    pub relation_type: RelationType,
    pub confidence: f64,
}

impl ExtractedRelation {
    pub fn new(src_idx: usize, dst_idx: usize, relation_type: RelationType, confidence: f64) -> Self {
        Self {
            src_idx,
            dst_idx,
            relation_type,
            confidence,
        }
    }
}
