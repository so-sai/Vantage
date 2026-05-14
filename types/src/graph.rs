//! # Symbol Dependency Graph DTO (v1.2.4)
//!
//! Primitives and Data Transfer Objects for the Phase C Semantic Engine.
//! Separates the Transport Layer from the Authoritative Engine in core.

use crate::symbol::SymbolKind;
use crate::symbol_id::SymbolId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolState {
    /// Signature parsed, body analysis pending or deferred.
    Discovered,

    /// Successfully mapped to a physical NodeId in the AST.
    Bound,

    /// Outgoing and incoming dependencies fully resolved.
    Validated,

    /// Underlying structure or dependencies changed. Recompute required.
    Dirty,

    /// Symbol no longer present in AST. Kept for one generation to broadcast invalidation.
    Tombstoned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DependencyKind {
    /// Strict dependency: if signature/layout changes, dependents are invalidated.
    SignatureRef,

    /// Loose dependency: body change does NOT invalidate dependents.
    CallEdge,

    /// Structural module relationship.
    ModuleImport,

    /// Alias relationship for canonicalization.
    ReExport,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DependencyEdge {
    pub target: SymbolId,
    pub kind: DependencyKind,
}

/// Lightweight DTO for transporting graph snapshots to Kit/Agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolGraphDTO {
    pub nodes: Vec<SymbolNodeDTO>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolNodeDTO {
    pub symbol: SymbolId,
    pub state: SymbolState,
    pub dependencies: Vec<DependencyEdge>,
    pub dependents: Vec<SymbolId>,
    /// v1.2.5: Source language identifier (e.g. "rust", "python", "ruby")
    pub language: String,
    /// v1.2.5: Structural kind (Struct, Class, Function, Component, etc.)
    pub kind: SymbolKind,
    /// v1.2.5: Fully-qualified name (e.g. "crate::module::SymbolName")
    pub fq_name: String,
}
