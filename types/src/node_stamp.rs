use crate::caf::CafHash;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeStamp {
    pub hash: CafHash,
    pub epoch: u32,
    pub dirty: bool,
}

impl NodeStamp {
    pub fn new(hash: CafHash, epoch: u32) -> Self {
        Self {
            hash,
            epoch,
            dirty: false,
        }
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn is_stale(&self, current_epoch: u32) -> bool {
        self.dirty || self.epoch < current_epoch
    }
}
