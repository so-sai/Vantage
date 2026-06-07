use sha2::Digest;
use std::collections::HashMap;
use vantage_core::{EpochId, NodeId};

use crate::predictive::PredictionCommitment;
use crate::state::CommitmentHash;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capital(pub u64);

impl Capital {
    pub fn zero() -> Self {
        Capital(0)
    }

    pub fn checked_add(self, other: Capital) -> Option<Capital> {
        self.0.checked_add(other.0).map(Capital)
    }

    pub fn checked_sub(self, other: Capital) -> Option<Capital> {
        self.0.checked_sub(other.0).map(Capital)
    }

    pub fn scale(&self, factor: f64) -> Capital {
        let scaled = (self.0 as f64 * factor).round() as u64;
        Capital(scaled)
    }
}

#[derive(Debug, Clone)]
pub struct ClaimToken {
    pub token_id: String,
    pub issuer: NodeId,
    pub stake: Capital,
    pub claim: PredictionCommitment,
    pub escrowed_at_epoch: EpochId,
}

impl ClaimToken {
    pub fn new(
        issuer: NodeId,
        stake: Capital,
        claim: PredictionCommitment,
        escrowed_at_epoch: EpochId,
    ) -> Self {
        let raw = format!(
            "{}:{}:{}:{}",
            issuer.0, stake.0, claim.target_epoch.0, escrowed_at_epoch.0
        );
        let hash = sha2::Sha256::digest(raw.as_bytes());
        let token_id = format!("ct:{:x}", u128::from_le_bytes(hash[..16].try_into().unwrap()));
        Self { token_id, issuer, stake, claim, escrowed_at_epoch }
    }
}

#[derive(Debug, Clone)]
pub struct MarketState {
    pub capital: HashMap<NodeId, Capital>,
    pub escrowed: HashMap<NodeId, Capital>,
    pub open_tokens: Vec<ClaimToken>,
    pub settled_tokens: Vec<SettledToken>,
    pub return_history: HashMap<NodeId, Vec<f64>>,
}

#[derive(Debug, Clone)]
pub struct SettledToken {
    pub token: ClaimToken,
    pub actual_hash: CommitmentHash,
    pub success: bool,
    pub payout: Capital,
}

impl MarketState {
    pub fn new() -> Self {
        Self {
            capital: HashMap::new(),
            escrowed: HashMap::new(),
            open_tokens: Vec::new(),
            settled_tokens: Vec::new(),
            return_history: HashMap::new(),
        }
    }

    pub fn initialize_capital(&mut self, node: NodeId, initial: Capital) {
        self.capital.entry(node.clone()).or_insert(initial);
        self.escrowed.entry(node.clone()).or_insert(Capital::zero());
        self.return_history.entry(node).or_default();
    }

    pub fn available_capital(&self, node: &NodeId) -> Capital {
        let total = self.capital.get(node).copied().unwrap_or(Capital::zero());
        let locked = self.escrowed.get(node).copied().unwrap_or(Capital::zero());
        total.checked_sub(locked).unwrap_or(Capital::zero())
    }

    pub fn issue_token(
        &mut self,
        issuer: NodeId,
        stake: Capital,
        claim: PredictionCommitment,
        current_epoch: EpochId,
    ) -> Result<ClaimToken, String> {
        if stake.0 == 0 {
            return Err("stake must be positive".into());
        }
        let available = self.available_capital(&issuer);
        if stake.0 > available.0 {
            return Err(format!(
                "insufficient capital: have {} available, need {}",
                available.0, stake.0
            ));
        }

        *self.escrowed.get_mut(&issuer).unwrap() = self.escrowed[&issuer]
            .checked_add(stake)
            .ok_or("escrow overflow")?;

        let token = ClaimToken::new(issuer, stake, claim, current_epoch);
        self.open_tokens.push(token.clone());
        Ok(token)
    }

