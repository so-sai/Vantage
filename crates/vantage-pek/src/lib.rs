use std::sync::atomic::{AtomicU64, Ordering};
use serde::{Deserialize, Serialize};
use vantage_core::{KnowledgeMutation, CommitReceipt};

pub trait Attestation: Send + Sync {
    fn verify(&self) -> bool;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofCertificate {
    pub claim_id: String,
    pub proof_hash: [u8; 32],
    pub pcf_version: u32,
}

impl ProofCertificate {
    pub fn new(claim_id: String, proof_hash: [u8; 32], pcf_version: u32) -> Self {
        Self { claim_id, proof_hash, pcf_version }
    }
}

impl Attestation for ProofCertificate {
    fn verify(&self) -> bool {
        self.proof_hash != [0u8; 32] && self.pcf_version == 1
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemProof {
    BootstrapKernel,
    GCReconciliation,
    SchemaMigration,
    RuntimeInvariantRepair,
    Test,
}

impl Attestation for SystemProof {
    fn verify(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProofPolicy {
    Disabled,
    Advisory,
    Enforced,
    StrictCanonical,
}

pub struct MutationRequest<A: Attestation> {
    pub mutation: KnowledgeMutation,
    pub attestation: A,
}

impl<A: Attestation> MutationRequest<A> {
    pub fn new(mutation: KnowledgeMutation, attestation: A) -> Self {
        Self { mutation, attestation }
    }
}

pub struct TransactionRequest<A: Attestation> {
    pub mutations: Vec<KnowledgeMutation>,
    pub attestation: A,
}

impl<A: Attestation> TransactionRequest<A> {
    pub fn new(mutations: Vec<KnowledgeMutation>, attestation: A) -> Self {
        Self { mutations, attestation }
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum PEKError {
    #[error("PEK: Chứng thực không hợp lệ hoặc chữ ký bị giả mạo")]
    InvalidAttestation,
    #[error("PEK: Vi phạm chính sách an toàn hệ thống: {0}")]
    PolicyViolation(String),
    #[error("PEK: Lỗi thực thi ở tầng dưới: {0}")]
    RuntimeError(String),
}

#[derive(Default)]
pub struct PEKStats {
    pub admitted_count: AtomicU64,
    pub rejected_count: AtomicU64,
    pub advisory_warnings: AtomicU64,
}

impl PEKStats {
    pub fn new() -> Self {
        Self {
            admitted_count: AtomicU64::new(0),
            rejected_count: AtomicU64::new(0),
            advisory_warnings: AtomicU64::new(0),
        }
    }

    pub fn reset(&self) {
        self.admitted_count.store(0, Ordering::SeqCst);
        self.rejected_count.store(0, Ordering::SeqCst);
        self.advisory_warnings.store(0, Ordering::SeqCst);
    }
}

pub trait EpistemicExecutor {
    fn execute(&self, mutation: KnowledgeMutation) -> Result<CommitReceipt, String>;
    fn execute_transaction(&self, mutations: Vec<KnowledgeMutation>) -> Result<Vec<CommitReceipt>, String>;
}

pub struct ProofGate;

impl ProofGate {
    pub fn commit<A: Attestation, E: EpistemicExecutor>(
        request: MutationRequest<A>,
        policy: ProofPolicy,
        executor: &E,
        stats: &PEKStats,
    ) -> Result<CommitReceipt, PEKError> {
        let is_verified = request.attestation.verify();

        match policy {
            ProofPolicy::Disabled => {
                stats.admitted_count.fetch_add(1, Ordering::SeqCst);
                executor
                    .execute(request.mutation)
                    .map_err(|err| PEKError::RuntimeError(err))
            }
            ProofPolicy::Advisory => {
                if !is_verified {
                    stats.advisory_warnings.fetch_add(1, Ordering::SeqCst);
                }
                stats.admitted_count.fetch_add(1, Ordering::SeqCst);
                executor
                    .execute(request.mutation)
                    .map_err(|err| PEKError::RuntimeError(err))
            }
            ProofPolicy::Enforced => {
                if !is_verified {
                    stats.rejected_count.fetch_add(1, Ordering::SeqCst);
                    return Err(PEKError::InvalidAttestation);
                }
                stats.admitted_count.fetch_add(1, Ordering::SeqCst);
                executor
                    .execute(request.mutation)
                    .map_err(|err| PEKError::RuntimeError(err))
            }
            ProofPolicy::StrictCanonical => {
                if !is_verified {
                    stats.rejected_count.fetch_add(1, Ordering::SeqCst);
                    return Err(PEKError::InvalidAttestation);
                }
                if !Self::is_canonical(&request.mutation) {
                    stats.rejected_count.fetch_add(1, Ordering::SeqCst);
                    return Err(PEKError::PolicyViolation(format!(
                        "MutationId '{:?}' không đạt tiêu chuẩn chuẩn tắc (Bắt buộc phải có tiền tố 'mut_')",
                        request.mutation.mutation_id
                    )));
                }
                stats.admitted_count.fetch_add(1, Ordering::SeqCst);
                executor
                    .execute(request.mutation)
                    .map_err(|err| PEKError::RuntimeError(err))
            }
        }
    }

    pub fn commit_transaction<A: Attestation, E: EpistemicExecutor>(
        request: TransactionRequest<A>,
        policy: ProofPolicy,
        executor: &E,
        stats: &PEKStats,
    ) -> Result<Vec<CommitReceipt>, PEKError> {
        let is_verified = request.attestation.verify();

        match policy {
            ProofPolicy::Disabled => {
                stats.admitted_count.fetch_add(request.mutations.len() as u64, Ordering::SeqCst);
                executor
                    .execute_transaction(request.mutations)
                    .map_err(|err| PEKError::RuntimeError(err))
            }
            ProofPolicy::Advisory => {
                if !is_verified {
                    stats.advisory_warnings.fetch_add(1, Ordering::SeqCst);
                }
                stats.admitted_count.fetch_add(request.mutations.len() as u64, Ordering::SeqCst);
                executor
                    .execute_transaction(request.mutations)
                    .map_err(|err| PEKError::RuntimeError(err))
            }
            ProofPolicy::Enforced => {
                if !is_verified {
                    stats.rejected_count.fetch_add(1, Ordering::SeqCst);
                    return Err(PEKError::InvalidAttestation);
                }
                stats.admitted_count.fetch_add(request.mutations.len() as u64, Ordering::SeqCst);
                executor
                    .execute_transaction(request.mutations)
                    .map_err(|err| PEKError::RuntimeError(err))
            }
            ProofPolicy::StrictCanonical => {
                if !is_verified {
                    stats.rejected_count.fetch_add(1, Ordering::SeqCst);
                    return Err(PEKError::InvalidAttestation);
                }
                for mutation in &request.mutations {
                    if !Self::is_canonical(mutation) {
                        stats.rejected_count.fetch_add(1, Ordering::SeqCst);
                        return Err(PEKError::PolicyViolation(format!(
                            "Giao dịch bị từ chối: MutationId '{:?}' không chuẩn tắc.",
                            mutation.mutation_id
                        )));
                    }
                }
                stats.admitted_count.fetch_add(request.mutations.len() as u64, Ordering::SeqCst);
                executor
                    .execute_transaction(request.mutations)
                    .map_err(|err| PEKError::RuntimeError(err))
            }
        }
    }

    fn is_canonical(mutation: &KnowledgeMutation) -> bool {
        mutation.mutation_id.0.starts_with("mut_")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    use vantage_core::{AgentId, MutationId, MutationOp, ResourceId};

    struct MockExecutor;
    impl EpistemicExecutor for MockExecutor {
        fn execute(&self, m: KnowledgeMutation) -> Result<CommitReceipt, String> {
            Ok(CommitReceipt {
                tx_id: format!("tx_{}", m.mutation_id.0),
                actor: m.actor,
                timestamp: m.timestamp,
                invariant_hash: "mock_sha256".to_string(),
            })
        }

        fn execute_transaction(&self, mutations: Vec<KnowledgeMutation>) -> Result<Vec<CommitReceipt>, String> {
            mutations.into_iter().map(|m| {
                Ok(CommitReceipt {
                    tx_id: format!("tx_{}", m.mutation_id.0),
                    actor: m.actor,
                    timestamp: m.timestamp,
                    invariant_hash: "mock_sha256".to_string(),
                })
            }).collect()
        }
    }

    fn create_test_mutation(id_str: &str) -> KnowledgeMutation {
        KnowledgeMutation {
            mutation_id: MutationId(id_str.to_string()),
            actor: AgentId("pek_tester".to_string()),
            op: MutationOp::Delete { resource_id: ResourceId("unit:test".to_string()) },
            timestamp: SystemTime::now(),
        }
    }

    #[test]
    fn test_policy_disabled_always_admits() {
        let stats = PEKStats::new();
        let executor = MockExecutor;
        let mutation = create_test_mutation("invalid_id_format");
        let bad_certificate = ProofCertificate::new("claim_01".to_string(), [0u8; 32], 1);
        let req = MutationRequest::new(mutation, bad_certificate);

        let res = ProofGate::commit(req, ProofPolicy::Disabled, &executor, &stats);
        assert!(res.is_ok());
        assert_eq!(stats.admitted_count.load(Ordering::SeqCst), 1);
        assert_eq!(stats.rejected_count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_policy_advisory_warns_but_admits() {
        let stats = PEKStats::new();
        let executor = MockExecutor;
        let mutation = create_test_mutation("mut_valid_id");
        let bad_certificate = ProofCertificate::new("claim_01".to_string(), [0u8; 32], 1);
        let req = MutationRequest::new(mutation, bad_certificate);

        let res = ProofGate::commit(req, ProofPolicy::Advisory, &executor, &stats);
        assert!(res.is_ok());
        assert_eq!(stats.admitted_count.load(Ordering::SeqCst), 1);
        assert_eq!(stats.advisory_warnings.load(Ordering::SeqCst), 1);
        assert_eq!(stats.rejected_count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_policy_enforced_rejects_invalid_attestation() {
        let stats = PEKStats::new();
        let executor = MockExecutor;
        let mutation = create_test_mutation("mut_valid_id");
        let bad_certificate = ProofCertificate::new("claim_01".to_string(), [0u8; 32], 1);
        let req = MutationRequest::new(mutation, bad_certificate);

        let res = ProofGate::commit(req, ProofPolicy::Enforced, &executor, &stats);
        assert_eq!(res.err(), Some(PEKError::InvalidAttestation));
        assert_eq!(stats.admitted_count.load(Ordering::SeqCst), 0);
        assert_eq!(stats.rejected_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_policy_enforced_admits_valid_certificate() {
        let stats = PEKStats::new();
        let executor = MockExecutor;
        let mutation = create_test_mutation("mut_valid_id");
        let mut hash = [0u8; 32];
        hash[0] = 0xFF;
        let valid_certificate = ProofCertificate::new("claim_01".to_string(), hash, 1);
        let req = MutationRequest::new(mutation, valid_certificate);

        let res = ProofGate::commit(req, ProofPolicy::Enforced, &executor, &stats);
        assert!(res.is_ok());
        assert_eq!(stats.admitted_count.load(Ordering::SeqCst), 1);
        assert_eq!(stats.rejected_count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_policy_strict_canonical_rejects_non_canonical_id() {
        let stats = PEKStats::new();
        let executor = MockExecutor;
        let mutation = create_test_mutation("bad_id_prefix_without_mut");
        let mut hash = [0u8; 32];
        hash[0] = 0xFF;
        let valid_certificate = ProofCertificate::new("claim_01".to_string(), hash, 1);
        let req = MutationRequest::new(mutation, valid_certificate);

        let res = ProofGate::commit(req, ProofPolicy::StrictCanonical, &executor, &stats);
        assert!(matches!(res.err(), Some(PEKError::PolicyViolation(_))));
        assert_eq!(stats.admitted_count.load(Ordering::SeqCst), 0);
        assert_eq!(stats.rejected_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_pcf_version_mismatch_fails_verification() {
        let mut hash = [0u8; 32];
        hash[0] = 0xFF;
        let bad_version_cert = ProofCertificate::new("claim_01".to_string(), hash, 2);
        assert!(!bad_version_cert.verify());
    }

    #[test]
    fn test_commit_transaction_enforced_admits_valid_batch() {
        let stats = PEKStats::new();
        let executor = MockExecutor;
        let mutations = vec![
            create_test_mutation("mut_a"),
            create_test_mutation("mut_b"),
        ];
        let tx_req = TransactionRequest::new(mutations, SystemProof::Test);
        let res = ProofGate::commit_transaction(tx_req, ProofPolicy::Enforced, &executor, &stats);
        assert!(res.is_ok());
        assert_eq!(res.unwrap().len(), 2);
        assert_eq!(stats.admitted_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_commit_transaction_strict_canonical_rejects_non_canonical() {
        let stats = PEKStats::new();
        let executor = MockExecutor;
        let mutations = vec![
            create_test_mutation("bad_prefix"),
        ];
        let mut hash = [0u8; 32];
        hash[0] = 0xFF;
        let cert = ProofCertificate::new("claim_01".to_string(), hash, 1);
        let tx_req = TransactionRequest::new(mutations, cert);
        let res = ProofGate::commit_transaction(tx_req, ProofPolicy::StrictCanonical, &executor, &stats);
        assert!(matches!(res.err(), Some(PEKError::PolicyViolation(_))));
        assert_eq!(stats.rejected_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_commit_transaction_disabled_bypasses_attestation() {
        let stats = PEKStats::new();
        let executor = MockExecutor;
        let mutations = vec![create_test_mutation("bad")];
        let bad_cert = ProofCertificate::new("x".into(), [0u8; 32], 1);
        let tx_req = TransactionRequest::new(mutations, bad_cert);
        let res = ProofGate::commit_transaction(tx_req, ProofPolicy::Disabled, &executor, &stats);
        assert!(res.is_ok());
        assert_eq!(stats.admitted_count.load(Ordering::SeqCst), 1);
    }
}
