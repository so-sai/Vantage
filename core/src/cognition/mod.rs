pub mod claim;
pub mod decision;
pub mod enforcer;
pub mod label;
pub mod lens;
pub mod pipeline;

#[cfg(test)]
mod tests_rigorous;

pub use claim::{ClaimType, CognitiveClaim, Seal};
pub use decision::{Decision, Invariant, InvariantEngine, InvariantRule, Verdict};
pub use enforcer::{EnforcementDecision, ExecutionContext, enforce_claim};
pub use label::CognitiveLabel;
pub use lens::{Block, BlockType, LensData};
pub use pipeline::{Pipeline, PipelineResult};
pub use vantage_types::signal::{CognitiveSignal, Origin, SourceLocation};
pub use vantage_types::symbol::SymbolKind;
