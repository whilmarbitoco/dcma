use dcma_core::state::CognitiveState;
use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON serialization error: {0}")]
    JsonSerialization(#[from] serde_json::Error),
    #[error("State file corrupted")]
    Corrupted,
}

pub struct Storage;

impl Storage {
    pub fn save_state(state: &CognitiveState, path: &Path) -> Result<(), StorageError> {
        let json = serde_json::to_string(state)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load_state(path: &Path) -> Result<CognitiveState, StorageError> {
        let json = fs::read_to_string(path)?;
        let state: CognitiveState = serde_json::from_str(&json)?;
        Ok(state)
    }

    pub fn save_state_json(state: &CognitiveState, path: &Path) -> Result<(), StorageError> {
        let json = serde_json::to_string_pretty(state)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load_state_json(path: &Path) -> Result<CognitiveState, StorageError> {
        let json = fs::read_to_string(path)?;
        let state: CognitiveState = serde_json::from_str(&json)?;
        Ok(state)
    }

    pub fn append_wal(wal_path: &Path, tick: u64, state: &CognitiveState) -> Result<(), StorageError> {
        let json = serde_json::to_string(&(tick, state))?;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(wal_path)?;
        writeln!(file, "{}", json)?;
        Ok(())
    }

    pub fn replay_wal(wal_path: &Path, _checkpoint: &CognitiveState) -> Result<Vec<(u64, CognitiveState)>, StorageError> {
        if !wal_path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(wal_path)?;
        let mut states = Vec::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let record: (u64, CognitiveState) = serde_json::from_str(line)?;
            states.push(record);
        }
        Ok(states)
    }

    pub fn checkpoint(state: &CognitiveState, dir: &Path, tick: u64) -> Result<(), StorageError> {
        fs::create_dir_all(dir)?;
        let filename = format!("checkpoint_{:020}.json", tick);
        let path = dir.join(filename);
        Self::save_state(state, &path)
    }

    pub fn load_latest_checkpoint(dir: &Path) -> Result<(u64, CognitiveState), StorageError> {
        fs::create_dir_all(dir)?;
        let mut entries: Vec<_> = fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|ext| ext == "json").unwrap_or(false))
            .collect();
        entries.sort_by_key(|e| e.path().to_string_lossy().to_string());
        if let Some(latest) = entries.last() {
            let path = latest.path();
            let filename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let tick: u64 = filename
                .strip_prefix("checkpoint_")
                .and_then(|s| s.parse().ok())
                .ok_or(StorageError::Corrupted)?;
            let state = Self::load_state(&path)?;
            Ok((tick, state))
        } else {
            Err(StorageError::Corrupted)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dcma_core::state::CognitiveState;
    use dcma_core::types::EngineConfig;
    use dcma_runtime::{Runtime, Stimulus};
    use tempfile::TempDir;

    fn make_temp_dir() -> TempDir {
        TempDir::new().unwrap()
    }

    #[test]
    fn test_save_load_binary() {
        let state = CognitiveState::new(EngineConfig::default());
        let dir = make_temp_dir();
        let path = dir.path().join("state.json");
        Storage::save_state(&state, &path).unwrap();
        let loaded = Storage::load_state(&path).unwrap();
        assert_eq!(loaded.tick, state.tick);
        assert_eq!(loaded.graph.atom_count(), state.graph.atom_count());
    }

    #[test]
    fn test_save_load_json() {
        let state = CognitiveState::new(EngineConfig::default());
        let dir = make_temp_dir();
        let path = dir.path().join("state.pretty.json");
        Storage::save_state_json(&state, &path).unwrap();
        let loaded = Storage::load_state_json(&path).unwrap();
        assert_eq!(loaded.tick, state.tick);
        assert_eq!(loaded.graph.atom_count(), state.graph.atom_count());
    }

    #[test]
    fn test_roundtrip_with_runtime() {
        let config = EngineConfig::default();
        let mut rt = Runtime::new(config);
        let dir = make_temp_dir();
        let path = dir.path().join("rt_state.json");

        for _ in 0..10 {
            let stimulus = Stimulus {
                source_nodes: vec![],
                query_context: vec![],
                label: "auto".into(),
            };
            rt.tick(&stimulus);
        }

        Storage::save_state(rt.state(), &path).unwrap();
        let loaded = Storage::load_state(&path).unwrap();
        assert_eq!(loaded.tick, 10);

        let config2 = EngineConfig::default();
        let mut rt2 = Runtime::with_state(config2, loaded);

        for _ in 0..10 {
            let stimulus = Stimulus {
                source_nodes: vec![],
                query_context: vec![],
                label: "auto".into(),
            };
            rt2.tick(&stimulus);
        }

        assert_eq!(rt2.tick_count(), 20);
    }

    #[test]
    fn test_wal_append() {
        let config = EngineConfig::default();
        let state = CognitiveState::new(config);
        let dir = make_temp_dir();
        let wal_path = dir.path().join("wal.json");

        Storage::append_wal(&wal_path, 1, &state).unwrap();
        Storage::append_wal(&wal_path, 2, &state).unwrap();

        let records = Storage::replay_wal(&wal_path, &state).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].0, 1);
        assert_eq!(records[1].0, 2);
    }

    #[test]
    fn test_checkpoint() {
        let state = CognitiveState::new(EngineConfig::default());
        let dir = make_temp_dir();
        let cp_dir = dir.path().join("checkpoints");

        Storage::checkpoint(&state, &cp_dir, 42).unwrap();
        let (tick, loaded) = Storage::load_latest_checkpoint(&cp_dir).unwrap();
        assert_eq!(tick, 42);
        assert_eq!(loaded.graph.atom_count(), 0);
    }

    #[test]
    fn test_empty_state_roundtrip() {
        let state = CognitiveState::new(EngineConfig::default());
        let dir = make_temp_dir();
        let path = dir.path().join("empty.json");

        Storage::save_state(&state, &path).unwrap();
        let loaded = Storage::load_state(&path).unwrap();
        assert_eq!(loaded.graph.atom_count(), 0);
        assert_eq!(loaded.tick, 0);
    }
}
