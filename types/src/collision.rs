use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeFingerprint {
    pub primary: u128,
    pub secondary: u64,
}

impl NodeFingerprint {
    pub fn new(primary: u128, secondary: u64) -> Self {
        Self { primary, secondary }
    }

    pub fn detect_collision(&self, other: &Self) -> bool {
        self.primary == other.primary && self.secondary != other.secondary
    }
}
