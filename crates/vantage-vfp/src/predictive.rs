use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vantage_core::{EpochId, NodeId};

use crate::state::CommitmentHash;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionCommitment {
    pub target_epoch: EpochId,
    pub claim_hash: CommitmentHash,
    pub confidence: f64,
}

impl PredictionCommitment {
    pub fn new(target_epoch: EpochId, claim_hash: CommitmentHash, confidence: f64) -> Self {
        let confidence = confidence.clamp(0.0, 1.0);
        Self { target_epoch, claim_hash, confidence }
    }

    pub fn claim_id(&self, predictor: &NodeId) -> String {
        let mut hasher = Sha256::new();
        hasher.update(predictor.0.as_bytes());
        hasher.update(self.target_epoch.0.to_le_bytes());
        hasher.update(&self.claim_hash.0);
        let result = hasher.finalize();
        format!("claim:{:x}", u128::from_le_bytes(result[..16].try_into().unwrap()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealityClaim {
    pub predictor: NodeId,
    pub epoch: EpochId,
    pub commit_hash: CommitmentHash,
    pub predictions: Vec<PredictionCommitment>,
}

impl RealityClaim {
    pub fn new(
        predictor: NodeId,
        epoch: EpochId,
        commit_hash: CommitmentHash,
    ) -> Self {
        Self { predictor, epoch, commit_hash, predictions: Vec::new() }
    }

    pub fn with_prediction(mut self, prediction: PredictionCommitment) -> Self {
        self.predictions.push(prediction);
        self
    }

    pub fn claim_hash(&self) -> CommitmentHash {
        let mut hasher = Sha256::new();
        hasher.update(self.predictor.0.as_bytes());
        hasher.update(self.epoch.0.to_le_bytes());
        hasher.update(&self.commit_hash.0);
        for p in &self.predictions {
            hasher.update(p.target_epoch.0.to_le_bytes());
            hasher.update(&p.claim_hash.0);
            hasher.update(p.confidence.to_le_bytes());
        }
        let result = hasher.finalize();
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&result);
        CommitmentHash(arr)
    }
}

#[derive(Debug, Clone)]
pub enum SettlementResult {
    Correct,
    Incorrect { predicted: CommitmentHash, actual: CommitmentHash },
    NotYetSettled,
    TargetEpochUnknown,
}

#[derive(Debug, Clone)]
pub struct PredictionOutcome {
    pub predictor: NodeId,
    pub target_epoch: EpochId,
    pub claimed_hash: CommitmentHash,
    pub actual_hash: Option<CommitmentHash>,
    pub confidence: f64,
    pub success: bool,
    pub pnl: f64,
}

impl PredictionOutcome {
    pub fn settle(
        predictor: NodeId,
        claim: &PredictionCommitment,
        actual_hash: Option<CommitmentHash>,
    ) -> Self {
        let (success, pnl) = match &actual_hash {
            Some(actual) if *actual == claim.claim_hash => {
                // Correct prediction: gain proportional to confidence
                // Higher confidence → higher gain when right
                (true, claim.confidence)
            }
            Some(_actual) => {
                // Wrong prediction: lose confidence^2 (penalize overconfidence)
                (false, -(claim.confidence * claim.confidence))
            }
            None => {
                // Target epoch not yet settled: neutral
                (false, 0.0)
            }
        };

        Self {
            predictor,
            target_epoch: claim.target_epoch,
            claimed_hash: claim.claim_hash.clone(),
            actual_hash,
            confidence: claim.confidence,
            success,
            pnl,
        }
    }
}

impl SettlementResult {
    pub fn evaluate(
        claim: &PredictionCommitment,
        actual: Option<&CommitmentHash>,
    ) -> Self {
        match actual {
            Some(hash) if *hash == claim.claim_hash => SettlementResult::Correct,
            Some(hash) => SettlementResult::Incorrect {
                predicted: claim.claim_hash.clone(),
                actual: hash.clone(),
            },
            None => SettlementResult::NotYetSettled,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PredictionLedger {
    pub entries: Vec<PredictionOutcome>,
    pub trust_scores: Vec<(NodeId, f64)>,
}

impl PredictionLedger {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            trust_scores: Vec::new(),
        }
    }

    pub fn record_settlement(&mut self, outcome: PredictionOutcome) {
        self.entries.push(outcome.clone());
        self.update_trust(&outcome.predictor, outcome.pnl);
    }

    fn update_trust(&mut self, node: &NodeId, delta: f64) {
        for (id, score) in &mut self.trust_scores {
            if *id == *node {
                *score = (*score + delta).clamp(0.0, 100.0);
                return;
            }
        }
        let initial = (50.0 + delta).clamp(0.0, 100.0);
        self.trust_scores.push((node.clone(), initial));
    }

    pub fn trust_score(&self, node: &NodeId) -> f64 {
        self.trust_scores
            .iter()
            .find(|(id, _)| id == node)
            .map(|(_, score)| *score)
            .unwrap_or(50.0)
    }

    pub fn hit_rate(&self, node: &NodeId) -> f64 {
        let outcomes: Vec<&PredictionOutcome> = self
            .entries
            .iter()
            .filter(|o| o.predictor == *node && o.actual_hash.is_some())
            .collect();
        let total = outcomes.len();
        if total == 0 {
            return 0.5;
        }
        let hits = outcomes.iter().filter(|o| o.success).count();
        hits as f64 / total as f64
    }

    pub fn cumulative_pnl(&self, node: &NodeId) -> f64 {
        self.entries
            .iter()
            .filter(|o| o.predictor == *node)
            .map(|o| o.pnl)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vantage_core::NodeId;

    fn test_hash(data: &str) -> CommitmentHash {
        CommitmentHash::from_bytes(data.as_bytes())
    }

    #[test]
    fn test_prediction_commitment_confidence_clamped() {
        let high = PredictionCommitment::new(EpochId(10), test_hash("r1"), 1.5);
        let low = PredictionCommitment::new(EpochId(10), test_hash("r1"), -0.5);
        assert!((high.confidence - 1.0).abs() < 1e-9);
        assert!((low.confidence - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_claim_id_deterministic() {
        let node = NodeId("alice".into());
        let c1 = PredictionCommitment::new(EpochId(10), test_hash("r1"), 0.8);
        let c2 = PredictionCommitment::new(EpochId(10), test_hash("r1"), 0.8);
        assert_eq!(c1.claim_id(&node), c2.claim_id(&node));
    }

    #[test]
    fn test_settlement_correct() {
        let hash = test_hash("predicted-reality");
        let claim = PredictionCommitment::new(EpochId(10), hash.clone(), 0.7);
        let result = SettlementResult::evaluate(&claim, Some(&hash));
        assert!(matches!(result, SettlementResult::Correct));
    }

    #[test]
    fn test_settlement_incorrect() {
        let claim = PredictionCommitment::new(EpochId(10), test_hash("predicted"), 0.7);
        let result = SettlementResult::evaluate(&claim, Some(&test_hash("actual")));
        match result {
            SettlementResult::Incorrect { predicted, actual } => {
                assert_eq!(predicted, test_hash("predicted"));
                assert_eq!(actual, test_hash("actual"));
            }
            _ => panic!("expected Incorrect"),
        }
    }

    #[test]
    fn test_settlement_not_yet_settled() {
        let claim = PredictionCommitment::new(EpochId(10), test_hash("predicted"), 0.7);
        let result = SettlementResult::evaluate(&claim, None);
        assert!(matches!(result, SettlementResult::NotYetSettled));
    }

    #[test]
    fn test_prediction_outcome_pnl_correct() {
        let node = NodeId("alice".into());
        let claim = PredictionCommitment::new(EpochId(10), test_hash("r1"), 0.8);
        let outcome = PredictionOutcome::settle(node.clone(), &claim, Some(test_hash("r1")));
        assert!(outcome.success);
        assert!((outcome.pnl - 0.8).abs() < 1e-9);
    }

    #[test]
    fn test_prediction_outcome_pnl_wrong() {
        let node = NodeId("alice".into());
        let claim = PredictionCommitment::new(EpochId(10), test_hash("r1"), 0.8);
        let outcome = PredictionOutcome::settle(node.clone(), &claim, Some(test_hash("r2")));
        assert!(!outcome.success);
        assert!((outcome.pnl - (-0.64)).abs() < 1e-9);
    }

    #[test]
    fn test_prediction_outcome_pnl_not_settled() {
        let node = NodeId("alice".into());
        let claim = PredictionCommitment::new(EpochId(10), test_hash("r1"), 0.8);
        let outcome = PredictionOutcome::settle(node.clone(), &claim, None);
        assert!(!outcome.success);
        assert!((outcome.pnl - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_ledger_trust_update() {
        let mut ledger = PredictionLedger::new();
        let alice = NodeId("alice".into());

        // Correct prediction → trust increases from 50
        let claim = PredictionCommitment::new(EpochId(5), test_hash("r1"), 0.7);
        let outcome = PredictionOutcome::settle(alice.clone(), &claim, Some(test_hash("r1")));
        ledger.record_settlement(outcome);

        let trust = ledger.trust_score(&alice);
        assert!((trust - 50.7).abs() < 1e-9);

        // Wrong prediction → trust decreases
        let claim = PredictionCommitment::new(EpochId(6), test_hash("r2"), 0.5);
        let outcome = PredictionOutcome::settle(alice.clone(), &claim, Some(test_hash("r3")));
        ledger.record_settlement(outcome);

        let trust = ledger.trust_score(&alice);
        assert!((trust - 50.45).abs() < 1e-9);
    }

    #[test]
    fn test_ledger_hit_rate() {
        let mut ledger = PredictionLedger::new();
        let alice = NodeId("alice".into());

        let c1 = PredictionCommitment::new(EpochId(1), test_hash("r1"), 0.7);
        let c2 = PredictionCommitment::new(EpochId(2), test_hash("r2"), 0.7);
        let c3 = PredictionCommitment::new(EpochId(3), test_hash("wrong"), 0.7);

        ledger.record_settlement(PredictionOutcome::settle(
            alice.clone(), &c1, Some(test_hash("r1")),
        ));
        ledger.record_settlement(PredictionOutcome::settle(
            alice.clone(), &c2, Some(test_hash("r2")),
        ));
        ledger.record_settlement(PredictionOutcome::settle(
            alice.clone(), &c3, Some(test_hash("actual")),
        ));

        let rate = ledger.hit_rate(&alice);
        assert!((rate - 2.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_cumulative_pnl() {
        let mut ledger = PredictionLedger::new();
        let alice = NodeId("alice".into());
        let bob = NodeId("bob".into());

        let c1 = PredictionCommitment::new(EpochId(1), test_hash("r1"), 0.8);
        let c2 = PredictionCommitment::new(EpochId(2), test_hash("r2"), 0.6);

        ledger.record_settlement(PredictionOutcome::settle(
            alice.clone(), &c1, Some(test_hash("r1")),
        ));
        ledger.record_settlement(PredictionOutcome::settle(
            bob.clone(), &c2, Some(test_hash("other")),
        ));

        let alice_pnl = ledger.cumulative_pnl(&alice);
        let bob_pnl = ledger.cumulative_pnl(&bob);
        assert!((alice_pnl - 0.8).abs() < 1e-9);
        assert!((bob_pnl - (-0.36)).abs() < 1e-9);
    }

    #[test]
    fn test_reality_claim_hash_deterministic() {
        let node = NodeId("alice".into());
        let c1 = RealityClaim::new(node.clone(), EpochId(5), test_hash("h"))
            .with_prediction(PredictionCommitment::new(EpochId(6), test_hash("p"), 0.7));
        let c2 = RealityClaim::new(node, EpochId(5), test_hash("h"))
            .with_prediction(PredictionCommitment::new(EpochId(6), test_hash("p"), 0.7));
        assert_eq!(c1.claim_hash(), c2.claim_hash());
    }
}
