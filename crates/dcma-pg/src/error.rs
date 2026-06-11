use thiserror::Error;

#[derive(Debug, Error)]
pub enum PgError {
    #[error("Database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Schema migration failed: {0}")]
    Migration(String),
}

impl From<PgError> for dcma_core::state::StateError {
    fn from(_e: PgError) -> Self {
        dcma_core::state::StateError::InvalidActivation(
            dcma_core::types::AtomId::new(),
            rust_decimal::Decimal::ZERO,
        )
    }
}
