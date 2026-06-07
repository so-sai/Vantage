use std::collections::BTreeMap;
use vantage_core::NodeId;

use crate::state::NodeView;

#[derive(Debug, Clone)]
pub struct EpochDivergenceState {
    pub entries: BTreeMap<NodeId, NodeView>,
}

impl EpochDivergenceState {
    pub fn new() -> Self {
        Self { entries: BTreeMap::new() }
    }

    pub fn from_views(views: impl IntoIterator<Item = NodeView>) -> Self {
        let mut state = Self::new();
        for view in views {
            state.insert(view);
        }
        state
    }

    pub fn insert(&mut self, view: NodeView) {
        self.entries
            .entry(view.node_id.clone())
            .and_modify(|existing| {
                if view.epoch > existing.epoch {
                    *existing = view.clone();
                }
            })
            .or_insert(view);
    }

    pub fn node_view(&self, node: &NodeId) -> Option<&NodeView> {
        self.entries.get(node)
    }

    pub fn divergence_between(&self, a: &NodeId, b: &NodeId) -> Option<f64> {
        let va = self.entries.get(a)?;
        let vb = self.entries.get(b)?;

        if va.genesis_hash != vb.genesis_hash {
            return Some(f64::MAX);
        }

        let epoch_distance = (va.epoch.0 as i64 - vb.epoch.0 as i64).unsigned_abs() as f64;

        let hash_mismatch = if va.commit_hash == vb.commit_hash {
            0.0
        } else {
            1.0
        };

        Some(epoch_distance + hash_mismatch)
    }

    pub fn max_divergence(&self) -> f64 {
        let nodes: Vec<&NodeId> = self.entries.keys().collect();
        let mut max_d = 0.0f64;
        for i in 0..nodes.len() {
            for j in (i + 1)..nodes.len() {
                if let Some(d) = self.divergence_between(nodes[i], nodes[j]) {
                    if d > max_d {
                        max_d = d;
                    }
                }
            }
        }
        max_d
    }

    pub fn all_genesis_match(&self) -> bool {
        let mut hashes = self.entries.values().map(|v| &v.genesis_hash);
        let first = match hashes.next() {
            Some(h) => h,
            None => return true,
        };
        hashes.all(|h| h == first)
    }
}
