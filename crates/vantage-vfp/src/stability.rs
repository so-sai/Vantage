use vantage_core::NodeId;

use crate::closure::{ClosureGraph, FederationState};
use crate::AlignmentRelation;

const EPSILON: f64 = 1e-9;
const STABILITY_WINDOW: usize = 3;

#[derive(Debug, Clone)]
pub struct DivergenceEntropy {
    pub identical: f64,
    pub fast_forward: f64,
    pub forked: f64,
    pub incompatible: f64,
    pub shannon: f64,
    pub normalized: f64,
}

impl DivergenceEntropy {
    pub fn from_closure(closure: &[(NodeId, NodeId, AlignmentRelation)]) -> Self {
        let total = closure.len() as f64;
        if total == 0.0 {
            return Self {
                identical: 0.0,
                fast_forward: 0.0,
                forked: 0.0,
                incompatible: 0.0,
                shannon: 0.0,
                normalized: 0.0,
            };
        }

        let mut counts = [0u64; 4];
        for (_, _, r) in closure {
            match r {
                AlignmentRelation::Identical => counts[0] += 1,
                AlignmentRelation::FastForward { .. } => counts[1] += 1,
                AlignmentRelation::Forked { .. } => counts[2] += 1,
                AlignmentRelation::Incompatible { .. } => counts[3] += 1,
            }
        }

        let probs: Vec<f64> = counts.iter().map(|c| *c as f64 / total).collect();
        let shannon = -probs
            .iter()
            .filter(|p| **p > EPSILON)
            .map(|p| p * p.ln())
            .sum::<f64>();

        Self {
            identical: probs[0],
            fast_forward: probs[1],
            forked: probs[2],
            incompatible: probs[3],
            shannon,
            normalized: if total > 1.0 { shannon / (4.0f64).ln() } else { 0.0 },
        }
    }
}

#[derive(Debug, Clone)]
pub struct LyapunovState {
    pub entropy: DivergenceEntropy,
    pub cluster_count: usize,
    pub incompatible_edge_count: usize,
    pub total_nodes: usize,
    pub V: f64,
}

