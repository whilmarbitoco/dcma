pub mod phi;
pub mod tick;

pub use phi::{TickResult, Stimulus};
pub use tick::{Runtime, CONSOLIDATION_INTERVAL};
pub use dcma_core::state::CognitiveState;
pub use dcma_core::types::EngineConfig;
