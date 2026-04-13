use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HashVersion {
    /// Version of the SemanticRole enum layout
    pub semantic_role_version: u32,

    /// Version of the hashing algorithm (e.g., Sha256 vs Blake3)
    pub hash_algo_version: u32,

    /// Version of the canonicalization rules (e.g., sorting logic)
    pub canonicalization_version: u32,
}

impl HashVersion {
    /// The current version of the Vantage Structural Engine identity physics.
    pub const CURRENT: Self = Self {
        semantic_role_version: 1,
        hash_algo_version: 1,
        canonicalization_version: 1,
    };

    pub fn is_compatible(&self, other: &Self) -> bool {
        self.semantic_role_version == other.semantic_role_version
            && self.hash_algo_version == other.hash_algo_version
            && self.canonicalization_version == other.canonicalization_version
    }
}
