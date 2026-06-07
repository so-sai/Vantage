use std::collections::HashMap;
use vantage_core::{EpochId, NodeId};

use crate::packet::FederationPacket;
use crate::state::{CommitmentHash, NodeView};

#[derive(Debug, Clone)]
pub struct FederationView {
    pub local: NodeView,
    pub peers: HashMap<NodeId, NodeView>,
}

impl FederationView {
    pub fn new(local: NodeView) -> Self {
        Self { local, peers: HashMap::new() }
    }

    pub fn ingest(&mut self, packet: FederationPacket) {
        for (peer_id, (epoch, hash)) in &packet.known_peers {
            let view = NodeView::new(
                peer_id.clone(),
                *epoch,
                hash.clone(),
                packet.origin.genesis_hash.clone(),
            );
            self.peers.entry(peer_id.clone()).or_insert(view);
        }

        self.peers
            .entry(packet.origin.node_id.clone())
            .and_modify(|existing| {
                if packet.origin.epoch > existing.epoch {
                    *existing = packet.origin.clone();
                }
            })
            .or_insert(packet.origin);
    }

    pub fn nth_commitment(&self, node: &NodeId, epoch: EpochId) -> Option<CommitmentHash> {
        let view = if *node == self.local.node_id {
            if epoch == self.local.epoch {
                return Some(self.local.commit_hash.clone());
            }
            return None;
        } else {
            self.peers.get(node)?
        };
        if view.epoch == epoch {
            Some(view.commit_hash.clone())
        } else {
            None
        }
    }

    pub fn node_views(&self) -> Vec<&NodeView> {
        std::iter::once(&self.local)
            .chain(self.peers.values())
            .collect()
    }
}
