use dcma_core::state::CognitiveState;
use dcma_core::types::*;
use dcma_runtime::{Runtime, Stimulus};
use dcma_topology::edge::clear_weight_history;

fn make_stimulus() -> Stimulus {
    Stimulus {
        source_nodes: vec![],
        query_context: vec![],
        label: "tick".to_string(),
    }
}

#[test]
fn test_tick_determinism() {
    clear_weight_history();

    let stimulus = make_stimulus();

    let mut runtime1 = Runtime::new(EngineConfig::default());
    for _ in 0..50 {
        runtime1.tick(&stimulus);
    }

    clear_weight_history();

    let mut runtime2 = Runtime::new(EngineConfig::default());
    for _ in 0..50 {
        runtime2.tick(&stimulus);
    }

    let state1_json = runtime1.state().to_json().unwrap();
    let state2_json = runtime2.state().to_json().unwrap();
    assert_eq!(
        state1_json, state2_json,
        "Two identical runs must produce identical states"
    );

    assert_eq!(
        runtime1.activations().len(),
        runtime2.activations().len(),
        "Activations must have same count across runs"
    );
}

#[test]
fn test_tick_determinism_across_runs() {
    clear_weight_history();

    let stimulus = make_stimulus();

    let mut runtime1 = Runtime::new(EngineConfig::default());
    for _ in 0..25 {
        runtime1.tick(&stimulus);
    }

    let json = runtime1.state().to_json().unwrap();
    let restored =
        CognitiveState::from_json(&json).expect("State must deserialize from JSON");

    let config = EngineConfig::default();
    let mut runtime2 = Runtime::with_state(config, restored);
    for _ in 0..25 {
        runtime2.tick(&stimulus);
    }

    for _ in 25..50 {
        runtime1.tick(&stimulus);
    }

    let state1_json = runtime1.state().to_json().unwrap();
    let state2_json = runtime2.state().to_json().unwrap();
    assert_eq!(
        state1_json, state2_json,
        "State loaded from JSON must produce identical final state"
    );
}
