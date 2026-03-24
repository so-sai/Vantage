//! # Vantage Constitution
//!
//! This module defines the invariants and authoritative rules that govern
//! the behavior of the Vantage system.

pub trait Invariant {
    fn check(&self) -> Result<(), String>;
}

/// Invariant 6.2: Discovery != Acceptance
/// Explicit Human Acknowledgement is required for all state mutations or high-privilege discoveries.
pub struct HumanAcknowledgement(pub bool);

impl Invariant for HumanAcknowledgement {
    fn check(&self) -> Result<(), String> {
        if self.0 {
            Ok(())
        } else {
            Err("CONSTITUTIONAL VIOLATION: Invariant 6.2. Discovery requires explicit Human Acknowledgement.".to_string())
        }
    }
}
