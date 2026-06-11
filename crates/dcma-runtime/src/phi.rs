use dcma_activation::convergence::ConvergenceInfo;
use dcma_consolidation::crystallization::CrystallizationReport;
use dcma_core::types::*;
use dcma_entropy::entropy::EntropyStatus;
use dcma_topology::edge::{StabilizationReport, WeightChangeReport};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct TickResult {
    pub tick: u64,
    pub convergence: ConvergenceInfo,
    pub entropy_status: EntropyStatus,
    pub regulation_applied: bool,
    pub consolidation_applied: bool,
    pub consolidation_report: Option<CrystallizationReport>,
    pub weight_changes: WeightChangeReport,
    pub stabilization: StabilizationReport,
    pub activations: HashMap<AtomId, Activation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Stimulus {
    pub source_nodes: Vec<AtomId>,
    pub query_context: Vec<AtomId>,
    pub label: String,
}
