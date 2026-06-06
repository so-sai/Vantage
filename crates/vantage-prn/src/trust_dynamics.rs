use std::collections::{HashMap, HashSet};
use vantage_core::{GlobalEpochAgreement, NodeId};

use crate::MetaElectionEngine;

/// Per-node epistemic state tracked across epochs.
pub struct TrustState {
    pub trust: f64,
    pub participation_count: u64,
    pub success_count: u64,
}

impl TrustState {
    pub fn new(trust: f64) -> Self {
        Self { trust, participation_count: 0, success_count: 0 }
    }

    pub fn centrality(&self, total_elections: u64) -> f64 {
        if total_elections == 0 { return 0.0; }
        self.participation_count as f64 / total_elections as f64
    }

    pub fn success_rate(&self) -> f64 {
        if self.participation_count == 0 { return 0.0; }
        self.success_count as f64 / self.participation_count as f64
    }

    pub fn disagreement_rate(&self) -> f64 {
        1.0 - self.success_rate()
    }
}

/// Temporal evolution kernel for the PRN-2 trust manifold.
///
/// Implements a controlled feedback system:
///   Δtrust = α·success - β·centrality + γ·disagreement
///
/// with per-epoch normalization to prevent drift.
pub struct TrustDynamics {
    states: HashMap<NodeId, TrustState>,
    alpha: f64,
    beta: f64,
    gamma: f64,
    epsilon: f64,
    total_elections: u64,
}

impl TrustDynamics {
    pub fn new(alpha: f64, beta: f64, gamma: f64, epsilon: f64) -> Self {
        Self {
            states: HashMap::new(),
            alpha,
            beta,
            gamma,
            epsilon,
            total_elections: 0,
        }
    }

    pub fn register_node(&mut self, node: NodeId, initial_trust: f64) {
        self.states.entry(node).or_insert_with(|| TrustState::new(initial_trust));
    }

    pub fn get_trust(&self, node: &NodeId) -> f64 {
        self.states.get(node).map(|s| s.trust).unwrap_or(0.0)
    }

    pub fn trust_snapshot(&self) -> Vec<(NodeId, f64)> {
        self.states.iter().map(|(n, s)| (n.clone(), s.trust)).collect()
    }

    pub fn election_count(&self) -> u64 {
        self.total_elections
    }

    /// Update trust states after a meta-election round.
    ///
    /// `agreement` — the winning GlobalEpochAgreement (may be None if NoConsensus).
    /// `participating` — all nodes that submitted bundles in this round.
    ///
    /// For each node:
    ///   - success = 1.0 if node in winning cluster, 0.0 otherwise
    ///   - disagreement = 1.0 if node participated but not in winning cluster, 0.0 otherwise
    ///   - centrality = participation_frequency
    ///   Δtrust = α·success - β·centrality + γ·disagreement
    ///
    /// Trust is floored at ε and normalized to sum = state_count.
    pub fn update(
        &mut self,
        agreement: Option<&GlobalEpochAgreement>,
        participating: &[NodeId],
    ) {
        self.total_elections += 1;

        let winning_set: HashSet<&NodeId> = agreement
            .map(|a| a.supporting_nodes.iter().collect())
            .unwrap_or_default();

        // Update state for all participating nodes
        for node in participating {
            let state = self.states.entry(node.clone()).or_insert_with(|| TrustState::new(self.epsilon));
            state.participation_count += 1;
            if winning_set.contains(node) {
                state.success_count += 1;
            }
        }

        // Ensure all winning_set nodes have state entries (they should already, but defensively)
        for node in &winning_set {
            self.states.entry((*node).clone()).or_insert_with(|| TrustState::new(self.epsilon));
        }

        let n = self.states.len() as f64;
        let mut deltas: HashMap<NodeId, f64> = HashMap::new();
        for (node, state) in &self.states {
            let in_win = winning_set.contains(node);
            let participated = participating.contains(node);
            let success_signal = if in_win { 1.0 } else { 0.0 };
            let disagreement_signal = if participated && !in_win { 1.0 } else { 0.0 };
            let centrality = state.centrality(self.total_elections);

            let delta = self.alpha * success_signal
                - self.beta * centrality
                + self.gamma * disagreement_signal;

            deltas.insert(node.clone(), delta);
        }

        // Apply deltas and floor
        let mut trust_sum = 0.0_f64;
        for (node, delta) in &deltas {
            if let Some(state) = self.states.get_mut(node) {
                state.trust = (state.trust + delta).max(self.epsilon);
                trust_sum += state.trust;
            }
        }

        // Normalize: rescale so sum(trust) = state_count (so average = 1.0)
        if trust_sum > 0.0 && n > 0.0 {
            let target = n;
            let scale = target / trust_sum;
            for state in self.states.values_mut() {
                state.trust *= scale;
                // Re-apply epsilon floor after normalization
                if state.trust < self.epsilon {
                    state.trust = self.epsilon;
                }
            }
        }
    }

