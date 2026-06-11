pub mod hlc;
pub mod merge;
pub mod protocol;

use dcma_core::state::CognitiveState;
use dcma_core::types::{AtomId, EngineConfig};
use dcma_runtime::phi::TickResult;
use dcma_runtime::Runtime;
use hlc::Timestamp;
use merge::merge_states;
use protocol::Message;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Node {
    id: AtomId,
    config: EngineConfig,
    runtime: Runtime,
    peers: HashMap<AtomId, Timestamp>,
    hlc: Timestamp,
    pub(crate) last_heartbeat: HashMap<AtomId, u64>,
}

impl Node {
    pub fn new(config: EngineConfig) -> Self {
        let node_id = AtomId::new();
        let hlc = Timestamp::now(node_id);
        let runtime = Runtime::new(config.clone());
        Self {
            id: node_id,
            config,
            runtime,
            peers: HashMap::new(),
            hlc,
            last_heartbeat: HashMap::new(),
        }
    }

    pub fn connect(&mut self, peer_id: AtomId) {
        self.peers.insert(peer_id, Timestamp::new(peer_id));
        self.last_heartbeat.insert(peer_id, current_time_ms());
    }

    pub fn broadcast_tick(&mut self, result: &TickResult) {
        self.hlc.tick();
        let state = self.runtime.state();
        let fragment = state.clone();
        for _peer_id in self.peers.keys() {
            let msg = Message::TickSync {
                timestamp: self.hlc,
                tick: result.tick,
                state_fragment: fragment.clone(),
            };
            let _ = protocol::serialize_message(&msg);
        }
    }

    pub fn receive_state(&mut self, msg: &Message) -> Result<(), dcma_storage::StorageError> {
        match msg {
            Message::TickSync {
                timestamp,
                tick: _,
                state_fragment,
            } => {
                self.hlc.receive(timestamp);
                let current_state = self.runtime.state();
                let merged = merge_states(current_state, state_fragment);
                *self.runtime.state_mut() = merged;
                Ok(())
            }
            Message::StateRequest {
                timestamp,
                node_ids: _,
            } => {
                self.hlc.receive(timestamp);
                Ok(())
            }
            Message::StateResponse {
                timestamp,
                state_fragment,
            } => {
                self.hlc.receive(timestamp);
                let current_state = self.runtime.state();
                let merged = merge_states(current_state, state_fragment);
                *self.runtime.state_mut() = merged;
                Ok(())
            }
            Message::MergeRequest {
                timestamp,
                state_fragment,
            } => {
                self.hlc.receive(timestamp);
                let current_state = self.runtime.state();
                let merged = merge_states(current_state, state_fragment);
                *self.runtime.state_mut() = merged;
                Ok(())
            }
            Message::MergeAck {
                timestamp,
                merged_state_hash: _,
            } => {
                self.hlc.receive(timestamp);
                Ok(())
            }
            Message::Heartbeat {
                timestamp,
                node_id,
            } => {
                self.hlc.receive(timestamp);
                self.last_heartbeat.insert(*node_id, current_time_ms());
                Ok(())
            }
        }
    }

    pub fn detect_partitions(&self) -> Vec<AtomId> {
        let now = current_time_ms();
        let timeout = 5000;
        self.peers
            .keys()
            .filter(|peer_id| {
                self.last_heartbeat
                    .get(peer_id)
                    .map(|last| now - last > timeout)
                    .unwrap_or(true)
            })
            .copied()
            .collect()
    }

    pub fn heal_partition(&mut self, peer_id: &AtomId) -> Result<(), dcma_storage::StorageError> {
        self.hlc.tick();
        let request = Message::StateRequest {
            timestamp: self.hlc,
            node_ids: vec![self.id],
        };
        let _ = protocol::serialize_message(&request);
        self.last_heartbeat.insert(*peer_id, current_time_ms());
        Ok(())
    }

    pub fn id(&self) -> AtomId {
        self.id
    }

    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    pub fn timestamp(&self) -> &Timestamp {
        &self.hlc
    }

    pub fn state(&self) -> &CognitiveState {
        self.runtime.state()
    }

    pub fn state_mut(&mut self) -> &mut CognitiveState {
        self.runtime.state_mut()
    }

    pub fn peers(&self) -> &HashMap<AtomId, Timestamp> {
        &self.peers
    }

    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    pub fn runtime_mut(&mut self) -> &mut Runtime {
        &mut self.runtime
    }
}

fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_node_creation() {
        let config = EngineConfig::default();
        let node = Node::new(config);
        assert!(node.id() != AtomId::default());
        assert_eq!(node.state().tick, 0);
    }

    #[test]
    fn test_detect_partitions() {
        let config = EngineConfig::default();
        let mut node = Node::new(config);
        let peer = AtomId::new();
        node.connect(peer);
        node.last_heartbeat.insert(peer, 0);
        let partitions = node.detect_partitions();
        assert!(!partitions.is_empty());
        assert!(partitions.contains(&peer));
    }
}
