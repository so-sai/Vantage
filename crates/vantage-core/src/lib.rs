use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceId(pub String);

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentId(pub String);

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MutationOp {
    Insert { resource_id: ResourceId, payload: String },
    Delete { resource_id: ResourceId },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeMutation {
    pub mutation_id: MutationId,
    pub actor: AgentId,
    pub op: MutationOp,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitReceipt {
    pub tx_id: String,
    pub actor: AgentId,
    pub timestamp: SystemTime,
    pub invariant_hash: String,
}

#[derive(Debug, Clone)]
pub enum InvariantViolation {
    Contradiction { reason: String },
    CircularDependency { path: Vec<ResourceId> },
    Custom(String),
}

pub trait EpistemicReader {
    fn read_unit(&self, id: &ResourceId) -> Option<String>;
    fn exists(&self, id: &ResourceId) -> bool;
}

pub struct InvariantContext<'a, R: EpistemicReader + ?Sized> {
    pub current_world_view: &'a R,
    pub proposal: &'a KnowledgeMutation,
}

pub trait EpistemicInvariant: Send + Sync {
    fn name(&self) -> &str;
    fn validate<'a>(&self, ctx: &InvariantContext<'a, dyn EpistemicReader + 'a>) -> Result<(), InvariantViolation>;
}
