//! # Structural Edge Types for Kit Integration — v1.2.5
//!
//! Data Transfer Objects for graph edge extraction between Vantage and Kit.
//! `EdgeType` is the legacy transport contract for Kit JSONL.
//! `UnifiedEdgeKind` is the canonical v1.2.5 edge taxonomy (supersedes both EdgeType and DependencyKind).

use serde::{Deserialize, Serialize};

/// Canonical edge taxonomy (v1.2.5) — replaces both `EdgeType` and `DependencyKind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnifiedEdgeKind {
    // --- Module structure ---
    /// Direct import: `import os`, `use core::auth`
    Imports,
    /// From import: `from core.auth import login`
    FromImport,
    /// Re-export: `pub use`, `__all__`
    ReExport,

    // --- Inheritance ---
    /// Class/struct extension: `class Foo(Bar)`, `struct Foo: Bar`
    Extends,
    /// Interface/trait implementation
    Implements,

    // --- Call graph ---
    /// Resolved function/method call
    CallsResolved,
    /// Unresolved function/method call
    CallsUnresolved,

    // --- React/JSX (component-oriented) ---
    /// Component renders another component
    Renders,
    /// Hook invocation
    HooksInto,
    /// Context consumption
    UsesContext,

    // --- Generic ---
    /// Weak dependency (reference, usage)
    References,
}

/// Legacy edge type enumeration - locked contract for Kit JSONL transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum EdgeType {
    /// Direct module import: `import os`, `import core.auth`
    Imports,
    /// From import: `from core.auth import login`
    FromImport,
    /// Class inheritance: `class Foo(Bar)`
    Inherits,
    /// Function/method call - unresolved
    CallsUnresolved,
    /// Function/method call - resolved to target
    CallsResolved,
}

impl EdgeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EdgeType::Imports => "IMPORTS",
            EdgeType::FromImport => "FROM_IMPORT",
            EdgeType::Inherits => "INHERITS",
            EdgeType::CallsUnresolved => "CALLS_UNRESOLVED",
            EdgeType::CallsResolved => "CALLS_RESOLVED",
        }
    }
}

/// Edge event emitted from Vantage extractor to Kit
/// JSONL format: one JSON object per line
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeEvent {
    /// FQN of source module (e.g., "kit.api")
    pub source: String,
    /// Target module/symbol (e.g., "core.auth")
    pub target: String,
    /// Type of edge connection
    pub edge_type: EdgeType,
    /// Source code language
    pub language: String,
    /// Physical file path of source
    pub source_file: String,
    /// Physical file path of target (if known)
    pub target_file: Option<String>,
    /// Line number in source file
    pub line: usize,
    /// Confidence score (0.0-1.0) - 1.0 for IMPORTS
    pub confidence: f32,
    /// Raw import text for resolution (e.g., ".utils" for relative imports)
    pub raw_text: String,
}

impl EdgeEvent {
    /// Emit as JSONL line (no trailing newline - caller adds it)
    pub fn to_jsonl(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}
