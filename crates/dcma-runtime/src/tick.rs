use crate::phi::{TickResult, Stimulus};
use dcma_activation::convergence::ConvergenceInfo;
use dcma_activation::propagation::{activate, ActivationVector};
use dcma_consolidation::crystallization::consolidate;
use dcma_contradiction::pressure::ContradictionStructure;
use dcma_core::state::CognitiveState;
use dcma_core::types::*;
use dcma_entropy::entropy::{EntropyRegulator, EntropyStatus};
use dcma_topology::edge::{evolve_weights, stabilize_relations};
use std::collections::HashMap;

pub const CONSOLIDATION_INTERVAL: u64 = 100;

pub struct Runtime {
    config: EngineConfig,
    tick_count: u64,
    state: CognitiveState,
    contradiction_structure: ContradictionStructure,
    entropy_regulator: EntropyRegulator,
}

impl Runtime {
    pub fn new(config: EngineConfig) -> Self {
        let state = CognitiveState::new(config.clone());
        Self {
            config: config.clone(),
            tick_count: 0,
            contradiction_structure: ContradictionStructure::new(),
            entropy_regulator: EntropyRegulator::new(config),
            state,
        }
    }

    pub fn with_state(config: EngineConfig, state: CognitiveState) -> Self {
        let mut cs = ContradictionStructure::new();
        for ((a, b), p) in &state.contradictions {
            cs.set(*a, *b, *p);
        }
        Self {
            tick_count: state.tick,
            config: config.clone(),
            state,
            contradiction_structure: cs,
            entropy_regulator: EntropyRegulator::new(config),
        }
    }

    pub fn tick(&mut self, stimulus: &Stimulus) -> TickResult {
        self.tick_count += 1;
        self.state.advance_tick();

        // 1. Apply stimulus — set source nodes to activation 1.0
        for &id in &stimulus.source_nodes {
            self.state.set_activation(id, Activation::ONE);
        }

        // 2. Activation propagation
        let result = activate(
            &self.state.graph,
            &stimulus.source_nodes,
            &stimulus.query_context,
            self.tick_count,
            &self.config,
        );

        let initial = ActivationVector::init_from_sources(&self.state.graph, &stimulus.source_nodes);
        let max_diff = initial.max_diff(&result);
        let convergence = ConvergenceInfo {
            converged: max_diff < self.config.convergence_threshold.get(),
            iterations: 0,
            final_diff: max_diff,
        };

        self.state.activations = result.values;

        // 3. Contradiction update — update pressure for all Contradicts edges
        let activation_map: HashMap<AtomId, Activation> =
            self.state.activation_values().into_iter().collect();
        self.contradiction_structure
            .update_all(&mut self.state.graph, &activation_map, &self.config);

        // Sync contradiction data back to state
        for ((a, b), p) in self.contradiction_structure.pairs() {
            self.state.set_contradiction(a, b, p);
        }

        // 4. Entropy check — compute H(G), apply regulation if out of bounds
        let entropy_status = self.entropy_regulator.check(&self.state.graph);
        let regulation_applied = !matches!(entropy_status, EntropyStatus::Normal(_));
        if regulation_applied {
            self.entropy_regulator.regulate(&self.state.graph);
        }

        // 5. Topology evolution — evolve edge weights, update relation stability
        let weight_changes =
            evolve_weights(&mut self.state.graph, &self.config, self.tick_count);
        let stabilization =
            stabilize_relations(&mut self.state.graph, &self.config);

        // 6. Consolidation — run every CONSOLIDATION_INTERVAL ticks
        let (consolidation_applied, consolidation_report) =
            if self.tick_count % CONSOLIDATION_INTERVAL == 0 {
                let report = consolidate(&mut self.state.graph, &self.config);
                (true, Some(report))
            } else {
                (false, None)
            };

        // 7. Record history
        self.state.record_history();

        TickResult {
            tick: self.tick_count,
            convergence,
            entropy_status,
            regulation_applied,
            consolidation_applied,
            consolidation_report,
            weight_changes,
            stabilization,
            activations: self.state.activation_values().into_iter().collect(),
        }
    }

    pub fn run(&mut self, stimuli: &[Stimulus]) -> Vec<TickResult> {
        stimuli.iter().map(|s| self.tick(s)).collect()
    }

    pub fn state(&self) -> &CognitiveState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut CognitiveState {
        &mut self.state
    }

    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }

    pub fn get_activation(&self, id: AtomId) -> Activation {
        self.state.get_activation(id).unwrap_or(Activation::ZERO)
    }

    pub fn activations(&self) -> HashMap<AtomId, Activation> {
        self.state.activation_values().into_iter().collect()
    }
}
