//! # Unified Intermediate Representation (UIR) — v1.2.5
//!
//! Language-agnostic structural graph contract for multi-language normalization.
//! All language parsers converge to this representation.
//!
//! ## Design rules
//! - Normalize graph topology, NOT language philosophy
//! - `same normalized structure → same normalized hash`
//! - No semantic equivalence cross-language (Rust struct ≠ Python class)

use serde::{Deserialize, Serialize};

use crate::edge::UnifiedEdgeKind;
use crate::intent::IntentOverlay;
use crate::symbol::SymbolKind;
use crate::SymbolId;

/// Supported languages for structural extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    Rust,
    Python,
    Ruby,
    JavaScript,
    TypeScript,
    Tsx,
}

impl Language {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "rs" => Some(Language::Rust),
            "py" => Some(Language::Python),
            "rb" => Some(Language::Ruby),
            "js" | "jsx" => Some(Language::JavaScript),
            "ts" => Some(Language::TypeScript),
            "tsx" => Some(Language::Tsx),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::Python => "python",
            Language::Ruby => "ruby",
            Language::JavaScript => "javascript",
            Language::TypeScript => "typescript",
            Language::Tsx => "tsx",
        }
    }
}

/// Visibility scope of a symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Private,
    Internal,
    Exported,
}

/// Edge in the unified graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UnifiedEdge {
    pub target: SymbolId,
    pub kind: UnifiedEdgeKind,
}

/// Node in the unified graph — language-agnostic structural representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedNode {
    /// Unique identity within the graph
    pub id: SymbolId,
    /// Fully-qualified name (e.g. "module::submodule::SymbolName")
    pub fq_name: String,
    /// Source language
    pub language: Language,
    /// Structural kind (Struct, Class, Function, Component, etc.)
    pub kind: SymbolKind,
    /// Visibility boundary
    pub visibility: Visibility,
    /// Outgoing edges
    pub dependencies: Vec<UnifiedEdge>,
    /// Forensic location
    pub file: String,
    pub line: u32,
    /// Deterministic hashes
    pub structural_hash: String,
    pub normalized_hash: String,
    /// Architectural intent overlay (optional — extracted from vantage: comments)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent: Option<IntentOverlay>,
}

/// A complete unified graph for a single language extraction pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedGraph {
    pub nodes: Vec<UnifiedNode>,
    pub source_language: Language,
}