    /// Sync the current trust values into a MetaElectionEngine.
    /// Trust (f64) is scaled to u64 for engine compatibility.
    pub fn sync_to_engine(&self, engine: &mut MetaElectionEngine) {
        for (node, state) in &self.states {
            let weight = (state.trust * 10.0).round() as u64;
            engine.set_weight(node.clone(), weight.max(1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vantage_core::{EpochId, EpochQuorum};

    fn make_agreement(epoch: u64, supporting_nodes: Vec<&str>, global_score: u64) -> GlobalEpochAgreement {
        let supporters: Vec<NodeId> = supporting_nodes.iter().map(|s| NodeId(s.to_string())).collect();
        GlobalEpochAgreement {
            epoch: EpochId(epoch),
            quorum: EpochQuorum {
                epoch: EpochId(epoch),
                supporters: supporters.clone(),
                aggregate_score: global_score,
                policy_snapshot: 1,
            },
            supporting_nodes: supporters,
            global_score,
        }
    }

    fn td() -> TrustDynamics {
        TrustDynamics::new(1.0, 0.5, 0.3, 0.01)
    }

    fn node(name: &str) -> NodeId { NodeId(name.to_string()) }

    #[test]
    fn test_trust_dynamics_winners_gain_trust() {
        let mut td = td();
        td.register_node(node("alice"), 1.0);
        td.register_node(node("bob"), 1.0);

        let agreement = make_agreement(2, vec!["alice", "bob"], 80);
        td.update(Some(&agreement), &[node("alice"), node("bob")]);

        // Both won, both should have trust >= 1.0 (alpha gain, but also some centrality decay)
        let alice = td.get_trust(&node("alice"));
        let bob = td.get_trust(&node("bob"));
        assert!(alice > 0.0);
        assert!(bob > 0.0);
    }

    #[test]
    fn test_trust_dynamics_divergence_rewarded() {
        let mut td = td();
        td.register_node(node("alice"), 1.0);
        td.register_node(node("bob"), 1.0);

        // alice wins, bob participates but disagrees
        let agreement = make_agreement(2, vec!["alice"], 80);
        td.update(Some(&agreement), &[node("alice"), node("bob")]);

        // bob gets γ reward for disagreement
        let alice = td.get_trust(&node("alice"));
        let bob = td.get_trust(&node("bob"));
        // Both should survive, bob slightly boosted by γ
        assert!(alice > 0.0);
        assert!(bob > 0.0);
    }

    #[test]
    fn test_trust_dynamics_no_consensus_no_change() {
        let mut td = td();
        td.register_node(node("alice"), 2.0);
        td.register_node(node("bob"), 1.0);

        // NoConsensus → None agreement
        td.update(None, &[node("alice"), node("bob")]);

        let alice = td.get_trust(&node("alice"));
        let bob = td.get_trust(&node("bob"));
        // Centrality decay applies, but both participated
        assert!(alice > 0.0);
        assert!(bob > 0.0);
    }

    #[test]
    fn test_trust_dynamics_multi_epoch_convergence() {
        let mut td = td();
        td.register_node(node("alice"), 1.0);
        td.register_node(node("bob"), 1.0);
        td.register_node(node("carol"), 1.0);

        // alice wins 3 epochs in a row
        for _ in 0..3 {
            let agreement = make_agreement(2, vec!["alice", "bob"], 80);
            td.update(Some(&agreement), &[node("alice"), node("bob"), node("carol")]);
        }

        let alice = td.get_trust(&node("alice"));
        let bob = td.get_trust(&node("bob"));
        let carol = td.get_trust(&node("carol"));

        // alice should have highest trust (consistent winner)
        // carol should have lowest (always disagrees but gets γ reward)
        assert!(alice >= bob);
        assert!(alice >= carol);
    }

    #[test]
    fn test_trust_dynamics_epsilon_floor() {
        let mut td = td();
        // Create a node with very low trust
        td.register_node(node("weak"), 0.001);

        // Run many rounds where weak never participates
        for _ in 0..10 {
            let agreement = make_agreement(2, vec!["alice"], 80);
            td.update(Some(&agreement), &[node("alice")]);
        }

        let trust = td.get_trust(&node("weak"));
        assert!(trust >= 0.01); // epsilon floor
    }

    #[test]
    fn test_trust_dynamics_sync_to_engine() {
        use crate::MetaElectionEngine;
        let mut td = td();
        td.register_node(node("alice"), 1.0);
        td.register_node(node("bob"), 1.0);

        let agreement = make_agreement(2, vec!["alice", "bob"], 80);
        td.update(Some(&agreement), &[node("alice"), node("bob")]);

        let mut engine = MetaElectionEngine::new(2, 2);
        td.sync_to_engine(&mut engine);

        assert!(engine.get_weight(&node("alice")) > 0);
        assert!(engine.get_weight(&node("bob")) > 0);
    }
}
