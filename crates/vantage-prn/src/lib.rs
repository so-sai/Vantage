pub mod attractor_monitor;
pub mod trust_dynamics;

pub use attractor_monitor::{AttractorMonitor, EpochSnapshot, PhaseState};
pub use trust_dynamics::TrustDynamics;

use std::collections::HashMap;
use vantage_core::{
    ElectionResult, EpochCandidateBundle, EpochId, EpochProposal, EpochQuorum,
    GlobalEpochAgreement, MetaElectionResult, NodeId,
};

pub struct ElectionEngine {
    node_id: NodeId,
    threshold: u32,
    weights: HashMap<NodeId, u64>,
}

impl ElectionEngine {
    pub fn new(node_id: NodeId, threshold: u32) -> Self {
        Self { node_id, threshold, weights: HashMap::new() }
    }

    #[allow(dead_code)]
    pub fn set_weight(&mut self, node: NodeId, weight: u64) {
        self.weights.insert(node, weight);
    }

    pub fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    /// Hard filter: reject proposals that violate system invariants.
    pub fn validate(&self, proposals: &[EpochProposal], current_epoch: EpochId) -> Vec<EpochProposal> {
        proposals.iter().filter(|p| {
            if p.epoch != EpochId(current_epoch.0 + 1) {
                return false;
            }
            if self.weights.get(&p.proposer).copied().unwrap_or(0) == 0 {
                return false;
            }
            if p.min_sequence == 0 {
                return false;
            }
            if p.cutoff_time.lamport == 0 {
                return false;
            }
            true
        }).cloned().collect()
    }

    /// Score a single validated proposal.
    pub fn score(&self, proposal: &EpochProposal) -> u64 {
        let trust = self.weights.get(&proposal.proposer).copied().unwrap_or(0);
        let sequence_progress = proposal.min_sequence.min(100);
        let temporal_freshness = proposal.cutoff_time.lamport.min(100);

        trust * 3 + sequence_progress * 2 + temporal_freshness * 1
    }

    /// Build a quorum for a given epoch from a set of proposals.
    fn build_quorum(&self, epoch: EpochId, policy_snapshot: u64, proposals: &[&EpochProposal]) -> Option<EpochQuorum> {
        let supporters: Vec<NodeId> = proposals.iter()
            .filter(|p| p.epoch == epoch && p.policy_snapshot == policy_snapshot)
            .map(|p| p.proposer.clone())
            .collect();

        if (supporters.len() as u32) < self.threshold {
            return None;
        }

        let aggregate_score: u64 = proposals.iter()
            .filter(|p| p.epoch == epoch && p.policy_snapshot == policy_snapshot)
            .map(|p| self.score(p))
            .sum();

        Some(EpochQuorum { epoch, supporters, aggregate_score, policy_snapshot })
    }

    /// Form a quorum from validated proposals.
    pub fn form_quorum(&self, proposals: &[EpochProposal]) -> ElectionResult {
        if proposals.is_empty() {
            return ElectionResult::NoConsensus;
        }

        let mut groups: HashMap<(EpochId, u64), Vec<&EpochProposal>> = HashMap::new();
        for p in proposals {
            groups.entry((p.epoch, p.policy_snapshot)).or_default().push(p);
        }

        let mut candidates: Vec<EpochQuorum> = Vec::new();
        for ((epoch, snapshot), group) in groups {
            if let Some(quorum) = self.build_quorum(epoch, snapshot, &group) {
                candidates.push(quorum);
            }
        }

        candidates.into_iter()
            .max_by_key(|q| q.aggregate_score)
            .map(ElectionResult::Candidate)
            .unwrap_or(ElectionResult::NoConsensus)
    }

    /// Full election round: validate -> score -> form quorum.
    pub fn run_election(&self, proposals: Vec<EpochProposal>, current_epoch: EpochId) -> ElectionResult {
        let valid = self.validate(&proposals, current_epoch);
        self.form_quorum(&valid)
    }
}

/// PRN-2: Meta-election kernel that reconciles divergent PRN-1 epoch candidates
/// across nodes into a single globally consistent epoch agreement.
///
/// Pipeline:
///   Incoming Bundles → Validation → Clustering → Scoring → MetaElectionResult
pub struct MetaElectionEngine {
    weights: HashMap<NodeId, u64>,
    quorum_threshold: u32,
    agreement_threshold: u32,
    penalty: u64,
}

impl MetaElectionEngine {
    pub fn new(quorum_threshold: u32, agreement_threshold: u32) -> Self {
        Self {
            weights: HashMap::new(),
            quorum_threshold,
            agreement_threshold,
            penalty: 1,
        }
    }

    #[allow(dead_code)]
    pub fn set_weight(&mut self, node: NodeId, weight: u64) {
        self.weights.insert(node, weight);
    }

