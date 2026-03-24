pub mod signal;
pub mod symbol;
pub mod hash;

pub use signal::{StructuralSignal, Origin};
pub use symbol::{SymbolKind, SymbolDefinition};
pub use hash::{HashAlgorithm, StructuralHash, SemanticHash};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use chrono::{DateTime, Utc};

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
    pub issued_at: DateTime<Utc>,
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
