// ============================================================
// VANTAGE CORE v1.2.4 - Structural Sensor Engine with Symbol Graph
// ============================================================

/// Canonical version string for all Vantage JSON outputs.
/// Kit parses this to determine output schema version.
pub const VANTAGE_VERSION: &str = "1.2.4";

pub mod cognition;
pub mod drift;
pub mod fingerprint;
pub mod graph;
pub mod intent;
pub mod parser;

// Active exports (used by CLI + pipeline)
pub use cognition::{
    ClaimType, CognitiveClaim, Decision, InvariantEngine, InvariantRule, Pipeline, PipelineResult,
    Verdict,
};
pub use drift::{DriftItem, DriftReport, DriftStatus};
pub use graph::{Edge, EdgeType, SymbolGraph, SymbolNode};
pub use parser::{EpistemicParser, Language, get_parser};
pub use vantage_types::signal::{CognitiveSignal, Origin, SourceLocation};
pub use vantage_types::symbol::SymbolKind;
pub use vantage_types::FailureReason;

// Legacy: kept for backward compatibility, will be removed in v1.3.0
#[doc(hidden)]
pub use cognition::enforcer::{EnforcementDecision, ExecutionContext, enforce_claim};
#[doc(hidden)]
pub use fingerprint::hasher::Hasher;
