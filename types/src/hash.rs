use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HashAlgorithm {
    Sha256,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralHash {
    pub algorithm: HashAlgorithm,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticHash {
    pub algorithm: HashAlgorithm,
    pub value: String,
}

/// Bipartite Symbol Identity Hash (v1.2.4)
/// Signature changes trigger dependent invalidation, while Body changes are local.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SymbolHash {
    /// Affects dependents (e.g. function signature, struct layout)
    pub signature_hash: crate::caf::CafHash,
    /// Only affects local validation (implementation details)
    pub body_hash: crate::caf::CafHash,
}
