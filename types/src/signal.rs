use crate::symbol::SymbolKind;
use crate::symbol_id::SymbolId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveSignal {
    pub uuid: String,           // Epistemic identifier (@epistemic:<uuid>)
    pub symbol_id: SymbolId,    // Stable identity (e.g., "core::parser::solve")
    pub parent: Option<String>, // Hierarchy parent UUID or name
    pub symbol_kind: SymbolKind,
    pub language: String,
    pub structural_hash: String,
    pub semantic_hash: String,   // Whitespace-invariant
    pub normalized_hash: String, // Rename-invariant (identifier-stripped AST)
    pub signature: Option<String>,
    pub location: SourceLocation,
    pub metadata: HashMap<String, String>,
    pub origin: Origin,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file: String,
    pub start_line: u32, // 1-indexed
    pub start_col: u32,  // 0-indexed
    pub end_line: u32,   // 1-indexed
    pub end_col: u32,    // 0-indexed
    pub byte_start: usize,
    pub byte_end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Origin {
    pub parser: String,
    pub version: String,
}
