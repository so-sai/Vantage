use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::symbol::SymbolKind;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralSignal {
    pub uuid: String,            // Epistemic identifier (@epistemic:<uuid>)
    pub symbol_id: String,       // Stable identity (e.g., "core::parser::solve")
    pub parent: Option<String>,  // Hierarchy parent UUID or name
    pub symbol_kind: SymbolKind, 
    pub language: String,        
    pub structural_hash: String, 
    pub normalized_hash: String, // Formerly semantic_hash
    pub signature: Option<String>,
    pub metadata: HashMap<String, String>,
    pub origin: Origin,          
    pub confidence: f32,         
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Origin {
    pub parser: String,          
    pub version: String,         
}
