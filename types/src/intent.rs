//! # Architectural Intent Overlay — v1.2.5
//!
//! Language-agnostic intent annotation extracted from structured comments.
//! Attaches WHY semantics to structural nodes without mutating topology.
//!
//! ## Source syntax (language-agnostic YAML-style block)
//! ```rust
//! // vantage:
//! //   invariant: AppendOnly
//! //   reason: Prevent rollback corruption
//! ```
//!
//! ## Design rules
//! - Intent is metadata only — NEVER changes graph topology
//! - Structural hash is independent of intent hash
//! - Intent is optional — graph is valid with or without it

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Architectural invariant type — the WHY behind a structural constraint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IntentInvariant {
    /// Data can only be appended, never modified or deleted
    AppendOnly,
    /// Execution order must be deterministic
    DeterministicOrdering,
    /// Must be safe to call from multiple threads
    ThreadSafe,
    /// Function or method must have no side effects
    NoSideEffects,
    /// Multiple invocations produce the same result
    Idempotent,
    /// No heap allocation in hot path
    ZeroAllocation,
    /// No mutable state (pure function)
    Stateless,
    /// Custom user-defined invariant
    Custom(String),
}

/// A constraint on architectural relationships.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Constraint {
    pub kind: ConstraintKind,
    pub description: String,
}

/// Type of architectural constraint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConstraintKind {
    MustImport(String),
    MustNotImport(String),
    MustExtend(String),
    MustBeSealed,
    MustBeStateless,
}

/// Intent overlay attached to a structural node.
/// Maps to the YAML-style `# vantage:` / `// vantage:` comment block.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntentOverlay {
    /// Primary invariant being enforced
    pub invariant: IntentInvariant,
    /// Human-readable justification
    pub reason: String,
    /// Additional constraints
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub constraints: Vec<Constraint>,
    /// Owning team or individual
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    /// Arbitrary metadata (pass-through from comment)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}
