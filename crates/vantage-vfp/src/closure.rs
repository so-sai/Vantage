use std::collections::{HashMap, HashSet, VecDeque};
use vantage_core::{EpochId, NodeId};

use crate::AlignmentRelation;

fn rank(r: &AlignmentRelation) -> u8 {
    match r {
        AlignmentRelation::Identical => 0,
        AlignmentRelation::FastForward { .. } => 1,
        AlignmentRelation::Forked { .. } => 2,
        AlignmentRelation::Incompatible { .. } => 3,
    }
}

fn compose_forked(a: &AlignmentRelation, b: &AlignmentRelation) -> AlignmentRelation {
    let fork_epoch = match (a, b) {
        (AlignmentRelation::Forked { fork_epoch, .. }, _) => *fork_epoch,
        (_, AlignmentRelation::Forked { fork_epoch, .. }) => *fork_epoch,
        _ => EpochId(0),
    };
    AlignmentRelation::Forked {
        fork_epoch,
        local_divergence: 0.0,
        remote_divergence: 0.0,
    }
}

fn compose_ff(a: &AlignmentRelation, b: &AlignmentRelation) -> AlignmentRelation {
    match (a, b) {
        (
            AlignmentRelation::FastForward { behind: bh1, common_epoch: e1, .. },
            AlignmentRelation::FastForward {
                ahead: ah2,
                common_epoch: e2,
                ..
            },
        ) => AlignmentRelation::FastForward {
            ahead: ah2.clone(),
            behind: bh1.clone(),
            common_epoch: std::cmp::min(*e1, *e2),
            catch_up_epochs: vec![],
        },
        _ => unreachable!(),
    }
}

