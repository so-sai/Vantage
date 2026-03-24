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
