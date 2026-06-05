use std::collections::HashSet;
use vantage_core::KnowledgeMutation;
use vantage_pek::{Attestation, VerifiedCertificate};

/// A lightweight identity identifier for TIA-1 Phase 1.
/// Will be replaced by full Identity type in later phases.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct IdentityId(pub String);

/// Digest of the policy used during authorization.
/// Two certificates with different policy_digests were authorized
/// under different policy versions.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct PolicyDigest(pub String);

/// Provenance context: who authorized this certificate and under which policy.
#[derive(Debug, Clone)]
pub struct AuthorityContext {
    pub issuer: IdentityId,
    pub policy_digest: PolicyDigest,
}

/// A certificate that has been both verified AND authorized.
/// This is the only certificate type the runtime admits.
///
/// Typestate chain:
///   ProofCertificate → verify() → VerifiedCertificate → authorize() → AuthorizedCertificate
#[derive(Debug, Clone)]
pub struct AuthorizedCertificate {
    inner: VerifiedCertificate,
    authority: AuthorityContext,
}

impl AuthorizedCertificate {
    pub fn new(verified: VerifiedCertificate, authority: AuthorityContext) -> Self {
        Self { inner: verified, authority }
    }

    pub fn claim_id(&self) -> &str {
        self.inner.claim_id()
    }

    pub fn proof_hash(&self) -> &[u8; 32] {
        self.inner.proof_hash()
    }

    pub fn issuer(&self) -> &IdentityId {
        &self.authority.issuer
    }

    pub fn policy_digest(&self) -> &PolicyDigest {
        &self.authority.policy_digest
    }

    pub fn authority(&self) -> &AuthorityContext {
        &self.authority
    }
}

impl Attestation for AuthorizedCertificate {
    fn verify(&self) -> bool {
        true
    }
}

/// A mutation bound to its authorized certificate.
/// Prevents mismatch between mutation and authority at the type level.
#[derive(Debug, Clone)]
pub struct AuthorizedMutation {
    pub mutation: KnowledgeMutation,
    pub certificate: AuthorizedCertificate,
}

impl AuthorizedMutation {
    pub fn new(mutation: KnowledgeMutation, certificate: AuthorizedCertificate) -> Self {
        Self { mutation, certificate }
    }
}

/// Phase 1 trust evaluator: determines whether a verified certificate
/// is authorized to admit a mutation.
pub trait TrustEvaluator: Send + Sync {
    fn authorize(&self, cert: VerifiedCertificate) -> Result<AuthorizedCertificate, String>;
}

/// Phase 1 revocation registry: a simple set of revoked identities.
#[derive(Debug, Default)]
pub struct RevocationRegistry {
    revoked: HashSet<IdentityId>,
}

impl RevocationRegistry {
    pub fn new() -> Self {
        Self { revoked: HashSet::new() }
    }

    pub fn revoke(&mut self, identity: IdentityId) {
        self.revoked.insert(identity);
    }

    pub fn unrevoke(&mut self, identity: &IdentityId) {
        self.revoked.remove(identity);
    }

    pub fn is_revoked(&self, identity: &IdentityId) -> bool {
        self.revoked.contains(identity)
    }
}

/// Phase 1 static trust policy: a simple hardcoded policy.
/// In later phases this will be replaced by a configurable policy engine.
pub struct StaticTrustPolicy {
    pub trusted_issuer: IdentityId,
    pub policy_digest: PolicyDigest,
    pub revocation: RevocationRegistry,
}

impl StaticTrustPolicy {
    pub fn new(trusted_issuer: IdentityId, policy_digest: PolicyDigest) -> Self {
        Self { trusted_issuer, policy_digest, revocation: RevocationRegistry::new() }
    }
}

impl TrustEvaluator for StaticTrustPolicy {
    fn authorize(&self, cert: VerifiedCertificate) -> Result<AuthorizedCertificate, String> {
        let issuer = &self.trusted_issuer;

        if self.revocation.is_revoked(issuer) {
            return Err(format!("Issuer {:?} has been revoked", issuer));
        }

        let authority = AuthorityContext {
            issuer: issuer.clone(),
            policy_digest: self.policy_digest.clone(),
        };

        Ok(AuthorizedCertificate::new(cert, authority))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vantage_core::{AgentId, MutationId, MutationOp, ResourceId};
    use vantage_pek::ProofCertificate;
    use std::time::SystemTime;

    fn make_verified_cert() -> VerifiedCertificate {
        let mut hash = [0u8; 32];
        hash[0] = 0xFF;
        let cert = ProofCertificate::new("claim_01".into(), hash, 1);
        cert.verify().expect("valid cert")
    }

    #[test]
    fn test_authorize_verified_certificate() {
        let cert = make_verified_cert();
        let policy = StaticTrustPolicy::new(
            IdentityId("alice".into()),
            PolicyDigest("policy-v1".into()),
        );
        let authorized = policy.authorize(cert).expect("should authorize");
        assert_eq!(authorized.issuer().0, "alice");
        assert_eq!(authorized.policy_digest().0, "policy-v1");
    }

    #[test]
    fn test_authorize_revoked_issuer_fails() {
        let cert = make_verified_cert();
        let issuer = IdentityId("alice".into());
        let mut policy = StaticTrustPolicy::new(
            issuer.clone(),
            PolicyDigest("policy-v1".into()),
        );
        policy.revocation.revoke(issuer);
        let result = policy.authorize(cert);
        assert!(result.is_err());
    }

    #[test]
    fn test_authorized_certificate_passes_attestation() {
        let cert = make_verified_cert();
        let policy = StaticTrustPolicy::new(
            IdentityId("bob".into()),
            PolicyDigest("policy-v1".into()),
        );
        let authorized = policy.authorize(cert).unwrap();
        assert!(authorized.verify());
    }

    #[test]
    fn test_authorized_mutation_binds_cert_and_mutation() {
        let cert = make_verified_cert();
        let policy = StaticTrustPolicy::new(
            IdentityId("bob".into()),
            PolicyDigest("policy-v1".into()),
        );
        let authorized_cert = policy.authorize(cert).unwrap();

        let mutation = KnowledgeMutation {
            mutation_id: MutationId("mut_01".into()),
            actor: AgentId("test".into()),
            op: MutationOp::Insert {
                resource_id: ResourceId("unit:test".into()),
                payload: "data".into(),
            },
            timestamp: SystemTime::now(),
        };

        let auth_mutation = AuthorizedMutation::new(mutation.clone(), authorized_cert);
        assert_eq!(auth_mutation.mutation.mutation_id.0, "mut_01");
    }

    #[test]
    fn test_revocation_registry() {
        let mut registry = RevocationRegistry::new();
        let id = IdentityId("mallory".into());
        assert!(!registry.is_revoked(&id));
        registry.revoke(id.clone());
        assert!(registry.is_revoked(&id));
        registry.unrevoke(&id);
        assert!(!registry.is_revoked(&id));
    }
}
