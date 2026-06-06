use std::collections::HashMap;
use vantage_core::NodeId;

/// Classification of the current epistemic regime.
#[derive(Debug, Clone, PartialEq)]
pub enum PhaseState {
    /// Not enough data to classify.
    Forming,
    /// Trust is concentrated in a stable cluster — same nodes win consistently.
    Convergence { dominant_nodes: Vec<NodeId>, concentration: f64 },
    /// No stable winner — different clusters alternate across epochs.
    Oscillation { period: u64, alternation_rate: f64 },
    /// Trust is uniformly distributed — no node has meaningful influence.
    Collapse { entropy: f64 },
}

/// An epoch-level snapshot of the epistemic field state.
#[derive(Debug, Clone)]
pub struct EpochSnapshot {
    pub epoch: u64,
    pub trust_values: HashMap<NodeId, f64>,
    pub winning_nodes: Vec<NodeId>,
}

/// Observes TrustDynamics state over a sliding window and classifies
/// the current phase regime of the epistemic field.
pub struct AttractorMonitor {
    window_size: usize,
    history: Vec<EpochSnapshot>,
}

impl AttractorMonitor {
    pub fn new(window_size: usize) -> Self {
        Self { window_size, history: Vec::new() }
    }

    /// Record a new epoch snapshot. Maintains a sliding window of `window_size`.
    pub fn record(&mut self, snapshot: EpochSnapshot) {
        self.history.push(snapshot);
        if self.history.len() > self.window_size {
            self.history.remove(0);
        }
    }

    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    /// Classify the current regime based on the recorded window.
    pub fn classify(&self) -> PhaseState {
        if self.history.len() < self.window_size {
            return PhaseState::Forming;
        }

        let entropy = self.compute_entropy();
        if entropy > 0.85 {
            return PhaseState::Collapse { entropy };
        }

        let (dominant, stability) = self.winner_stability();
        if stability >= 0.7 {
            return PhaseState::Convergence { dominant_nodes: dominant, concentration: stability };
        }

        let (period, alternation) = self.oscillation_score();
        PhaseState::Oscillation { period, alternation_rate: alternation }
    }

    /// Shannon entropy of the latest trust distribution, normalized to [0, 1].
    fn compute_entropy(&self) -> f64 {
        let latest = match self.history.last() {
            Some(s) => s,
            None => return 0.0,
        };

        let total: f64 = latest.trust_values.values().sum();
        if total <= 0.0 {
            return 0.0;
        }

        let n = latest.trust_values.len();
        if n <= 1 {
            return 0.0;
        }

        let h: f64 = latest.trust_values.values()
            .map(|t| {
                let p = t / total;
                if p <= 0.0 { 0.0 } else { -p * p.log2() }
            })
            .sum();

        h / (n as f64).log2()
    }

    /// Identify the most frequent winning cluster and its stability score.
    fn winner_stability(&self) -> (Vec<NodeId>, f64) {
        let mut cluster_counts: HashMap<Vec<NodeId>, usize> = HashMap::new();
        for snap in &self.history {
            let mut key = snap.winning_nodes.clone();
            key.sort();
            *cluster_counts.entry(key).or_insert(0) += 1;
        }

        let total = self.history.len() as f64;
        let (best_cluster, count) = cluster_counts.into_iter()
            .max_by_key(|(_, c)| *c)
            .unwrap_or((vec![], 0));

        let stability = count as f64 / total;
        (best_cluster, stability)
    }

    /// Estimate oscillation period and alternation rate from winner changes.
    fn oscillation_score(&self) -> (u64, f64) {
        if self.history.len() < 2 {
            return (0, 0.0);
        }

        let mut transitions = 0u64;
        let mut prev_cluster: Option<Vec<NodeId>> = None;
        for snap in &self.history {
            let mut key = snap.winning_nodes.clone();
            key.sort();
            if let Some(ref prev) = prev_cluster {
                if *prev != key {
                    transitions += 1;
                }
            }
            prev_cluster = Some(key);
        }

        let total_windows = (self.history.len() - 1) as f64;
        let alternation = if total_windows > 0.0 {
            transitions as f64 / total_windows
        } else {
            0.0
        };

        // Estimate period: average epochs between alternations
        let period = if transitions > 0 {
            self.history.len() as u64 / transitions
        } else {
            self.history.len() as u64
        };

        (period, alternation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(epoch: u64, trust: Vec<(&str, f64)>, winners: Vec<&str>) -> EpochSnapshot {
        EpochSnapshot {
            epoch,
            trust_values: trust.into_iter().map(|(n, t)| (NodeId(n.to_string()), t)).collect(),
            winning_nodes: winners.into_iter().map(|n| NodeId(n.to_string())).collect(),
        }
    }

    fn monitor(window: usize, snaps: Vec<EpochSnapshot>) -> AttractorMonitor {
        let mut m = AttractorMonitor::new(window);
        for s in snaps { m.record(s); }
        m
    }

    #[test]
    fn test_monitor_forming_before_window_full() {
        let m = monitor(5, vec![snap(1, vec![("a", 1.0)], vec!["a"])]);
        assert_eq!(m.classify(), PhaseState::Forming);
    }

    #[test]
    fn test_monitor_collapse_uniform_trust() {
        let snaps = (0..5).map(|e| {
            snap(e, vec![("a", 1.0), ("b", 1.0), ("c", 1.0)], vec!["a"])
        }).collect();
        let m = monitor(3, snaps);
        assert!(matches!(m.classify(), PhaseState::Collapse { .. }));
    }

    #[test]
    fn test_monitor_convergence_stable_winner() {
        let snaps = (0..5).map(|e| {
            snap(e, vec![("a", 10.0), ("b", 2.0), ("c", 1.0)], vec!["a"])
        }).collect();
        let m = monitor(3, snaps);
        match m.classify() {
            PhaseState::Convergence { dominant_nodes, .. } => {
                assert!(dominant_nodes.contains(&NodeId("a".to_string())));
            }
            other => panic!("expected Convergence, got {:?}", other),
        }
    }

    #[test]
    fn test_monitor_oscillation_alternating_winners() {
        // Differentiated trust (entropy < 0.85) but winner alternates
        let snaps = vec![
            snap(1, vec![("a", 8.0), ("b", 2.0)], vec!["a"]),
            snap(2, vec![("a", 8.0), ("b", 2.0)], vec!["b"]),
            snap(3, vec![("a", 8.0), ("b", 2.0)], vec!["a"]),
            snap(4, vec![("a", 8.0), ("b", 2.0)], vec!["b"]),
        ];
        let m = monitor(3, snaps);
        assert!(matches!(m.classify(), PhaseState::Oscillation { .. }));
    }

    #[test]
    fn test_monitor_records_up_to_window() {
        let mut m = AttractorMonitor::new(3);
        for e in 0..10 {
            m.record(snap(e, vec![("a", 1.0)], vec!["a"]));
        }
        assert_eq!(m.history_len(), 3);
    }
}
