pub mod alignment;
pub mod closure;
pub mod divergence;
pub mod market;
pub mod packet;
pub mod predictive;
pub mod stability;
pub mod state;
pub mod view;

pub use alignment::{AlignmentAction, AlignmentHint, AlignmentRelation, IncompatibilityReason};
pub use closure::{compose, ClosureGraph, Cluster, FederationState, PhaseTransition, detect_transition};
pub use divergence::EpochDivergenceState;
pub use market::{Capital, ClaimToken, MarketState, SettledToken};
pub use packet::FederationPacket;
pub use predictive::{
    PredictionCommitment, PredictionLedger, PredictionOutcome, RealityClaim, SettlementResult,
};
pub use stability::{
    Attractor, DivergenceEntropy, LyapunovState, PhaseDynamics, StabilityCondition,
    check_stability_theorem, compute_lyapunov_derivative,
};
pub use state::{CommitmentHash, FederationNodeState, NodeView};
pub use view::FederationView;

use vantage_core::{EpochId, NodeId};

pub trait VantageFederation {
    fn node_id(&self) -> &NodeId;
    fn current_epoch(&self) -> EpochId;

    fn broadcast_state(&self) -> FederationPacket;
    fn ingest(&mut self, packet: FederationPacket) -> FederationView;
    fn compute_divergence(&self) -> EpochDivergenceState;
    fn propose_alignment(&self, remote: &FederationPacket) -> Vec<AlignmentHint>;
}
