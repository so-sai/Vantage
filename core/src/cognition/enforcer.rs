use vantage_types::{AuthorityLevel, Manifest};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionContext {
    Sandbox,
    Lab,
    Production,
}

pub enum EnforcementDecision {
    Allow,
    Reject(String),
}

/// Enforces constitutional invariants on a Vantage Manifest.
pub fn enforce_claim(manifest: &Manifest, ctx: ExecutionContext) -> EnforcementDecision {
    // 1. Basic validation from vantage-types
    if let Err(e) = manifest.validate() {
        return EnforcementDecision::Reject(e.to_string());
    }

    // 2. Context-specific enforcement
    match ctx {
        ExecutionContext::Production => {
            if manifest.authority.authority_level != AuthorityLevel::Privileged {
                return EnforcementDecision::Reject(
                    "F2_UNAUTHORIZED: Production requires Privileged authority.".to_string(),
                );
            }
        }
        ExecutionContext::Sandbox => {
            // Sandbox is more permissive
        }
        ExecutionContext::Lab => {
            // Lab allows Trusted or Privileged
            if manifest.authority.authority_level == AuthorityLevel::Sandbox {
                return EnforcementDecision::Reject(
                    "F2_UNAUTHORIZED: Lab requires at least Trusted authority.".to_string(),
                );
            }
        }
    }

    EnforcementDecision::Allow
}
