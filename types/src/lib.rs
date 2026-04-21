// Policy: Structural integrity mandatory for No-GIL Python 3.14.3 compatibility.

/// The Authoritative ABI Snapshot Seal for Vantage v1.2.4.
/// This hash locks the V0 Protocol schema, SymbolId layout, and Registry semantics.
/// Any drift in memory layout or binary encoding MUST trigger a forensic failure.
pub const SYSTEM_ABI_HASH: &str = "7e0921a4f89d23c4e9e03d156b2c5b3d";

pub mod arena;
pub mod caf;
pub mod collision;
pub mod dirty_propagation;
pub mod graph;
pub mod hash;
pub mod identity_anchor;
pub mod incremental;
pub mod invariant;
pub mod node_id;
pub mod node_stamp;
pub mod role_resolver;
pub mod semantic;
pub mod semantic_role;
pub mod signal;
pub mod symbol;
pub mod symbol_id;
pub mod telemetry;
pub mod version;

pub mod edge;
pub mod introspection;
pub mod introspection_registry;

#[cfg(test)]
mod introspection_tests;

pub use introspection::{
    CapabilityDescriptor, SystemEnvelope, VantageCapabilityRegistry, VantageIntrospect,
};
pub use introspection_registry::CAPABILITY_REGISTRY;

pub use arena::NodeArena;
pub use caf::{CafBuilder, CafDiffReason, CafDiffResult, CafDiffer, CafHash, CafNode};
pub use collision::NodeFingerprint;
pub use dirty_propagation::DirtyPropagator;
pub use graph::{DependencyEdge, DependencyKind, SymbolGraphDTO, SymbolNodeDTO, SymbolState};
pub use hash::{HashAlgorithm, SemanticHash, StructuralHash, SymbolHash};
pub use identity_anchor::IdentityAnchor;
pub use incremental::{
    CafCache, DirtyReason, DirtyRegion, EditKind, IncrementalCafBuilder, IncrementalState,
    InputEdit,
};
pub use invariant::{
    ChangeType, CrossLanguageVerifier, InvariantTestCase, InvariantType, InvariantVerifier,
};
pub use node_id::{NodeId, DOMAIN_ROOT};
pub use node_stamp::NodeStamp;
pub use role_resolver::RoleResolver;
pub use semantic::{
    AlgebraResolver, CafContext, Commutativity, CommutativityTable, DefaultAlgebraResolver,
    ScopeContext, SemanticError, SemanticKind,
};
pub use semantic_role::SemanticRole;
pub use signal::{CognitiveSignal, Origin, SourceLocation};
pub use symbol::SymbolKind;
pub use symbol_id::{interner, registry, SymbolId, SymbolRegistry, SymbolScopeRegistry};
pub use telemetry::PerfMetrics;
pub use version::HashVersion;
pub use edge::EdgeEvent;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Constitutional Failure States (Non-negotiable)
#[derive(Error, Debug)]
pub enum ConstitutionalError {
    #[error("F1: Unmanaged - {0}")]
    Unmanaged(String),
    #[error("F2: Tampered - {0}")]
    Tampered(String),
    #[error("F3: Conflict - {0}")]
    Conflict(String),
}

/// Standardized failure taxonomy for Kit integration.
/// Kit uses this enum to decide recovery strategy:
///   - SyntaxError / UnsupportedLanguage → skip or retry
///   - NoAnchorFound → skip file (not epistemic)
///   - FileReadError / InternalError → report to operator
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FailureReason {
    /// Source code has syntax errors that prevent AST parsing
    SyntaxError,
    /// File extension not supported (no tree-sitter grammar)
    UnsupportedLanguage,
    /// No @epistemic anchor found in file
    NoAnchorFound,
    /// I/O error reading file (permission denied, not found, etc.)
    FileReadError,
    /// Internal Vantage error (logic bug, panic)
    InternalError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub manifest_version: u32,
    pub identity: Identity,
    pub authority: Authority,
    pub host_binding: HostBinding,
    pub integrity: Integrity,
    pub constraints: Constraints,
    pub status: Status,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub skill_id: String,
    pub skill_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Authority {
    pub issued_by: String,
    /// Unix Epoch (Forensic Standard)
    pub issued_at: u64,
    pub authority_level: AuthorityLevel,
    pub human_acknowledgement: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AuthorityLevel {
    Sandbox,
    Trusted,
    Privileged,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostBinding {
    pub host_type: HostType,
    pub host_scope: HostScope,
    pub host_signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HostType {
    Antigravity,
    Opencode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HostScope {
    Workspace,
    Global,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Integrity {
    pub hash_algorithm: String,
    pub content_hash: String,
    pub manifest_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraints {
    pub read_only: bool,
    pub allow_dynamic_write: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Active,
    Revoked,
    Archived,
}

impl Manifest {
    /// Enforces Constitutional Law on the Manifest State.
    /// Returns ConstitutionalError if any Law is violated.
    pub fn validate(&self) -> Result<(), ConstitutionalError> {
        // Law 1: Explicit Intent (F1)
        if !self.authority.human_acknowledgement {
            return Err(ConstitutionalError::Unmanaged(
                "Invariant 1.1 Violation: Explicit Human Acknowledgement is FALSE.".to_string(),
            ));
        }

        // Law: Host Binding must be explicit (Implicitly handled by Enum, but strictly enforced here)
        if self.host_binding.host_signature.trim().is_empty() {
            return Err(ConstitutionalError::Unmanaged(
                "Invariant 6.X Violation: Host Signature cannot be empty.".to_string(),
            ));
        }

        Ok(())
    }
}
