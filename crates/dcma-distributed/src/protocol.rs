use crate::hlc::Timestamp;
use dcma_core::state::CognitiveState;
use dcma_core::types::AtomId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    TickSync {
        timestamp: Timestamp,
        tick: u64,
        state_fragment: CognitiveState,
    },
    StateRequest {
        timestamp: Timestamp,
        node_ids: Vec<AtomId>,
    },
    StateResponse {
        timestamp: Timestamp,
        state_fragment: CognitiveState,
    },
    MergeRequest {
        timestamp: Timestamp,
        state_fragment: CognitiveState,
    },
    MergeAck {
        timestamp: Timestamp,
        merged_state_hash: u64,
    },
    Heartbeat {
        timestamp: Timestamp,
        node_id: AtomId,
    },
}

pub fn serialize_message(msg: &Message) -> Result<Vec<u8>, dcma_storage::StorageError> {
    serde_json::to_vec(msg).map_err(dcma_storage::StorageError::JsonSerialization)
}

pub fn deserialize_message(data: &[u8]) -> Result<Message, dcma_storage::StorageError> {
    serde_json::from_slice(data).map_err(dcma_storage::StorageError::JsonSerialization)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hlc::Timestamp;
    use dcma_core::state::CognitiveState;
    use dcma_core::types::{AtomId, EngineConfig};

    #[test]
    fn test_serialize_message() {
        let node_id = AtomId::new();
        let ts = Timestamp::new(node_id);
        let state = CognitiveState::new(EngineConfig::default());
        let msg = Message::TickSync {
            timestamp: ts,
            tick: 42,
            state_fragment: state,
        };
        let bytes = serialize_message(&msg).unwrap();
        let deserialized = deserialize_message(&bytes).unwrap();
        match deserialized {
            Message::TickSync { tick, .. } => {
                assert_eq!(tick, 42);
            }
            _ => panic!("Wrong message type"),
        }
    }
}
