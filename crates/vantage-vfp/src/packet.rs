use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use vantage_core::{EpochId, NodeId, Signature};

use crate::predictive::PredictionCommitment;
use crate::state::{CommitmentHash, NodeView};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationPacket {
    pub origin: NodeView,
    pub known_peers: HashMap<NodeId, (EpochId, CommitmentHash)>,
    pub signature: Signature,
    pub claims: Vec<PredictionCommitment>,
}

impl FederationPacket {
    pub fn new(
        origin: NodeView,
        known_peers: HashMap<NodeId, (EpochId, CommitmentHash)>,
        signature: Signature,
    ) -> Self {
        Self { origin, known_peers, signature, claims: Vec::new() }
    }

    pub fn with_claims(
        origin: NodeView,
        known_peers: HashMap<NodeId, (EpochId, CommitmentHash)>,
        signature: Signature,
        claims: Vec<PredictionCommitment>,
    ) -> Self {
        Self { origin, known_peers, signature, claims }
    }

    pub fn origin_node(&self) -> &NodeId {
        &self.origin.node_id
    }

    pub fn origin_epoch(&self) -> EpochId {
        self.origin.epoch
    }

    pub fn shares_genesis_with(&self, local: &NodeView) -> bool {
        self.origin.genesis_hash == local.genesis_hash
    }
}
