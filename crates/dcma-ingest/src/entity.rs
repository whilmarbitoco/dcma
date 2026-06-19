use dcma_core::types::AtomType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtractedEntity {
    pub name: String,
    pub entity_type: AtomType,
    pub confidence: f64,
    pub position: usize,
}

impl ExtractedEntity {
    pub fn new(name: String, entity_type: AtomType, confidence: f64, position: usize) -> Self {
        Self {
            name,
            entity_type,
            confidence,
            position,
        }
    }

    pub fn canonical(&self) -> String {
        self.name.to_lowercase().trim().to_string()
    }
}
