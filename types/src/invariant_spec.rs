//! # Invariant Spec — v1.2.5
//!
//! Machine-checkable architectural contracts for graph validation.
//! These are the "laws" that a codebase must obey.
//! Invariants are checked at `kit-vantage verify` time.
//!
//! ## Design rules
//! - Invariants are stateless — they validate a single graph snapshot
//! - Invariants are deterministic — same graph always passes/fails the same way
//! - Invariants do NOT mutate the graph

use serde::{Deserialize, Serialize};
use crate::intent::IntentInvariant;
use crate::symbol::SymbolKind;

/// A machine-checkable architectural invariant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvariantSpec {
    /// Unique identifier (e.g. "rust-core-has-arena")
    pub id: String,
    /// Human-readable description
    pub description: String,
    /// The rule that must always hold
    pub rule: InvariantRule,
    /// Scope of the invariant
    pub scope: InvariantScope,
}

/// The type of invariant rule to check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InvariantRule {
    /// Graph must contain at least N nodes
    NodeCountMin(usize),
    /// At least one node of the given SymbolKind must exist
    RequiredKind(SymbolKind),
    /// At least one node must have the given IntentInvariant
    RequiredIntent(IntentInvariant),
    /// A symbol with the given name must exist across multiple languages
    CrossLanguageParity(String),
    /// Graph hash must be reproducible (no non-determinism)
    HashStability,
}

/// Scope limits where an invariant applies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InvariantScope {
    /// Applies to the entire UnifiedGraph
    Global,
    /// Applies only to nodes of a specific language
    Language(String),
    /// Applies only to nodes matching an fq_name prefix
    Module(String),
}

/// Result of checking a single invariant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvariantResult {
    pub spec_id: String,
    pub passed: bool,
    pub message: String,
}

/// Collection of invariant check results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvariantReport {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub results: Vec<InvariantResult>,
}