    #[allow(dead_code)]
    pub fn get_weight(&self, node: &NodeId) -> u64 {
        self.weights.get(node).copied().unwrap_or(0)
    }

    /// Reduce trust for a node that submitted an invalid bundle.
    /// Returns the new trust weight.
    pub fn penalize_node(&mut self, node: &NodeId) -> u64 {
        let current = self.weights.get(node).copied().unwrap_or(0);
        let updated = current.saturating_sub(self.penalty);
        self.weights.insert(node.clone(), updated);
        updated
    }

    /// Recompute a quorum's structural score from local trust weights.
    /// Does NOT trust the bundle's aggregate_score — derives score from quorum composition.
    /// Uses ln(1 + trust) to apply sublinear influence scaling, preventing any single node
    /// or cartel from achieving exponential dominance.
    fn recompute_quorum_score(&self, quorum: &EpochQuorum) -> u64 {
        quorum.supporters.iter()
            .map(|n| {
                let w = self.weights.get(n).copied().unwrap_or(0);
                (1.0 + w as f64).ln().max(0.0) as u64
            })
            .sum()
    }

    /// Validate a received candidate bundle before accepting it into the meta-election buffer.
    /// All validation is derived from local state — nothing from the bundle is trusted as-is.
    pub fn validate_bundle(&self, bundle: &EpochCandidateBundle, current_epoch: EpochId) -> bool {
        if bundle.epoch != EpochId(current_epoch.0 + 1) {
            return false;
        }
        if bundle.quorum.epoch != bundle.epoch {
            return false;
        }
        if bundle.quorum.supporters.is_empty() {
            return false;
        }
        let submitting_weight = self.weights.get(&bundle.node_id).copied().unwrap_or(0);
        if submitting_weight == 0 {
            return false;
        }
        let recomputed = self.recompute_quorum_score(&bundle.quorum);
        if recomputed == 0 {
            return false;
        }
        if bundle.quorum.supporters.iter().all(|s| *s == bundle.node_id) {
            return false;
        }
        if bundle.quorum.supporters.iter().any(|s| self.weights.get(s).copied().unwrap_or(0) == 0) {
            return false;
        }
        true
    }

    /// Filter a list of bundles, keeping only valid ones.
    pub fn filter_valid_bundles(
        &self, bundles: Vec<EpochCandidateBundle>, current_epoch: EpochId,
    ) -> Vec<EpochCandidateBundle> {
        bundles.into_iter()
            .filter(|b| self.validate_bundle(b, current_epoch))
            .collect()
    }

    /// Score a cluster of bundles that share the same (epoch, policy_snapshot).
    /// Higher score = stronger global agreement.
    /// All scores are derived from local trust weights — no bundle-provided metrics are used.
    fn score_cluster(&self, bundles: &[&EpochCandidateBundle]) -> u64 {
        let sum_trust: u64 = bundles.iter()
            .map(|b| self.weights.get(&b.node_id).copied().unwrap_or(0))
            .sum();
        let max_structural_score = bundles.iter()
            .map(|b| self.recompute_quorum_score(&b.quorum))
            .max()
            .unwrap_or(0);
        let agreement_density = bundles.len() as u64;

        sum_trust * 3 + max_structural_score * 2 + agreement_density * 1
    }