impl LyapunovState {
    pub fn from_graph(graph: &ClosureGraph) -> Self {
        let closure = graph.compute_closure();
        let state = graph.classify();
        let entropy = DivergenceEntropy::from_closure(&closure);

        let incompatible_edge_count = closure
            .iter()
            .filter(|(_, _, r)| matches!(r, AlignmentRelation::Incompatible { .. }))
            .count();

        let cluster_count = match &state {
            FederationState::Partitioned { clusters, .. } => clusters.len(),
            _ => 1,
        };

        let total_nodes = graph.node_count();

        let w_entropy = 1.0;
        let w_incompatible = 10.0;
        let w_cluster = 3.0;

        let norm_incompatible = if total_nodes > 1 {
            incompatible_edge_count as f64 / (total_nodes * (total_nodes - 1) / 2) as f64
        } else {
            0.0
        };

        let norm_clusters = if total_nodes > 1 {
            (cluster_count as f64 - 1.0) / (total_nodes as f64 - 1.0)
        } else {
            0.0
        };

        let V = w_entropy * entropy.normalized
            + w_incompatible * norm_incompatible
            + w_cluster * norm_clusters;

        Self {
            entropy,
            cluster_count,
            incompatible_edge_count,
            total_nodes,
            V,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StabilityCondition {
    Convergent {
        delta_V: f64,
        steps_remaining_estimate: u32,
    },
    Oscillatory {
        amplitude: f64,
        period_estimate: u32,
    },
    Fragmented {
        entropy: f64,
        clusters: usize,
    },
    Collapsed,
    Undetermined,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Attractor {
    Convergent { V_asymptotic: f64 },
    Oscillatory { center: f64, amplitude: f64 },
    Fragmented { clusters: usize, entropy: f64 },
    Collapse,
}

pub struct PhaseDynamics {
    history: Vec<LyapunovState>,
    attractor: Option<Attractor>,
}

impl PhaseDynamics {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            attractor: None,
        }
    }

    pub fn record(&mut self, state: LyapunovState) {
        self.history.push(state);
    }

    pub fn len(&self) -> usize {
        self.history.len()
    }

    pub fn current(&self) -> Option<&LyapunovState> {
        self.history.last()
    }

    pub fn attractor(&self) -> Option<&Attractor> {
        self.attractor.as_ref()
    }

    pub fn classify_stability(&self) -> StabilityCondition {
        let n = self.history.len();
        if n < 2 {
            return StabilityCondition::Undetermined;
        }

        let current = &self.history[n - 1];

        if current.incompatible_edge_count > 0 {
            return StabilityCondition::Collapsed;
        }

        if n < STABILITY_WINDOW + 1 {
            return StabilityCondition::Undetermined;
        }

        let deltas: Vec<f64> = (n - STABILITY_WINDOW..n)
            .map(|i| self.history[i].V - self.history[i - 1].V)
            .collect();

        let mean_delta = deltas.iter().sum::<f64>() / deltas.len() as f64;
        let all_zero = deltas.iter().all(|d| d.abs() < EPSILON);

        if mean_delta < -EPSILON {
            let abs_mean = mean_delta.abs();
            let steps_remaining = if abs_mean > EPSILON {
                (current.V / abs_mean).ceil() as u32
            } else {
                0
            };
            StabilityCondition::Convergent {
                delta_V: mean_delta,
                steps_remaining_estimate: steps_remaining,
            }
        } else if all_zero {
            if current.V < 0.1 && current.cluster_count <= 1 {
                StabilityCondition::Convergent {
                    delta_V: 0.0,
                    steps_remaining_estimate: 0,
                }
            } else if current.cluster_count > 1 && current.entropy.normalized > 0.5 {
                StabilityCondition::Fragmented {
                    entropy: current.entropy.normalized,
                    clusters: current.cluster_count,
                }
            } else {
                StabilityCondition::Undetermined
            }
        } else {
            if current.cluster_count > 1 && current.entropy.normalized > 0.5 {
                StabilityCondition::Fragmented {
                    entropy: current.entropy.normalized,
                    clusters: current.cluster_count,
                }
            } else {
                StabilityCondition::Undetermined
            }
        }
    }

    pub fn update_attractor(&mut self) -> Option<Attractor> {
        let n = self.history.len();
        if n < 3 {
            return None;
        }

        let current = &self.history[n - 1];

        if current.incompatible_edge_count > 0 {
            self.attractor = Some(Attractor::Collapse);
            return self.attractor.clone();
        }

        let recent: Vec<f64> = self.history[(n - 3).max(0)..]
            .iter()
            .map(|s| s.V)
            .collect();

        let mean_V = recent.iter().sum::<f64>() / recent.len() as f64;
        let variance = recent
            .iter()
            .map(|v| (v - mean_V).powi(2))
            .sum::<f64>()
            / recent.len() as f64;
        let std_V = variance.sqrt();

        let cluster_count = current.cluster_count;
        let entropy = current.entropy.normalized;

        if std_V < EPSILON && current.cluster_count == 1 && entropy < 0.3 {
            self.attractor = Some(Attractor::Convergent {
                V_asymptotic: mean_V,
            });
        } else if std_V > EPSILON && variance < 0.1 {
            let amplitude = recent
                .iter()
                .map(|v| (v - mean_V).abs())
                .sum::<f64>()
                / recent.len() as f64;
            self.attractor = Some(Attractor::Oscillatory {
                center: mean_V,
                amplitude,
            });
        } else if cluster_count > 1 && entropy > 0.5 {
            self.attractor = Some(Attractor::Fragmented {
                clusters: cluster_count,
                entropy,
            });
        }

        self.attractor.clone()
    }
}

pub fn compute_lyapunov_derivative(prev: &LyapunovState, curr: &LyapunovState) -> f64 {
    curr.V - prev.V
}

pub fn check_stability_theorem(states: &[LyapunovState]) -> Option<String> {
    let n = states.len();
    if n < 2 {
        return None;
    }

    let monotonic_decreasing = states.windows(2).all(|w| w[1].V <= w[0].V + EPSILON);
    let strictly_decreasing = states.windows(2).all(|w| w[1].V < w[0].V - EPSILON);

    let no_incompatible = states.iter().all(|s| s.incompatible_edge_count == 0);
    let convergent = states
        .last()
        .map(|s| s.cluster_count == 1 && s.entropy.normalized < 0.3)
        .unwrap_or(false);

    if monotonic_decreasing && no_incompatible {
        if convergent {
            Some("Theorem 1: V is a Lyapunov function — system converges to Stable attractor".into())
        } else if strictly_decreasing {
            Some(
                "Theorem 2: V strictly decreases — system approaches Divergent or Partitioned boundary"
                    .into(),
            )
        } else {
            Some("Theorem 3: V non-increasing — system is bounded but may be oscillatory".into())
        }
    } else if no_incompatible {
        Some(
            "Theorem 4: No catastrophic edges — system is partitionable even if not convergent"
                .into(),
        )
    } else {
        Some(
            "Theorem 5: Incompatible edges detected — system is collapsed under closure algebra"
                .into(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::closure::ClosureGraph;
    use vantage_core::{EpochId, NodeId};

    fn make_graph(
        pairs: Vec<(&str, &str, AlignmentRelation)>,
    ) -> ClosureGraph {
        let mut g = ClosureGraph::new();
        for (a, b, r) in pairs {
            g.add_relation(NodeId(a.into()), NodeId(b.into()), r);
        }
        g
    }

    fn ff(ahead: &str, behind: &str) -> AlignmentRelation {
        AlignmentRelation::FastForward {
            ahead: NodeId(ahead.into()),
            behind: NodeId(behind.into()),
            common_epoch: EpochId(5),
            catch_up_epochs: vec![],
        }
    }

    fn fork(epoch: u64) -> AlignmentRelation {
        AlignmentRelation::Forked {
            fork_epoch: EpochId(epoch),
            local_divergence: 1.0,
            remote_divergence: 1.0,
        }
    }

    fn inc() -> AlignmentRelation {
        AlignmentRelation::Incompatible {
            reason: crate::IncompatibilityReason::GenesisMismatch,
        }
    }

    #[test]
    fn test_entropy_stable() {
        let g = make_graph(vec![("a", "b", ff("b", "a")), ("b", "c", ff("c", "b"))]);
        let closure = g.compute_closure();
        let entropy = DivergenceEntropy::from_closure(&closure);
        assert!(entropy.normalized < 0.5);
        assert!(entropy.fast_forward > 0.5);
    }

    #[test]
    fn test_entropy_forked() {
        let g = make_graph(vec![("a", "b", fork(3)), ("a", "c", fork(2))]);
        let closure = g.compute_closure();
        let entropy = DivergenceEntropy::from_closure(&closure);
        assert!(entropy.forked > 0.0);
    }

    #[test]
    fn test_entropy_empty() {
        let entropy = DivergenceEntropy::from_closure(&[]);
        assert!(entropy.shannon.abs() < EPSILON);
    }

    #[test]
    fn test_lyapunov_convergent_system() {
        // Same graph recorded twice — V should be identical
        let g = make_graph(vec![
            ("a", "b", ff("b", "a")),
            ("b", "c", ff("c", "b")),
        ]);
        let l1 = LyapunovState::from_graph(&g);
        let l2 = LyapunovState::from_graph(&g);
        let deriv = compute_lyapunov_derivative(&l1, &l2);
        assert!(deriv.abs() < EPSILON);
    }

    #[test]
    fn test_lyapunov_collapsed_spikes() {
        let g = make_graph(vec![("a", "b", inc())]);
        let l = LyapunovState::from_graph(&g);
        assert!(l.incompatible_edge_count > 0);
        assert!(l.V > 5.0);
    }

    #[test]
    fn test_stability_classification_convergent() {
        let mut dynamics = PhaseDynamics::new();
        let g = make_graph(vec![
            ("a", "b", ff("b", "a")),
            ("b", "c", ff("c", "b")),
            ("c", "d", ff("d", "c")),
            ("a", "d", ff("d", "a")),
        ]);
        // Record same stable state 5 times (STABILITY_WINDOW + 2)
        for _ in 0..5 {
            dynamics.record(LyapunovState::from_graph(&g));
        }
        let stability = dynamics.classify_stability();
        assert!(matches!(stability, StabilityCondition::Convergent { .. }));
    }

    #[test]
    fn test_stability_classification_collapsed() {
        let mut dynamics = PhaseDynamics::new();
        let g1 = make_graph(vec![("a", "b", ff("b", "a"))]);
        let g2 = make_graph(vec![("a", "b", inc())]);
        dynamics.record(LyapunovState::from_graph(&g1));
        dynamics.record(LyapunovState::from_graph(&g2));
        let stability = dynamics.classify_stability();
        assert_eq!(stability, StabilityCondition::Collapsed);
    }

    #[test]
    fn test_attractor_detection_convergent() {
        let mut dynamics = PhaseDynamics::new();
        let g = make_graph(vec![
            ("a", "b", ff("b", "a")),
            ("b", "c", ff("c", "b")),
            ("c", "d", ff("d", "c")),
            ("a", "d", ff("d", "a")),
        ]);
        dynamics.record(LyapunovState::from_graph(&g));
        dynamics.record(LyapunovState::from_graph(&g));
        dynamics.record(LyapunovState::from_graph(&g));

        let attr = dynamics.update_attractor();
        assert!(matches!(attr, Some(Attractor::Convergent { .. })));
    }

    #[test]
    fn test_stability_theorem_statements() {
        let states = vec![
            LyapunovState {
                entropy: DivergenceEntropy {
                    identical: 1.0,
                    fast_forward: 0.0,
                    forked: 0.0,
                    incompatible: 0.0,
                    shannon: 0.0,
                    normalized: 0.0,
                },
                cluster_count: 1,
                incompatible_edge_count: 0,
                total_nodes: 2,
                V: 0.0,
            },
            LyapunovState {
                entropy: DivergenceEntropy {
                    identical: 1.0,
                    fast_forward: 0.0,
                    forked: 0.0,
                    incompatible: 0.0,
                    shannon: 0.0,
                    normalized: 0.0,
                },
                cluster_count: 1,
                incompatible_edge_count: 0,
                total_nodes: 2,
                V: 0.0,
            },
        ];
        let theorem = check_stability_theorem(&states);
        assert!(theorem.is_some());
        assert!(theorem.unwrap().contains("Theorem 1"));
    }

    #[test]
    fn test_fragmented_entropy_measurable() {
        // Forked pairs produce measurable forked entropy
        let g = make_graph(vec![
            ("a", "b", fork(3)),
            ("c", "d", fork(2)),
        ]);
        let state = LyapunovState::from_graph(&g);
        // Two forked pairs out of 2 closure entries → 100% forked → entropy = 0
        assert!(state.entropy.forked > 0.0);
        // Only forked relations → shannon entropy = 0 (all same class)
        assert!(state.entropy.shannon.abs() < EPSILON);
    }
}