pub fn compose(a: &AlignmentRelation, b: &AlignmentRelation) -> AlignmentRelation {
    let (ra, rb) = (rank(a), rank(b));

    if ra == 3 || rb == 3 {
        return AlignmentRelation::Incompatible {
            reason: crate::IncompatibilityReason::GenesisMismatch,
        };
    }
    if ra == 2 || rb == 2 {
        return compose_forked(a, b);
    }
    if ra == 0 {
        return b.clone();
    }
    if rb == 0 {
        return a.clone();
    }

    compose_ff(a, b)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cluster(pub Vec<NodeId>);

impl Cluster {
    pub fn contains(&self, node: &NodeId) -> bool {
        self.0.contains(node)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FederationState {
    Stable,
    Divergent {
        fork_epochs: Vec<EpochId>,
    },
    Partitioned {
        clusters: Vec<Cluster>,
        cross_cluster_forks: Vec<(NodeId, NodeId, EpochId)>,
    },
    Collapsed {
        reason: String,
        incompatible_pairs: Vec<(NodeId, NodeId)>,
    },
}

impl FederationState {
    pub fn is_stable(&self) -> bool {
        matches!(self, FederationState::Stable)
    }

    pub fn is_collapsed(&self) -> bool {
        matches!(self, FederationState::Collapsed { .. })
    }
}

pub struct ClosureGraph {
    nodes: Vec<NodeId>,
    relations: HashMap<(NodeId, NodeId), AlignmentRelation>,
}

impl ClosureGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            relations: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node: NodeId) {
        if !self.nodes.contains(&node) {
            self.nodes.push(node);
        }
    }

    pub fn add_relation(&mut self, a: NodeId, b: NodeId, r: AlignmentRelation) {
        self.add_node(a.clone());
        self.add_node(b.clone());
        self.relations.insert((a, b), r);
    }

    pub fn get_relation(&self, a: &NodeId, b: &NodeId) -> Option<&AlignmentRelation> {
        if a == b {
            return Some(&AlignmentRelation::Identical);
        }
        self.relations.get(&(a.clone(), b.clone()))
            .or_else(|| self.relations.get(&(b.clone(), a.clone())))
    }

    pub fn compute_closure(&self) -> Vec<(NodeId, NodeId, AlignmentRelation)> {
        let n = self.nodes.len();
        let idx: HashMap<&NodeId, usize> =
            self.nodes.iter().enumerate().map(|(i, n)| (n, i)).collect();

        let mut mat: Vec<Vec<Option<AlignmentRelation>>> = vec![vec![None; n]; n];

        for i in 0..n {
            mat[i][i] = Some(AlignmentRelation::Identical);
        }

        for ((a, b), r) in &self.relations {
            let ai = idx[a];
            let bi = idx[b];
            mat[ai][bi] = Some(r.clone());
        }

        for k in 0..n {
            for i in 0..n {
                let ik = match &mat[i][k] {
                    Some(r) => r.clone(),
                    None => continue,
                };
                for j in 0..n {
                    let kj = match &mat[k][j] {
                        Some(r) => r.clone(),
                        None => continue,
                    };
                    let candidate = compose(&ik, &kj);
                    match &mat[i][j] {
                        Some(current) if rank(&candidate) >= rank(current) => {}
                        _ => {
                            mat[i][j] = Some(candidate);
                        }
                    }
                }
            }
        }

        let mut result = Vec::new();
        for i in 0..n {
            for j in (i + 1)..n {
                if let Some(r) = &mat[i][j] {
                    result.push((self.nodes[i].clone(), self.nodes[j].clone(), r.clone()));
                }
            }
        }
        result
    }

    pub fn classify(&self) -> FederationState {
        let n = self.nodes.len();
        if n <= 1 {
            return FederationState::Stable;
        }

        let closure = self.compute_closure();

        let mut incompatible_pairs = Vec::new();
        let mut fork_epochs = Vec::new();
        let mut has_forked = false;

        for (a, b, r) in &closure {
            match r {
                AlignmentRelation::Incompatible { .. } => {
                    incompatible_pairs.push((a.clone(), b.clone()));
                }
                AlignmentRelation::Forked { fork_epoch, .. } => {
                    has_forked = true;
                    fork_epochs.push(*fork_epoch);
                }
                _ => {}
            }
        }

        if !incompatible_pairs.is_empty() {
            return FederationState::Collapsed {
                reason: format!("{} incompatible pair(s) detected", incompatible_pairs.len()),
                incompatible_pairs,
            };
        }

        if !has_forked {
            return FederationState::Stable;
        }

        // Cluster detection: nodes connected by FastForward or Identical
        let mut visited: HashSet<&NodeId> = HashSet::new();
        let mut clusters: Vec<Cluster> = Vec::new();

        for node in &self.nodes {
            if visited.contains(node) {
                continue;
            }

            let mut cluster = vec![node.clone()];
            let mut queue = VecDeque::new();
            queue.push_back(node);
            visited.insert(node);

            while let Some(current) = queue.pop_front() {
                for (a, b, r) in &closure {
                    let neighbor = if a == current {
                        Some(b)
                    } else if b == current {
                        Some(a)
                    } else {
                        None
                    };
                    if let Some(nb) = neighbor {
                        if !visited.contains(nb) && (rank(r) == 0 || rank(r) == 1) {
                            visited.insert(nb);
                            cluster.push(nb.clone());
                            queue.push_back(nb);
                        }
                    }
                }
            }

            if cluster.len() > 1 || clusters.is_empty() {
                clusters.push(Cluster(cluster));
            } else {
                clusters.last_mut().unwrap().0.push(cluster.into_iter().next().unwrap());
            }
        }

        if clusters.len() > 1 {
            let mut cross = Vec::new();
            for ci in 0..clusters.len() {
                for cj in (ci + 1)..clusters.len() {
                    for a in &clusters[ci].0 {
                        for b in &clusters[cj].0 {
                            if let Some(AlignmentRelation::Forked { fork_epoch, .. }) =
                                self.get_relation(a, b)
                            {
                                cross.push((a.clone(), b.clone(), *fork_epoch));
                            }
                        }
                    }
                }
            }
            return FederationState::Partitioned {
                clusters,
                cross_cluster_forks: cross,
            };
        }

        FederationState::Divergent {
            fork_epochs,
        }
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn clusters(&self) -> Vec<Cluster> {
        let state = self.classify();
        match state {
            FederationState::Partitioned { clusters, .. } => clusters,
            _ => {
                vec![Cluster(self.nodes.clone())]
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PhaseTransition {
    StableToStable,
    StableToDivergent {
        fork_epochs: Vec<EpochId>,
        nodes: Vec<NodeId>,
    },
    DivergentToPartitioned {
        split_epochs: Vec<EpochId>,
        clusters: Vec<Cluster>,
    },
    AnyToCollapsed {
        reason: String,
        incompatible_pairs: Vec<(NodeId, NodeId)>,
    },
}

pub fn detect_transition(prev: &FederationState, curr: &FederationState) -> PhaseTransition {
    use FederationState::*;
    match (prev, curr) {
        (Stable, Stable) => PhaseTransition::StableToStable,
        (Stable, Divergent { fork_epochs }) => PhaseTransition::StableToDivergent {
            fork_epochs: fork_epochs.clone(),
            nodes: Vec::new(),
        },
        (Divergent { .. }, Partitioned { clusters, .. }) => {
            PhaseTransition::DivergentToPartitioned {
                split_epochs: Vec::new(),
                clusters: clusters.clone(),
            }
        }
        (_, Collapsed { reason, incompatible_pairs }) => {
            PhaseTransition::AnyToCollapsed {
                reason: reason.clone(),
                incompatible_pairs: incompatible_pairs.clone(),
            }
        }
        _ => PhaseTransition::StableToStable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vantage_core::NodeId;

    fn id_identical() -> AlignmentRelation {
        AlignmentRelation::Identical
    }

    fn id_ff(ahead: &str, behind: &str) -> AlignmentRelation {
        AlignmentRelation::FastForward {
            ahead: NodeId(ahead.into()),
            behind: NodeId(behind.into()),
            common_epoch: EpochId(5),
            catch_up_epochs: vec![],
        }
    }

    fn id_forked(epoch: u64) -> AlignmentRelation {
        AlignmentRelation::Forked {
            fork_epoch: EpochId(epoch),
            local_divergence: 1.0,
            remote_divergence: 1.0,
        }
    }

    fn id_incompatible() -> AlignmentRelation {
        AlignmentRelation::Incompatible {
            reason: crate::IncompatibilityReason::GenesisMismatch,
        }
    }

    #[test]
    fn test_identical_is_identity() {
        let a = id_ff("b", "a");
        assert_eq!(compose(&id_identical(), &a), a);
        assert_eq!(compose(&a, &id_identical()), a);
    }

    #[test]
    fn test_incompatible_is_absorbing() {
        let inc = id_incompatible();
        let ff = id_ff("b", "a");
        assert_eq!(rank(&compose(&inc, &ff)), 3);
        assert_eq!(rank(&compose(&ff, &inc)), 3);
        assert_eq!(rank(&compose(&inc, &id_forked(3))), 3);
        assert_eq!(rank(&compose(&inc, &id_identical())), 3);
    }

    #[test]
    fn test_forked_is_semi_absorbing() {
        let fork = id_forked(3);
        assert_eq!(rank(&compose(&fork, &id_ff("b", "a"))), 2);
        assert_eq!(rank(&compose(&id_ff("b", "a"), &fork)), 2);
        assert_eq!(rank(&compose(&fork, &id_identical())), 2);
    }

    #[test]
    fn test_ff_propagates() {
        let ab = id_ff("b", "a");
        let bc = id_ff("c", "b");
        let ac = compose(&ab, &bc);
        match &ac {
            AlignmentRelation::FastForward { ahead, behind, .. } => {
                assert_eq!(ahead.0, "c");
                assert_eq!(behind.0, "a");
            }
            other => panic!("expected FastForward, got {other:?}"),
        }
        assert_eq!(rank(&ac), 1);
    }

    #[test]
    fn test_closure_stable() {
        let mut g = ClosureGraph::new();
        g.add_relation(
            NodeId("a".into()),
            NodeId("b".into()),
            id_ff("b", "a"),
        );
        g.add_relation(
            NodeId("b".into()),
            NodeId("c".into()),
            id_ff("c", "b"),
        );
        g.add_relation(
            NodeId("a".into()),
            NodeId("c".into()),
            id_ff("c", "a"),
        );
        assert_eq!(g.classify(), FederationState::Stable);
    }

    #[test]
    fn test_closure_divergent() {
        let mut g = ClosureGraph::new();
        g.add_relation(
            NodeId("a".into()),
            NodeId("b".into()),
            id_forked(3),
        );
        let state = g.classify();
        assert!(matches!(state, FederationState::Divergent { .. }));
    }

    #[test]
    fn test_closure_collapsed() {
        let mut g = ClosureGraph::new();
        g.add_relation(
            NodeId("a".into()),
            NodeId("b".into()),
            id_incompatible(),
        );
        let state = g.classify();
        assert!(matches!(state, FederationState::Collapsed { .. }));
    }

    #[test]
    fn test_closure_partitioned() {
        let mut g = ClosureGraph::new();
        // cluster 1: a → b → c (FastForward chain)
        g.add_relation(NodeId("a".into()), NodeId("b".into()), id_ff("b", "a"));
        g.add_relation(NodeId("b".into()), NodeId("c".into()), id_ff("c", "b"));
        // cluster 2: x → y
        g.add_relation(NodeId("x".into()), NodeId("y".into()), id_ff("y", "x"));
        // cross-cluster fork
        g.add_relation(NodeId("a".into()), NodeId("x".into()), id_forked(2));

        let state = g.classify();
        match state {
            FederationState::Partitioned { clusters, .. } => {
                assert_eq!(clusters.len(), 2);
            }
            other => panic!("expected Partitioned, got {other:?}"),
        }
    }

    #[test]
    fn test_transition_detection() {
        let stable = FederationState::Stable;
        let div = FederationState::Divergent {
            fork_epochs: vec![EpochId(3)],
        };
        let t = detect_transition(&stable, &div);
        assert!(matches!(t, PhaseTransition::StableToDivergent { .. }));

        let coll = FederationState::Collapsed {
            reason: "test".into(),
            incompatible_pairs: vec![],
        };
        let t2 = detect_transition(&div, &coll);
        assert!(matches!(t2, PhaseTransition::AnyToCollapsed { .. }));
    }

    #[test]
    fn test_ff_ff_common_epoch_min() {
        let ab = AlignmentRelation::FastForward {
            ahead: NodeId("b".into()),
            behind: NodeId("a".into()),
            common_epoch: EpochId(3),
            catch_up_epochs: vec![],
        };
        let bc = AlignmentRelation::FastForward {
            ahead: NodeId("c".into()),
            behind: NodeId("b".into()),
            common_epoch: EpochId(7),
            catch_up_epochs: vec![],
        };
        let ac = compose(&ab, &bc);
        match ac {
            AlignmentRelation::FastForward { common_epoch, .. } => {
                assert_eq!(common_epoch, EpochId(3));
            }
            other => panic!("expected FastForward, got {other:?}"),
        }
    }
}