    pub fn settle_epoch(
        &mut self,
        target_epoch: EpochId,
        actual_hash: &CommitmentHash,
    ) -> Vec<SettledToken> {
        let mut settled = Vec::new();
        let mut remaining = Vec::new();

        for token in self.open_tokens.drain(..) {
            if token.claim.target_epoch == target_epoch {
                let success = token.claim.claim_hash == *actual_hash;
                let multiplier = if success {
                    1.0 + token.claim.confidence
                } else {
                    0.0
                };
                let payout = token.stake.scale(multiplier);

                let issuer = token.issuer.clone();

                let locked = self.escrowed.get_mut(&issuer).unwrap();
                *locked = locked.checked_sub(token.stake).unwrap_or(Capital::zero());

                let total = self.capital.get_mut(&issuer).unwrap();
                if payout.0 >= token.stake.0 {
                    let profit = payout.checked_sub(token.stake).unwrap();
                    *total = total.checked_add(profit).unwrap_or(*total);
                } else {
                    let loss = token.stake.checked_sub(payout).unwrap();
                    *total = total.checked_sub(loss).unwrap_or(Capital::zero());
                }

                let period_return = if token.stake.0 > 0 {
                    (payout.0 as f64) / (token.stake.0 as f64) - 1.0
                } else {
                    0.0
                };
                self.return_history.get_mut(&issuer).unwrap().push(period_return);

                let st = SettledToken {
                    token: token.clone(),
                    actual_hash: actual_hash.clone(),
                    success,
                    payout,
                };
                settled.push(st.clone());
                self.settled_tokens.push(st);
            } else {
                remaining.push(token);
            }
        }

        self.open_tokens = remaining;
        settled
    }

    pub fn total_capital(&self, node: &NodeId) -> Capital {
        self.capital.get(node).copied().unwrap_or(Capital::zero())
    }

    pub fn sharpe_ratio(&self, node: &NodeId) -> f64 {
        let returns = match self.return_history.get(node) {
            Some(r) if r.len() >= 2 => r,
            _ => return 0.0,
        };

        let n = returns.len() as f64;
        let mean = returns.iter().sum::<f64>() / n;
        let variance = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1.0);
        let std_dev = variance.sqrt();

        if std_dev < 1e-12 {
            return if mean > 0.0 { 10.0 } else { 0.0 };
        }

        mean / std_dev
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::predictive::PredictionCommitment;
    use crate::state::CommitmentHash;

    fn test_hash(data: &str) -> CommitmentHash {
        CommitmentHash::from_bytes(data.as_bytes())
    }

    fn claim(epoch: u64, hash: &str, confidence: f64) -> PredictionCommitment {
        PredictionCommitment::new(EpochId(epoch), test_hash(hash), confidence)
    }

    #[test]
    fn test_capital_arithmetic() {
        let a = Capital(100);
        let b = Capital(50);
        assert_eq!(a.checked_add(b), Some(Capital(150)));
        assert_eq!(a.checked_sub(b), Some(Capital(50)));
        assert_eq!(Capital(5).checked_sub(Capital(10)), None);
    }

    #[test]
    fn test_available_capital_excludes_escrowed() {
        let mut market = MarketState::new();
        market.initialize_capital(NodeId("alice".into()), Capital(100));
        assert_eq!(market.available_capital(&NodeId("alice".into())), Capital(100));

        let c = claim(10, "r1", 0.7);
        market.issue_token(NodeId("alice".into()), Capital(30), c, EpochId(5)).unwrap();
        assert_eq!(market.available_capital(&NodeId("alice".into())), Capital(70));
    }

