use crate::caf::CafNode;
use crate::node_id::NodeId;
use crate::node_stamp::NodeStamp;
use std::collections::HashMap;

pub struct NodeArena {
    pub nodes: HashMap<NodeId, NodeEntry>,
    generation: u64,
}

pub struct NodeEntry {
    pub node: CafNode,
    pub stamp: NodeStamp,
    pub generation: u64,
}

impl NodeArena {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            generation: 0,
        }
    }

    pub fn insert(&mut self, id: NodeId, node: CafNode, stamp: NodeStamp) {
        self.nodes.insert(
            id,
            NodeEntry {
                node,
                stamp,
                generation: self.generation,
            },
        );
    }

    pub fn get(&self, id: &NodeId) -> Option<&NodeEntry> {
        self.nodes.get(id)
    }

    pub fn get_mut(&mut self, id: &NodeId) -> Option<&mut NodeEntry> {
        self.nodes.get_mut(id)
    }

    pub fn get_mut_with_bump(&mut self, id: &NodeId) -> Option<&mut NodeEntry> {
        if let Some(entry) = self.nodes.get_mut(id) {
            entry.generation = self.generation;
            Some(entry)
        } else {
            None
        }
    }

    pub fn bump_generation(&mut self) {
        self.generation += 1;
    }

    pub fn gc(&mut self) {
        let current_gen = self.generation;
        self.nodes
            .retain(|_, entry| entry.generation == current_gen);
    }
}

impl Default for NodeArena {
    fn default() -> Self {
        Self::new()
    }
}
