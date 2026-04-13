use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::caf::{CafHash, CafNode};
use crate::semantic::CafContext;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditKind {
    Insert,
    Delete,
    Replace,
    Move,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputEdit {
    pub byte_start: usize,
    pub byte_end: usize,
    pub kind: EditKind,
    pub old_content: Option<String>,
    pub new_content: Option<String>,
}

impl InputEdit {
    pub fn insert(start: usize, content: &str) -> Self {
        Self {
            byte_start: start,
            byte_end: start,
            kind: EditKind::Insert,
            old_content: None,
            new_content: Some(content.to_string()),
        }
    }

    pub fn delete(start: usize, end: usize) -> Self {
        Self {
            byte_start: start,
            byte_end: end,
            kind: EditKind::Delete,
            old_content: None,
            new_content: None,
        }
    }

    pub fn replace(start: usize, end: usize, content: &str) -> Self {
        Self {
            byte_start: start,
            byte_end: end,
            kind: EditKind::Replace,
            old_content: None,
            new_content: Some(content.to_string()),
        }
    }

    pub fn affects_range(&self, node_start: usize, node_end: usize) -> bool {
        !(node_end <= self.byte_start || node_start >= self.byte_end)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DirtyRegion {
    pub byte_start: usize,
    pub byte_end: usize,
    pub reason: DirtyReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DirtyReason {
    DirectEdit,
    AncestorChanged,
    ScopeModified,
    CommutativityChanged,
}

impl Default for DirtyReason {
    fn default() -> Self {
        DirtyReason::DirectEdit
    }
}

#[derive(Debug, Clone, Default)]
pub struct IncrementalState {
    pub previous_hash: Option<CafHash>,
    pub dirty_regions: Vec<DirtyRegion>,
    pub scope_snapshot: CafContext,
    pub node_hashes: HashMap<usize, CafHash>,
}

pub struct IncrementalCafBuilder {
    state: IncrementalState,
    edits: Vec<InputEdit>,
}

impl IncrementalCafBuilder {
    pub fn new() -> Self {
        Self {
            state: IncrementalState::default(),
            edits: Vec::new(),
        }
    }

    pub fn with_previous_state(mut self, previous_hash: CafHash, scope: CafContext) -> Self {
        self.state.previous_hash = Some(previous_hash);
        self.state.scope_snapshot = scope;
        self
    }

    pub fn add_edit(&mut self, edit: InputEdit) {
        self.edits.push(edit);
    }

    pub fn add_edits(&mut self, edits: Vec<InputEdit>) {
        self.edits.extend(edits);
    }

    pub fn compute_dirty_regions(&self) -> Vec<DirtyRegion> {
        let mut regions = Vec::new();

        for edit in &self.edits {
            let reason = match edit.kind {
                EditKind::Insert | EditKind::Replace => DirtyReason::DirectEdit,
                EditKind::Delete => DirtyReason::DirectEdit,
                EditKind::Move => DirtyReason::AncestorChanged,
            };

            let region = DirtyRegion {
                byte_start: edit.byte_start,
                byte_end: edit.byte_end,
                reason,
            };
            regions.push(region);
        }

        let mut covered: HashSet<usize> = HashSet::new();
        let mut merged: Vec<DirtyRegion> = Vec::new();

        for region in &regions {
            let start = region.byte_start;
            let end = region.byte_end;
            let reason = region.reason;

            for i in start..end {
                if covered.contains(&i) {
                    continue;
                }
                covered.insert(i);
            }

            merged.push(DirtyRegion {
                byte_start: start,
                byte_end: end,
                reason,
            });
        }

        merged.sort_by_key(|r| r.byte_start);
        merged
    }

    pub fn is_subtree_dirty(&self, node_start: usize, node_end: usize) -> bool {
        for edit in &self.edits {
            if edit.affects_range(node_start, node_end) {
                return true;
            }
        }
        false
    }

    pub fn propagate_dirty_up(&self, dirty_nodes: &[usize]) -> HashSet<usize> {
        let mut dirty_ancestors: HashSet<usize> = dirty_nodes.iter().copied().collect();

        for &node_pos in dirty_nodes {
            let mut current = node_pos;
            while current > 0 {
                current = self.parent_offset(current);
                dirty_ancestors.insert(current);
            }
        }

        dirty_ancestors
    }

    fn parent_offset(&self, child: usize) -> usize {
        if child == 0 {
            return 0;
        }
        child.saturating_sub(1)
    }

    pub fn compute_rebuild_candidates(&self, node_positions: &[usize]) -> Vec<usize> {
        let dirty_regions = self.compute_dirty_regions();
        let mut candidates: Vec<usize> = Vec::new();

        for &pos in node_positions {
            for region in &dirty_regions {
                if pos >= region.byte_start && pos < region.byte_end {
                    candidates.push(pos);
                    break;
                }
            }
        }

        candidates
    }

    pub fn has_significant_change(&self, new_hash: &CafHash) -> bool {
        match &self.state.previous_hash {
            Some(old_hash) => old_hash != new_hash,
            None => true,
        }
    }

    pub fn get_previous_hash(&self) -> Option<&CafHash> {
        self.state.previous_hash.as_ref()
    }

    pub fn finalize(&mut self) -> IncrementalState {
        self.state.dirty_regions = self.compute_dirty_regions();
        self.state.clone()
    }
}

pub struct CafCache {
    node_map: HashMap<String, CafNode>,
    hash_index: HashMap<CafHash, Vec<String>>,
}

impl CafCache {
    pub fn new() -> Self {
        Self {
            node_map: HashMap::new(),
            hash_index: HashMap::new(),
        }
    }

    pub fn insert(&mut self, node_id: String, node: CafNode, hash: CafHash) {
        self.hash_index
            .entry(hash.clone())
            .or_insert_with(Vec::new)
            .push(node_id.clone());
        self.node_map.insert(node_id, node);
    }

    pub fn get(&self, node_id: &str) -> Option<&CafNode> {
        self.node_map.get(node_id)
    }

    pub fn find_by_hash(&self, hash: &CafHash) -> Option<&Vec<String>> {
        self.hash_index.get(hash)
    }

    pub fn invalidate(&mut self, node_id: &str) -> Option<CafNode> {
        if let Some(node) = self.node_map.remove(node_id) {
            for hash_list in self.hash_index.values_mut() {
                hash_list.retain(|id| id != node_id);
            }
            return Some(node);
        }
        None
    }

    pub fn invalidate_range(&mut self, start: usize, end: usize) -> Vec<String> {
        let mut invalidated = Vec::new();

        let to_remove: Vec<String> = self
            .node_map
            .iter()
            .filter(|(_, node)| node.byte_start >= start && node.byte_end <= end)
            .map(|(id, _)| id.clone())
            .collect();

        for id in &to_remove {
            if let Some(node) = self.node_map.remove(id) {
                invalidated.push(id.clone());
                let hash = CafHash::sha256(format!("{}:{}", id, node.kind).as_bytes());
                if let Some(list) = self.hash_index.get_mut(&hash) {
                    list.retain(|h| h != id);
                }
            }
        }

        invalidated
    }
}

impl Default for CafCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_edit_insert() {
        let edit = InputEdit::insert(10, "new content");
        assert_eq!(edit.byte_start, 10);
        assert_eq!(edit.byte_end, 10);
        assert_eq!(edit.kind, EditKind::Insert);
    }

    #[test]
    fn test_input_edit_affects_range() {
        let edit = InputEdit::replace(5, 15, "replacement");

        assert!(edit.affects_range(0, 10));
        assert!(edit.affects_range(10, 20));
        assert!(edit.affects_range(5, 15));
        assert!(!edit.affects_range(0, 4));
        assert!(!edit.affects_range(16, 20));
    }

    #[test]
    fn test_dirty_region_computation() {
        let mut builder = IncrementalCafBuilder::new();
        builder.add_edit(InputEdit::replace(10, 20, "new"));
        builder.add_edit(InputEdit::insert(30, "added"));

        let regions = builder.compute_dirty_regions();

        assert_eq!(regions.len(), 2);
    }

    #[test]
    fn test_subtree_dirty_detection() {
        let mut builder = IncrementalCafBuilder::new();
        builder.add_edit(InputEdit::replace(5, 15, "replacement"));

        assert!(builder.is_subtree_dirty(0, 100));
        assert!(builder.is_subtree_dirty(10, 12));
        assert!(!builder.is_subtree_dirty(100, 200));
    }

    #[test]
    fn test_cache_insert_and_lookup() {
        let mut cache = CafCache::new();

        let node = CafNode {
            kind: "function_item".to_string(),
            semantic_id: None,
            children: vec![],
            commutativity: crate::semantic::Commutativity::OrderInsensitive,
            byte_start: 0,
            byte_end: 50,
            parent_id: None,
        };

        let hash = CafHash::sha256(b"test");

        cache.insert("func1".to_string(), node.clone(), hash.clone());

        assert!(cache.get("func1").is_some());
        assert!(cache.find_by_hash(&hash).is_some());
    }

    #[test]
    fn test_cache_invalidation() {
        let mut cache = CafCache::new();

        let node = CafNode {
            kind: "function_item".to_string(),
            semantic_id: None,
            children: vec![],
            commutativity: crate::semantic::Commutativity::OrderSensitive,
            byte_start: 0,
            byte_end: 50,
            parent_id: None,
        };

        let hash = CafHash::sha256(b"test");

        cache.insert("func1".to_string(), node, hash);
        cache.invalidate("func1");

        assert!(cache.get("func1").is_none());
    }

    #[test]
    fn test_incremental_builder_with_previous() {
        let old_hash = CafHash::sha256(b"old_state");
        let old_scope = CafContext::new();

        let builder = IncrementalCafBuilder::new().with_previous_state(old_hash, old_scope);

        let mut builder = builder;
        builder.add_edit(InputEdit::insert(0, "new"));

        let regions = builder.compute_dirty_regions();
        assert!(!regions.is_empty());
    }
}
