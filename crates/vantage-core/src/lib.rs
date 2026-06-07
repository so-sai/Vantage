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

/// TIA-2: Temporal epoch identifier.
/// Policy changes, revocation snapshots, and certificate validity are epoch-relative.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EpochId(pub u64);

/// TIA-2: Logical time using Lamport clock semantics.
/// Enables deterministic ordering without wall-clock dependency.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct LogicalTime {
    pub lamport: u64,
}

impl LogicalTime {
    pub fn new(lamport: u64) -> Self {
        Self { lamport }
    }
}

/// TIA-2: Execution envelope — binds temporal context to a mutation.
/// Runtime validates monotonicity and epoch consistency; it does NOT interpret governance.
#[derive(Debug, Clone)]
pub struct ExecutionEnvelope {
    pub epoch: EpochId,
    pub sequence: u64,
    pub logical_time: LogicalTime,
}

impl ExecutionEnvelope {
    pub fn new(epoch: EpochId, sequence: u64, logical_time: LogicalTime) -> Self {
        Self { epoch, sequence, logical_time }
    }
}

/// PRN-1: Epoch lifecycle state.
/// Active → Locked → Committed.
/// Locked allows in-flight completion only; no new payloads accepted.
/// Committed is irreversible — epoch is sealed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpochState {
    Active,
    Locked,
    Committed,
}

/// PRN-1: Request to commit an epoch transition.
pub struct CommitRequest {
    pub epoch: EpochId,
    pub final_sequence: u64,
    pub state_root: u64,
}

/// PRN-1: Acknowledgement from a node in commit quorum.
pub struct CommitAck {
    pub node_id: String,
    pub epoch: EpochId,
    pub state_root_match: bool,
}

/// PRN-1: Result of a commit attempt.
#[derive(Debug, Clone)]
pub struct CommitResult {
    pub epoch: EpochId,
    pub success: bool,
    pub ack_count: u32,
    pub threshold: u32,
}

impl CommitResult {
    pub fn new(epoch: EpochId, success: bool, ack_count: u32, threshold: u32) -> Self {
        Self { epoch, success, ack_count, threshold }
    }
}

/// PRN-1: Node identifier in distributed context.
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NodeId(pub String);

/// PRN-1: A proposal for epoch transition from a participating node.
#[derive(Debug, Clone)]
pub struct EpochProposal {
    pub epoch: EpochId,
    pub policy_snapshot: u64,
    pub min_sequence: u64,
    pub cutoff_time: LogicalTime,
    pub proposer: NodeId,
    pub trust_weight: u64,
}

/// PRN-1: A quorum of nodes supporting the same epoch proposal.
#[derive(Debug, Clone)]
pub struct EpochQuorum {
    pub epoch: EpochId,
    pub supporters: Vec<NodeId>,
    pub aggregate_score: u64,
    pub policy_snapshot: u64,
}

/// PRN-1: Result of an election round.
#[derive(Debug, Clone)]
pub enum ElectionResult {
    NoConsensus,
    Candidate(EpochQuorum),
}

/// PRN-2: Placeholder signature for cross-node bundle attestation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature(pub Vec<u8>);

/// PRN-2: A node's locally-finalized PRN-1 election result, signed for cross-node exchange.
#[derive(Debug, Clone)]
pub struct EpochCandidateBundle {
    pub node_id: NodeId,
    pub epoch: EpochId,
    pub quorum: EpochQuorum,
    pub aggregate_score: u64,
    pub signature: Signature,
}

/// PRN-2: Globally agreed epoch after meta-election reconciliation.
#[derive(Debug, Clone)]
pub struct GlobalEpochAgreement {
    pub epoch: EpochId,
    pub quorum: EpochQuorum,
    pub supporting_nodes: Vec<NodeId>,
    pub global_score: u64,
}

/// PRN-2: Result of a meta-election round.
#[derive(Debug, Clone)]
pub enum MetaElectionResult {
    NoConsensus,
    Agreement(GlobalEpochAgreement),
}

/// PRN-2: Transport message for exchanging candidate bundles between nodes.
#[derive(Debug, Clone)]
pub struct PRN2Message {
    pub from: NodeId,
    pub bundle: EpochCandidateBundle,
    pub ttl_epoch: EpochId,
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
