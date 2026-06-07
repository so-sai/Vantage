use vantage_core::{EpochId, NodeId};

use crate::divergence::EpochDivergenceState;
use crate::packet::FederationPacket;
use crate::view::FederationView;

#[derive(Debug, Clone, PartialEq)]
pub enum IncompatibilityReason {
    GenesisMismatch,
    EpochCollision(EpochId),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AlignmentRelation {
    Identical,
    FastForward {
        ahead: NodeId,
        behind: NodeId,
        common_epoch: EpochId,
        catch_up_epochs: Vec<EpochId>,
    },
    Forked {
        fork_epoch: EpochId,
        local_divergence: f64,
        remote_divergence: f64,
    },
    Incompatible {
        reason: IncompatibilityReason,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum AlignmentAction {
    NoAction,
    CatchUpTo(NodeId),
    RebaseOnto {
        fork_epoch: EpochId,
        remote_node: NodeId,
    },
    DeclareMisalignment(IncompatibilityReason),
}

#[derive(Debug, Clone, PartialEq)]
pub struct AlignmentHint {
    pub target: NodeId,
    pub action: AlignmentAction,
}

impl AlignmentHint {
    pub fn no_action(target: NodeId) -> Self {
        Self { target, action: AlignmentAction::NoAction }
    }

    pub fn catch_up(target: NodeId, ahead: NodeId) -> Self {
        Self { target, action: AlignmentAction::CatchUpTo(ahead) }
    }

    pub fn rebase(target: NodeId, fork_epoch: EpochId, remote: NodeId) -> Self {
        Self {
            target,
            action: AlignmentAction::RebaseOnto { fork_epoch, remote_node: remote },
        }
    }
}

#[derive(Debug, Clone)]
pub enum AlignmentResult {
    Relation(AlignmentRelation),
    Hints(Vec<AlignmentHint>),
}

impl FederationView {
    pub fn compute_alignment(&self, remote: &FederationPacket) -> AlignmentRelation {
        if !remote.shares_genesis_with(&self.local) {
            return AlignmentRelation::Incompatible {
                reason: IncompatibilityReason::GenesisMismatch,
            };
        }

        let local_epoch = self.local.epoch;
        let remote_epoch = remote.origin_epoch();

        let local_hash = &self.local.commit_hash;
        let remote_hash = &remote.origin.commit_hash;

        let epoch_delta = (local_epoch.0 as i64 - remote_epoch.0 as i64).abs();

        if local_epoch == remote_epoch {
            if local_hash == remote_hash {
                return AlignmentRelation::Identical;
            }
            return AlignmentRelation::Incompatible {
                reason: IncompatibilityReason::EpochCollision(local_epoch),
            };
        }

        if local_epoch > remote_epoch {
            let ahead = self.local.node_id.clone();
            let behind = remote.origin_node().clone();
            let catch_up: Vec<EpochId> =
                (remote_epoch.0 + 1..=local_epoch.0).map(EpochId).collect();
            if local_hash == remote_hash {
                return AlignmentRelation::FastForward {
                    ahead,
                    behind,
                    common_epoch: remote_epoch,
                    catch_up_epochs: catch_up,
                };
            }
            return AlignmentRelation::Forked {
                fork_epoch: remote_epoch,
                local_divergence: epoch_delta as f64 + 1.0,
                remote_divergence: epoch_delta as f64 + 0.0,
            };
        }

        {
            let ahead = remote.origin_node().clone();
            let behind = self.local.node_id.clone();
            let catch_up: Vec<EpochId> =
                (local_epoch.0 + 1..=remote_epoch.0).map(EpochId).collect();
            if local_hash == remote_hash {
                return AlignmentRelation::FastForward {
                    ahead,
                    behind,
                    common_epoch: local_epoch,
                    catch_up_epochs: catch_up,
                };
            }
            AlignmentRelation::Forked {
                fork_epoch: local_epoch,
                local_divergence: epoch_delta as f64 + 0.0,
                remote_divergence: epoch_delta as f64 + 1.0,
            }
        }
    }

    pub fn propose_alignment_for(
        &self,
        _divergence: &EpochDivergenceState,
        remote: &FederationPacket,
    ) -> Vec<AlignmentHint> {
        let relation = self.compute_alignment(remote);
        let local_id = self.local.node_id.clone();
        let remote_id = remote.origin_node().clone();

        match relation {
            AlignmentRelation::Identical => {
                vec![AlignmentHint::no_action(local_id)]
            }
            AlignmentRelation::FastForward { behind, .. } => {
                if behind == local_id {
                    vec![AlignmentHint::catch_up(local_id, remote_id)]
                } else {
                    vec![AlignmentHint::no_action(local_id)]
                }
            }
            AlignmentRelation::Forked { fork_epoch, .. } => {
                vec![AlignmentHint::rebase(local_id, fork_epoch, remote_id)]
            }
            AlignmentRelation::Incompatible { reason } => {
                vec![AlignmentHint {
                    target: local_id,
                    action: AlignmentAction::DeclareMisalignment(reason),
                }]
            }
        }
    }
}

impl EpochDivergenceState {
    pub fn detect_forks(&self) -> Vec<(NodeId, NodeId, f64)> {
        let nodes: Vec<&NodeId> = self.entries.keys().collect();
        let mut forks = Vec::new();
        for i in 0..nodes.len() {
            for j in (i + 1)..nodes.len() {
                if let Some(d) = self.divergence_between(nodes[i], nodes[j]) {
                    if d > 0.0 && d < f64::MAX {
                        forks.push((nodes[i].clone(), nodes[j].clone(), d));
                    }
                }
            }
        }
        forks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{CommitmentHash, NodeView};
    use vantage_core::{EpochId, NodeId, Signature};

    fn local_view(epoch: u64, hash_data: &str) -> NodeView {
        NodeView::new(
            NodeId("alice".into()),
            EpochId(epoch),
            CommitmentHash::from_bytes(hash_data.as_bytes()),
            CommitmentHash::genesis(),
        )
    }

    fn remote_packet(epoch: u64, hash_data: &str) -> FederationPacket {
        let origin = NodeView::new(
            NodeId("bob".into()),
            EpochId(epoch),
            CommitmentHash::from_bytes(hash_data.as_bytes()),
            CommitmentHash::genesis(),
        );
        FederationPacket::new(origin, std::collections::HashMap::new(), Signature(vec![]))
    }

    #[test]
    fn test_identical_alignment() {
        let view = FederationView::new(local_view(5, "same"));
        let packet = remote_packet(5, "same");
        let rel = view.compute_alignment(&packet);
        assert_eq!(rel, AlignmentRelation::Identical);
    }

    #[test]
    fn test_fast_forward_local_ahead() {
        let view = FederationView::new(local_view(10, "hash1010"));
        let packet = remote_packet(5, "hash1010");
        let rel = view.compute_alignment(&packet);
        match rel {
            AlignmentRelation::FastForward { ahead, behind, common_epoch, .. } => {
                assert_eq!(ahead.0, "alice");
                assert_eq!(behind.0, "bob");
                assert_eq!(common_epoch, EpochId(5));
            }
            other => panic!("expected FastForward, got {other:?}"),
        }
    }

    #[test]
    fn test_fast_forward_remote_ahead() {
        let view = FederationView::new(local_view(3, "hash"));
        let packet = remote_packet(7, "hash");
        let rel = view.compute_alignment(&packet);
        match rel {
            AlignmentRelation::FastForward { ahead, behind, common_epoch, .. } => {
                assert_eq!(ahead.0, "bob");
                assert_eq!(behind.0, "alice");
                assert_eq!(common_epoch, EpochId(3));
            }
            other => panic!("expected FastForward, got {other:?}"),
        }
    }

    #[test]
    fn test_genesis_mismatch() {
        let local = NodeView::new(
            NodeId("alice".into()),
            EpochId(5),
            CommitmentHash::from_bytes(b"alice-data"),
            CommitmentHash::genesis(),
        );
        let remote_origin = NodeView::new(
            NodeId("bob".into()),
            EpochId(5),
            CommitmentHash::from_bytes(b"bob-data"),
            CommitmentHash::from_bytes(b"other-genesis"),
        );
        let view = FederationView::new(local);
        let packet = FederationPacket::new(
            remote_origin,
            std::collections::HashMap::new(),
            Signature(vec![]),
        );
        let rel = view.compute_alignment(&packet);
        assert_eq!(
            rel,
            AlignmentRelation::Incompatible {
                reason: IncompatibilityReason::GenesisMismatch
            }
        );
    }

    #[test]
    fn test_epoch_collision_different_hash() {
        let view = FederationView::new(local_view(5, "alice-data"));
        let packet = remote_packet(5, "bob-data");
        let rel = view.compute_alignment(&packet);
        assert_eq!(
            rel,
            AlignmentRelation::Incompatible {
                reason: IncompatibilityReason::EpochCollision(EpochId(5))
            }
        );
    }

    #[test]
    fn test_forked_detection() {
        let view = FederationView::new(local_view(7, "alice-fork"));
        let packet = remote_packet(5, "bob-fork");
        let rel = view.compute_alignment(&packet);
        match rel {
            AlignmentRelation::Forked { fork_epoch, .. } => {
                assert_eq!(fork_epoch, EpochId(5));
            }
            other => panic!("expected Forked, got {other:?}"),
        }
    }

    #[test]
    fn test_divergence_state() {
        let views = vec![
            local_view(5, "alice"),
            NodeView::new(
                NodeId("bob".into()),
                EpochId(7),
                CommitmentHash::from_bytes(b"bob"),
                CommitmentHash::genesis(),
            ),
        ];
        let div = EpochDivergenceState::from_views(views);
        let d = div
            .divergence_between(&NodeId("alice".into()), &NodeId("bob".into()))
            .unwrap();
        assert!((d - 3.0).abs() < f64::EPSILON);
    }
}
