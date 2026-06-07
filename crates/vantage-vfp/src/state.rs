use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vantage_core::{EpochId, NodeId};

const HASH_BYTES: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommitmentHash(pub [u8; HASH_BYTES]);

impl CommitmentHash {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let result = hasher.finalize();
        let mut arr = [0u8; HASH_BYTES];
        arr.copy_from_slice(&result);
        CommitmentHash(arr)
    }

    pub fn genesis() -> Self {
        Self::from_bytes(b"vantage-genesis")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeView {
    pub node_id: NodeId,
    pub epoch: EpochId,
    pub commit_hash: CommitmentHash,
    pub genesis_hash: CommitmentHash,
}

impl NodeView {
    pub fn new(
        node_id: NodeId,
        epoch: EpochId,
        commit_hash: CommitmentHash,
        genesis_hash: CommitmentHash,
    ) -> Self {
        Self { node_id, epoch, commit_hash, genesis_hash }
    }

    pub fn shares_genesis(&self, other: &NodeView) -> bool {
        self.genesis_hash == other.genesis_hash
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationNodeState {
    pub node_id: NodeId,
    pub epoch: EpochId,
    pub commit_hash: CommitmentHash,
    pub genesis_hash: CommitmentHash,
    pub known_peers: Vec<NodeView>,
}

impl FederationNodeState {
    pub fn new(node_id: NodeId, epoch: EpochId, genesis_hash: CommitmentHash) -> Self {
        let commit_hash = CommitmentHash::from_bytes(
            format!("{}:{}", node_id.0, epoch.0).as_bytes(),
        );
        Self { node_id, epoch, commit_hash, genesis_hash, known_peers: Vec::new() }
    }

    pub fn view(&self) -> NodeView {
        NodeView::new(
            self.node_id.clone(),
            self.epoch,
            self.commit_hash.clone(),
            self.genesis_hash.clone(),
        )
    }
}