    /// Run meta-election: cluster bundles by (epoch, policy_snapshot),
    /// score each cluster, and return the strongest agreement.
    pub fn run_meta_election(
        &self, bundles: Vec<EpochCandidateBundle>, current_epoch: EpochId,
    ) -> MetaElectionResult {
        let valid = self.filter_valid_bundles(bundles, current_epoch);

        if (valid.len() as u32) < self.quorum_threshold {
            return MetaElectionResult::NoConsensus;
        }

        let mut groups: HashMap<(EpochId, u64), Vec<&EpochCandidateBundle>> = HashMap::new();
        for b in &valid {
            groups.entry((b.epoch, b.quorum.policy_snapshot)).or_default().push(b);
        }

        let mut agreements: Vec<GlobalEpochAgreement> = Vec::new();
        for ((epoch, _snapshot), cluster) in groups {
            if (cluster.len() as u32) < self.agreement_threshold {
                continue;
            }
            let global_score = self.score_cluster(&cluster);
            let supporting_nodes: Vec<NodeId> = cluster.iter().map(|b| b.node_id.clone()).collect();
            let best_quorum = cluster.iter()
                .max_by_key(|b| self.recompute_quorum_score(&b.quorum))
                .map(|b| b.quorum.clone())
                .unwrap();

            agreements.push(GlobalEpochAgreement {
                epoch,
                quorum: best_quorum,
                supporting_nodes,
                global_score,
            });
        }

        agreements.into_iter()
            .max_by_key(|a| a.global_score)
            .map(MetaElectionResult::Agreement)
            .unwrap_or(MetaElectionResult::NoConsensus)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vantage_core::{EpochId, LogicalTime};

    fn proposal(epoch: u64, proposer: &str, weight: u64, seq: u64, time: u64) -> EpochProposal {
        EpochProposal {
            epoch: EpochId(epoch),
            policy_snapshot: 1,
            min_sequence: seq,
            cutoff_time: LogicalTime::new(time),
            proposer: NodeId(proposer.to_string()),
            trust_weight: weight,
        }
    }

    fn engine(threshold: u32) -> ElectionEngine {
        let mut e = ElectionEngine::new(NodeId("local".into()), threshold);
        e.set_weight(NodeId("alice".into()), 10);
        e.set_weight(NodeId("bob".into()), 8);
        e.set_weight(NodeId("carol".into()), 6);
        e
    }

    #[test]
    fn test_election_validation_rejects_wrong_epoch() {
        let eng = engine(2);
        let p = proposal(5, "alice", 10, 1, 1);
        let valid = eng.validate(&[p], EpochId(1));
        assert!(valid.is_empty(), "epoch 5 should be rejected when current is 1");
    }

    #[test]
    fn test_election_validation_passes_correct_epoch() {
        let eng = engine(2);
        let p = proposal(2, "alice", 10, 1, 1);
        let valid = eng.validate(&[p], EpochId(1));
        assert_eq!(valid.len(), 1);
    }

    #[test]
    fn test_election_validation_rejects_zero_trust() {
        let eng = engine(2);
        let p = proposal(2, "unknown", 0, 1, 1);
        let valid = eng.validate(&[p], EpochId(1));
        assert!(valid.is_empty());
    }

    #[test]
    fn test_election_quorum_simple_majority() {
        let eng = engine(2);
        let proposals = vec![
            proposal(2, "alice", 10, 5, 10),
            proposal(2, "bob", 8, 4, 9),
        ];
        let result = eng.run_election(proposals, EpochId(1));
        match result {
            ElectionResult::Candidate(q) => {
                assert_eq!(q.epoch, EpochId(2));
                assert!(q.supporters.len() >= 2);
            }
            _ => panic!("expected candidate"),
        }
    }

    #[test]
    fn test_election_quorum_below_threshold() {
        let eng = engine(3);
        let proposals = vec![
            proposal(2, "alice", 10, 5, 10),
            proposal(2, "bob", 8, 4, 9),
        ];
        let result = eng.run_election(proposals, EpochId(1));
        assert!(matches!(result, ElectionResult::NoConsensus));
    }

    #[test]
    fn test_election_empty_proposals() {
        let eng = engine(2);
        let result = eng.run_election(vec![], EpochId(1));
        assert!(matches!(result, ElectionResult::NoConsensus));
    }

    #[test]
    fn test_election_picks_highest_score() {
        let eng = engine(2);
        let proposals = vec![
            proposal(2, "alice", 10, 5, 10),
            proposal(2, "carol", 6, 2, 5),
            proposal(2, "bob", 8, 4, 9),
        ];
        let result = eng.run_election(proposals, EpochId(1));
        match result {
            ElectionResult::Candidate(q) => {
                assert_eq!(q.epoch, EpochId(2));
            }
            _ => panic!("expected candidate"),
        }
    }

    #[test]
    fn test_election_stale_proposal_rejected() {
        let eng = engine(2);
        let p = proposal(2, "alice", 10, 1, 0); // cutoff_time = 0
        let valid = eng.validate(&[p], EpochId(1));
        assert!(valid.is_empty());
    }

    // --- PRN-2 Meta-Election Tests ---

    fn bundle(node: &str, epoch: u64, _score: u64, supporters: Vec<&str>) -> EpochCandidateBundle {
        let node_id = NodeId(node.to_string());
        EpochCandidateBundle {
            node_id: node_id.clone(),
            epoch: EpochId(epoch),
            quorum: EpochQuorum {
                epoch: EpochId(epoch),
                supporters: supporters.into_iter().map(|s| NodeId(s.to_string())).collect(),
                aggregate_score: _score,
                policy_snapshot: 1,
            },
            aggregate_score: _score,
            signature: vantage_core::Signature(vec![]),
        }
    }

    fn meta_engine() -> MetaElectionEngine {
        let mut e = MetaElectionEngine::new(2, 2);
        e.set_weight(NodeId("alice".into()), 10);
        e.set_weight(NodeId("bob".into()), 8);
        e.set_weight(NodeId("carol".into()), 6);
        e
    }

    #[test]
    fn test_meta_election_valid_bundle_accepted() {
        let eng = meta_engine();
        let b = bundle("alice", 2, 75, vec!["alice", "bob"]);
        assert!(eng.validate_bundle(&b, EpochId(1)));
    }

    #[test]
    fn test_meta_election_wrong_epoch_rejected() {
        let eng = meta_engine();
        let b = bundle("alice", 5, 75, vec!["alice", "bob"]);
        assert!(!eng.validate_bundle(&b, EpochId(1)));
    }

    #[test]
    fn test_meta_election_unknown_node_rejected() {
        let eng = meta_engine();
        let b = bundle("unknown", 2, 75, vec!["unknown"]);
        assert!(!eng.validate_bundle(&b, EpochId(1)));
    }

    #[test]
    fn test_meta_election_below_quorum_threshold() {
        let eng = meta_engine();
        // Only 1 bundle when quorum_threshold=2 → NoConsensus
        let bundles = vec![bundle("alice", 2, 75, vec!["alice", "bob"])];
        let result = eng.run_meta_election(bundles, EpochId(1));
        assert!(matches!(result, MetaElectionResult::NoConsensus));
    }

    #[test]
    fn test_meta_election_agreement_formed() {
        let eng = meta_engine();
        let bundles = vec![
            bundle("alice", 2, 75, vec!["alice", "bob"]),
            bundle("bob", 2, 68, vec!["bob", "carol"]),
        ];
        let result = eng.run_meta_election(bundles, EpochId(1));
        match result {
            MetaElectionResult::Agreement(a) => {
                assert_eq!(a.epoch, EpochId(2));
                assert_eq!(a.supporting_nodes.len(), 2);
            }
            _ => panic!("expected agreement"),
        }
    }

    #[test]
    fn test_meta_election_picks_highest_global_score() {
        let eng = meta_engine();
        let bundles = vec![
            bundle("alice", 2, 75, vec!["alice", "bob"]),
            bundle("bob", 2, 68, vec!["bob", "carol"]),
            bundle("carol", 3, 90, vec!["carol", "alice"]), // different epoch, but valid supporters
        ];
        let result = eng.run_meta_election(bundles, EpochId(1));
        match result {
            MetaElectionResult::Agreement(a) => {
                assert_eq!(a.epoch, EpochId(2));
                // alice+bob cluster (epoch 2) should win over single carol bundle (epoch 3)
                assert!(a.global_score > 0);
            }
            _ => panic!("expected agreement"),
        }
    }

    #[test]
    fn test_meta_election_empty_bundles() {
        let eng = meta_engine();
        let result = eng.run_meta_election(vec![], EpochId(1));
        assert!(matches!(result, MetaElectionResult::NoConsensus));
    }

    #[test]
    fn test_meta_election_self_only_quorum_rejected() {
        let eng = meta_engine();
        // Supporters contain only the submitting node → rejected
        let b = bundle("alice", 2, 75, vec!["alice"]);
        assert!(!eng.validate_bundle(&b, EpochId(1)));
    }

    #[test]
    fn test_meta_election_recompute_rejects_zero_trust_supporters() {
        let eng = meta_engine();
        // Supporter "unknown" has no trust weight → recompute_quorum_score = 0 → rejected
        let b = bundle("alice", 2, 75, vec!["alice", "unknown"]);
        assert!(!eng.validate_bundle(&b, EpochId(1)));
    }

    #[test]
    fn test_meta_election_trust_decay_penalizes_node() {
        let mut eng = meta_engine();
        assert_eq!(eng.get_weight(&NodeId("alice".into())), 10);
        eng.penalize_node(&NodeId("alice".into()));
        assert_eq!(eng.get_weight(&NodeId("alice".into())), 9);
    }

    #[test]
    fn test_meta_election_recompute_score_ignores_bundle_claim() {
        let eng = meta_engine();
        // Bundle claims aggregate_score=9999, but meta-election recomputes from trust weights
        let b = bundle("alice", 2, 9999, vec!["alice", "bob"]);
        assert!(eng.validate_bundle(&b, EpochId(1))); // passes validation (recomputed=18 > 0)
        // Cluster scoring uses recomputed score, not the inflated claim
        let cluster_score = eng.score_cluster(&[&b]);
        // Cluster score: sum_trust(10)*3 + max_structural(ln11+ln9≈4)*2 + density(1)*1 = 39
        // Log-compressed vs 67 under linear — inflated claim is ignored
        assert_eq!(cluster_score, 39);
    }

    #[test]
    fn test_meta_election_validates_all_supporters_have_trust() {
        let eng = meta_engine();
        // Quorum contains bob+carol who both have trust > 0
        let b = bundle("bob", 2, 68, vec!["bob", "carol"]);
        assert!(eng.validate_bundle(&b, EpochId(1)));
    }
}
