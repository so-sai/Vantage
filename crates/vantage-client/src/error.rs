use std::time::SystemTime;
use vantage_pek::PEKError;
use vantage_core::ResourceId;

#[derive(Debug, Clone)]
pub struct StatsSnapshot {
    pub admitted: u64,
    pub rejected: u64,
    pub advisory_warnings: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum VantageError {
    #[error("Mutation rejected by PEK: {0}")]
    MutationRejected(#[from] PEKError),

    #[error("Resource {0:?} not found")]
    ResourceNotFound(ResourceId),

    #[error("Epoch operation failed: {0}")]
    EpochError(String),

    #[error("Query failed: {0}")]
    QueryError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub sequence: u64,
    pub resource_id: ResourceId,
    pub mutation_id: String,
    pub actor: String,
    pub timestamp: SystemTime,
    pub payload: Option<String>,
}