    #[test]
    fn test_issue_token_rejects_insufficient_capital() {
        let mut market = MarketState::new();
        market.initialize_capital(NodeId("alice".into()), Capital(10));
        let c = claim(10, "r1", 0.7);
        let result = market.issue_token(NodeId("alice".into()), Capital(20), c, EpochId(5));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("insufficient"));
    }

    #[test]
    fn test_settlement_correct_payout() {
        let mut market = MarketState::new();
        let alice = NodeId("alice".into());
        market.initialize_capital(alice.clone(), Capital(100));

        let c = claim(10, "r1", 0.5);
        market.issue_token(alice.clone(), Capital(20), c, EpochId(5)).unwrap();

        let settled = market.settle_epoch(EpochId(10), &test_hash("r1"));
        assert_eq!(settled.len(), 1);
        assert!(settled[0].success);
        // Payout = 20 * (1.0 + 0.5) = 30
        assert_eq!(settled[0].payout, Capital(30));

        // Total capital: 100 - 20 (escrowed) + 30 (payout) = 110
        assert_eq!(market.total_capital(&alice), Capital(110));
    }

    #[test]
    fn test_settlement_wrong_loses_stake() {
        let mut market = MarketState::new();
        let alice = NodeId("alice".into());
        market.initialize_capital(alice.clone(), Capital(100));

        let c = claim(10, "r1", 0.7);
        market.issue_token(alice.clone(), Capital(20), c, EpochId(5)).unwrap();

        let settled = market.settle_epoch(EpochId(10), &test_hash("r2"));
        assert_eq!(settled.len(), 1);
        assert!(!settled[0].success);
        // Payout = 20 * 0.0 = 0 (stake lost)
        assert_eq!(settled[0].payout, Capital(0));

        // Total capital: 100 - 20 (escrowed) + 0 (payout) = 80
        assert_eq!(market.total_capital(&alice), Capital(80));
    }

    #[test]
    fn test_multiple_tokens_different_epochs() {
        let mut market = MarketState::new();
        let alice = NodeId("alice".into());
        market.initialize_capital(alice.clone(), Capital(200));

        let c1 = claim(10, "r1", 0.5);
        let c2 = claim(20, "r2", 0.8);
        market.issue_token(alice.clone(), Capital(30), c1, EpochId(5)).unwrap();
        market.issue_token(alice.clone(), Capital(40), c2, EpochId(5)).unwrap();

        // Settle epoch 10 only
        let settled_10 = market.settle_epoch(EpochId(10), &test_hash("r1"));
        assert_eq!(settled_10.len(), 1);
        assert!(settled_10[0].success);

        // Epoch 20 token still open
        assert_eq!(market.open_tokens.len(), 1);
        assert_eq!(market.open_tokens[0].claim.target_epoch, EpochId(20));

        // Settle epoch 20
        let settled_20 = market.settle_epoch(EpochId(20), &test_hash("other"));
        assert_eq!(settled_20.len(), 1);
        assert!(!settled_20[0].success);

        // No open tokens remain
        assert!(market.open_tokens.is_empty());
    }

    #[test]
    fn test_bankruptcy_zero_capital() {
        let mut market = MarketState::new();
        let alice = NodeId("alice".into());
        market.initialize_capital(alice.clone(), Capital(10));

        let c = claim(10, "r1", 0.9);
        market.issue_token(alice.clone(), Capital(10), c, EpochId(5)).unwrap();
        assert_eq!(market.available_capital(&alice), Capital(0));

        // Can't issue more
        let c2 = claim(11, "r2", 0.5);
        let result = market.issue_token(alice.clone(), Capital(1), c2, EpochId(6));
        assert!(result.is_err());
    }

    #[test]
    fn test_sharpe_ratio() {
        let mut market = MarketState::new();
        let alice = NodeId("alice".into());
        market.initialize_capital(alice.clone(), Capital(100));

        // 3 correct predictions with varying confidence
        for i in 0..3 {
            let c = claim(10 + i, "r", 0.5);
            market.issue_token(alice.clone(), Capital(10), c, EpochId(i)).unwrap();
            market.settle_epoch(EpochId(10 + i), &test_hash("r"));
        }

        let sharpe = market.sharpe_ratio(&alice);
        assert!(sharpe > 0.0);

        // Bob with no history
        let bob = NodeId("bob".into());
        assert!((market.sharpe_ratio(&bob) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_capital_flow_between_nodes() {
        let mut market = MarketState::new();
        let alice = NodeId("alice".into());
        let bob = NodeId("bob".into());
        market.initialize_capital(alice.clone(), Capital(100));
        market.initialize_capital(bob.clone(), Capital(100));

        // Alice predicts correctly
        let ca = claim(10, "r1", 0.8);
        market.issue_token(alice.clone(), Capital(50), ca, EpochId(5)).unwrap();
        market.settle_epoch(EpochId(10), &test_hash("r1"));

        // Bob predicts wrongly
        let cb = claim(10, "r2", 0.8);
        market.issue_token(bob.clone(), Capital(50), cb, EpochId(5)).unwrap();
        market.settle_epoch(EpochId(10), &test_hash("r1"));

        // Alice: 100 - 50 + 50*1.8 = 140
        // Bob: 100 - 50 + 0 = 50
        assert_eq!(market.total_capital(&alice), Capital(140));
        assert_eq!(market.total_capital(&bob), Capital(50));

        // Capital has flowed from Bob to Alice via prediction accuracy
        let total = market.total_capital(&alice).0 + market.total_capital(&bob).0;
        assert_eq!(total, 190); // preserved (minus one loss)
    }
}
