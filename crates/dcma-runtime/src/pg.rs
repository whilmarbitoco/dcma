use dcma_pg::PgStorage;
use crate::{Runtime, TickResult, Stimulus};
use dcma_core::state::CognitiveState;
use dcma_core::types::EngineConfig;

/// Runtime with PostgreSQL persistence
pub struct PgRuntime {
    runtime: Runtime,
    storage: PgStorage,
}

impl PgRuntime {
    /// Create a new PgRuntime with a PostgreSQL connection
    pub async fn new(config: EngineConfig, pg_url: &str) -> Result<Self, dcma_pg::PgError> {
        let storage = PgStorage::connect(pg_url).await?;
        let runtime = Runtime::new(config);
        Ok(Self { runtime, storage })
    }

    /// Create from an existing state loaded from PostgreSQL
    pub async fn with_state(config: EngineConfig, pg_url: &str) -> Result<Self, dcma_pg::PgError> {
        let storage = PgStorage::connect(pg_url).await?;
        let state = storage.load_state().await?;
        let runtime = Runtime::with_state(config, state);
        Ok(Self { runtime, storage })
    }

    /// Create from a pre-existing state and storage connection
    pub fn with_state_and_storage(config: EngineConfig, state: CognitiveState, storage: PgStorage) -> Self {
        let runtime = Runtime::with_state(config, state);
        Self { runtime, storage }
    }

    /// Execute a tick and persist the result to PostgreSQL
    pub async fn tick(&mut self, stimulus: &Stimulus) -> TickResult {
        let result = self.runtime.tick(stimulus);
        // Persist state after each tick (ignore errors for now — log them)
        if let Err(e) = self.storage.save_state(self.runtime.state()).await {
            tracing::warn!("Failed to persist state: {}", e);
        }
        result
    }

    /// Execute multiple ticks
    pub async fn run(&mut self, stimuli: &[Stimulus]) -> Vec<TickResult> {
        let mut results = Vec::new();
        for stimulus in stimuli {
            results.push(self.tick(stimulus).await);
        }
        results
    }

    /// Get the current in-memory state
    pub fn state(&self) -> &CognitiveState {
        self.runtime.state()
    }

    /// Get mutable state
    pub fn state_mut(&mut self) -> &mut CognitiveState {
        self.runtime.state_mut()
    }

    /// Get tick count
    pub fn tick_count(&self) -> u64 {
        self.runtime.tick_count()
    }

    /// Get the storage backend
    pub fn storage(&self) -> &PgStorage {
        &self.storage
    }

    /// Get the inner runtime
    pub fn inner(&self) -> &Runtime {
        &self.runtime
    }
}
