use std::collections::HashMap;
use std::sync::Mutex;
use std::time::SystemTime;
use tracing::warn;
use vantage_core::{
    CommitReceipt, CommitResult, EpochId, EpochState, ExecutionEnvelope, EpistemicInvariant,
    EpistemicReader, InvariantContext, InvariantViolation, KnowledgeMutation, MutationOp, ResourceId
};
use vantage_pek::EpistemicExecutor;
use vantage_tx::{TransactionalView, TransactionDAG};

#[derive(Debug, Clone)]
pub struct Revision {
    pub payload: Option<String>,
    pub timestamp: SystemTime,
    pub invariant_hash: String,
}

#[derive(Default)]
pub struct TemporalIndex {
    pub history: HashMap<ResourceId, Vec<Revision>>,
}

impl EpistemicReader for TemporalIndex {
    fn read_unit(&self, id: &ResourceId) -> Option<String> {
        self.history
            .get(id)
            .and_then(|revs| revs.last())
            .and_then(|rev| rev.payload.clone())
    }

    fn exists(&self, id: &ResourceId) -> bool {
        self.history
            .get(id)
            .and_then(|revs| revs.last())
            .map(|rev| rev.payload.is_some())
            .unwrap_or(false)
    }
}

impl EpistemicReader for VantageRuntime {
    fn read_unit(&self, id: &ResourceId) -> Option<String> {
        self.index.lock().unwrap().read_unit(id)
    }

    fn exists(&self, id: &ResourceId) -> bool {
        self.index.lock().unwrap().exists(id)
    }
}

pub struct NoDuplicateUnitInvariant;

impl EpistemicInvariant for NoDuplicateUnitInvariant {
    fn name(&self) -> &str {
        "NoDuplicateUnit"
    }

    fn validate<'a>(&self, ctx: &InvariantContext<'a, dyn EpistemicReader + 'a>) -> Result<(), InvariantViolation> {
        if let MutationOp::Insert { resource_id, .. } = &ctx.proposal.op {
            if ctx.current_world_view.exists(resource_id) {
                return Err(InvariantViolation::Contradiction {
                    reason: format!("Resource {:?} already exists.", resource_id),
                });
            }
        }
        Ok(())
    }
}

/// Opaque execution payload — constructed by daemon from an AuthorizedMutation.
/// Runtime does NOT inspect governance internals (issuer, policy_digest, etc.).
/// This is the TIA-2 boundary: runtime is an opaque execution kernel.
pub struct ExecutionPayload {
    pub mutation: KnowledgeMutation,
    pub envelope: ExecutionEnvelope,
}

impl ExecutionPayload {
    pub fn new(mutation: KnowledgeMutation, envelope: ExecutionEnvelope) -> Self {
        Self { mutation, envelope }
    }
}

/// TIA-2: Temporal execution state — tracks epoch and sequence monotonicity.
/// Protected by Mutex for interior mutability (runtime uses &self API).
pub struct TemporalState {
    pub current_epoch: EpochId,
    pub last_sequence: u64,
    pub epoch_state: EpochState,
}

impl TemporalState {
    pub fn new(epoch: EpochId) -> Self {
        Self { current_epoch: epoch, last_sequence: 0, epoch_state: EpochState::Active }
    }
}

pub struct VantageRuntime {
    index: Mutex<TemporalIndex>,
    invariants: Vec<Box<dyn EpistemicInvariant>>,
    temporal: Mutex<TemporalState>,
}

impl VantageRuntime {
    /// Access the temporal index (read-only operations).
    /// Direct mutation outside PEK is deprecated.
    pub fn index(&self) -> &Mutex<TemporalIndex> {
        &self.index
    }

    /// Access the invariant list.
    pub fn invariants(&self) -> &[Box<dyn EpistemicInvariant>] {
        &self.invariants
    }

    pub fn new() -> Self {
        Self {
            index: Mutex::new(TemporalIndex::default()),
            invariants: vec![Box::new(NoDuplicateUnitInvariant)],
            temporal: Mutex::new(TemporalState::new(EpochId(1))),
        }
    }

    pub fn with_epoch(epoch: EpochId) -> Self {
        Self {
            index: Mutex::new(TemporalIndex::default()),
            invariants: vec![Box::new(NoDuplicateUnitInvariant)],
            temporal: Mutex::new(TemporalState::new(epoch)),
        }
    }

    /// PRN-1: Lock the current epoch — no new payloads accepted after this point.
    /// Active → Locked. In-flight completion still allowed.
    pub fn lock_epoch(&self) -> Result<EpochId, String> {
        let mut temporal = self.temporal.lock().map_err(|e| e.to_string())?;
        match temporal.epoch_state {
            EpochState::Active => {
                temporal.epoch_state = EpochState::Locked;
                Ok(temporal.current_epoch)
            }
            _ => Err(format!(
                "Cannot lock epoch {:?}: state is {:?}",
                temporal.current_epoch, temporal.epoch_state
            )),
        }
    }

    /// PRN-1: Commit the current epoch and transition to a new one.
    /// Locked → Committed (old epoch sealed). New epoch becomes Active.
    /// Returns CommitResult with quorum info (single-node: threshold = 1).
    pub fn commit_epoch(&self, new_epoch: EpochId) -> Result<CommitResult, String> {
        let mut temporal = self.temporal.lock().map_err(|e| e.to_string())?;
        match temporal.epoch_state {
            EpochState::Locked => {
                let old_epoch = temporal.current_epoch;
                temporal.current_epoch = new_epoch;
                temporal.last_sequence = 0;
                temporal.epoch_state = EpochState::Active;
                Ok(CommitResult::new(old_epoch, true, 1, 1))
            }
            _ => Err(format!(
                "Cannot commit epoch {:?}: state is {:?}",
                temporal.current_epoch, temporal.epoch_state
            )),
        }
    }

    /// TIA-2: Commit authorized mutations.
    /// Accepts ExecutionPayload (opaque — no governance internals leaked).
    /// Runtime validates:
    ///   - epoch state (only Active accepts new payloads)
    ///   - epoch consistency (all payloads must match current epoch)
    ///   - sequence monotonicity (strictly increasing)
    /// This is the ONLY public mutation API from runtime.
    pub fn commit_authorized(&self, mutations: Vec<ExecutionPayload>) -> Result<Vec<CommitReceipt>, String> {
        // Phase 1: validate temporal envelope (epoch state + match + sequence monotonic)
        {
            let mut temporal = self.temporal.lock().map_err(|e| e.to_string())?;

            // Only Active epoch accepts new payloads
            if temporal.epoch_state != EpochState::Active {
                return Err(format!(
                    "Epoch {:?} is {:?}: not accepting new payloads",
                    temporal.current_epoch, temporal.epoch_state
                ));
            }

            for payload in &mutations {
                if payload.envelope.epoch != temporal.current_epoch {
                    return Err(format!(
                        "TIA-2 epoch mismatch: expected {:?}, got {:?}",
                        temporal.current_epoch, payload.envelope.epoch
                    ));
                }
                if payload.envelope.sequence <= temporal.last_sequence {
                    return Err(format!(
                        "TIA-2 sequence violation: {} <= last sequence {}",
                        payload.envelope.sequence, temporal.last_sequence
                    ));
                }
                temporal.last_sequence = payload.envelope.sequence;
            }
        }

        let raw: Vec<KnowledgeMutation> = mutations.into_iter().map(|ep| ep.mutation).collect();
        self.commit_transaction(raw)
    }

    /// Multi-mutation DAG transaction with full atomicity.
    /// This is `pub(crate)` in PEK-2C — external callers must use `ProofGate::commit()`.
    #[deprecated(
        since = "0.97",
        note = "Direct commit_transaction bypasses ProofGate. Use ProofGate::commit() or ProofGate::commit_transaction() instead."
    )]
    pub(crate) fn commit_transaction(&self, mutations: Vec<KnowledgeMutation>) -> Result<Vec<CommitReceipt>, String> {
        warn!(
            target: "pek.bypass",
            "PEK BYPASS: VantageRuntime::commit_transaction called directly without ProofGate ({} mutations)",
            mutations.len()
        );
        let dag = TransactionDAG::compile(mutations)?;
        let mut index_guard = self.index.lock().map_err(|e| e.to_string())?;
        let execution_order = dag.topological_sort();

        // Phase 1: simulate all mutations on virtual view, validate invariants
        let pending = {
            let mut virtual_view = TransactionalView::new(&*index_guard);
            let mut pending = Vec::new();

            for mutation_id in &execution_order {
                let node = &dag.nodes[mutation_id];
                let mutation = &node.0;

                let ctx = InvariantContext {
                    current_world_view: &virtual_view as &dyn EpistemicReader,
                    proposal: mutation,
                };
                for invariant in &self.invariants {
                    invariant
                        .validate(&ctx)
                        .map_err(|e| format!("Aborted at {:?}: {:?}", mutation_id, e))?;
                }

                let resource_id = match &mutation.op {
                    MutationOp::Insert { resource_id, payload } => {
                        virtual_view.stage_write(resource_id.clone(), payload.clone());
                        resource_id.clone()
                    }
                    MutationOp::Delete { resource_id } => {
                        virtual_view.stage_delete(resource_id.clone());
                        resource_id.clone()
                    }
                };

                let staged_content = virtual_view.overlay.get(&resource_id).cloned().flatten();
                let timestamp = mutation.timestamp;
                let invariant_hash = format!("sha256:pass:{}", self.invariants.len());

                pending.push((
                    resource_id,
                    mutation.mutation_id.0.clone(),
                    mutation.actor.clone(),
                    staged_content,
                    timestamp,
                    invariant_hash,
                ));
            }

            pending
        };

        // Phase 2: commit all revisions to physical index
        let mut receipts = Vec::new();
        for (resource_id, tx_id, actor, staged_content, timestamp, invariant_hash) in pending {
            let revision = Revision {
                payload: staged_content,
                timestamp,
                invariant_hash: invariant_hash.clone(),
            };
            index_guard
                .history
                .entry(resource_id)
                .or_insert_with(Vec::new)
                .push(revision);

            receipts.push(CommitReceipt {
                tx_id,
                actor,
                timestamp,
                invariant_hash,
            });
        }

        Ok(receipts)
    }
}

impl EpistemicExecutor for VantageRuntime {
    fn execute(&self, mutation: KnowledgeMutation) -> Result<CommitReceipt, String> {
        let mut receipts = self.commit_transaction(vec![mutation])?;
        receipts.pop().ok_or_else(|| "Empty receipt".to_string())
    }

    fn execute_transaction(&self, mutations: Vec<KnowledgeMutation>) -> Result<Vec<CommitReceipt>, String> {
        self.commit_transaction(mutations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vantage_core::{AgentId, CommitResult, EpochId, EpochState, ExecutionEnvelope, LogicalTime, MutationId, MutationOp, ResourceId};
    use vantage_pek::{MutationRequest, TransactionRequest, ProofGate, ProofPolicy, SystemProof, PEKStats};

    #[test]
    fn test_vantage_runtime_integration_flow() {
        let runtime = VantageRuntime::new();
        let stats = PEKStats::new();
        let actor = AgentId("developer_01".to_string());
        let unit_id = ResourceId("unit:core_module".to_string());

        let mutation = KnowledgeMutation {
            mutation_id: MutationId("mut_01".to_string()),
            actor: actor.clone(),
            op: MutationOp::Insert {
                resource_id: unit_id.clone(),
                payload: "fn main() { println!(\"Vantage\"); }".to_string(),
            },
            timestamp: SystemTime::now(),
        };

        let request = MutationRequest::new(mutation, SystemProof::Test);
        let receipt_result = ProofGate::commit(request, ProofPolicy::Enforced, &runtime, &stats);
        assert!(receipt_result.is_ok());

        {
            let index = runtime.index().lock().unwrap();
            assert!(index.exists(&unit_id));
            let content = index.read_unit(&unit_id);
            assert_eq!(content, Some("fn main() { println!(\"Vantage\"); }".to_string()));
        }

        let duplicate_mutation = KnowledgeMutation {
            mutation_id: MutationId("mut_02".to_string()),
            actor: actor.clone(),
            op: MutationOp::Insert {
                resource_id: unit_id.clone(),
                payload: "fn override_main() {}".to_string(),
            },
            timestamp: SystemTime::now(),
        };

        let dup_request = MutationRequest::new(duplicate_mutation, SystemProof::Test);
        let dup_result = ProofGate::commit(dup_request, ProofPolicy::Enforced, &runtime, &stats);
        assert!(dup_result.is_err());
    }

    #[test]
    fn test_delete_nonexistent_resource() {
        let runtime = VantageRuntime::new();
        let stats = PEKStats::new();
        let actor = AgentId("test".to_string());
        let unit_id = ResourceId("unit:ghost".to_string());

        let mutation = KnowledgeMutation {
            mutation_id: MutationId("mut_del".to_string()),
            actor,
            op: MutationOp::Delete {
                resource_id: unit_id,
            },
            timestamp: SystemTime::now(),
        };

        let request = MutationRequest::new(mutation, SystemProof::Test);
        let result = ProofGate::commit(request, ProofPolicy::Enforced, &runtime, &stats);
        assert!(result.is_ok());
    }

    #[test]
    #[allow(deprecated)]
    fn test_progressive_transaction_dag_flow() {
        let runtime = VantageRuntime::new();
        let actor = AgentId("developer_01".to_string());

        let unit_a = ResourceId("unit:module_a".to_string());
        let unit_b = ResourceId("unit:module_b".to_string());

        let mutations = vec![
            KnowledgeMutation {
                mutation_id: MutationId("mut_a".to_string()),
                actor: actor.clone(),
                op: MutationOp::Insert {
                    resource_id: unit_a.clone(),
                    payload: "module_a_code".to_string(),
                },
                timestamp: SystemTime::now(),
            },
            KnowledgeMutation {
                mutation_id: MutationId("mut_b".to_string()),
                actor: actor.clone(),
                op: MutationOp::Insert {
                    resource_id: unit_b.clone(),
                    payload: "module_b_code".to_string(),
                },
                timestamp: SystemTime::now(),
            },
        ];

        let tx_result = runtime.commit_transaction(mutations);
        assert!(tx_result.is_ok());
        let receipts = tx_result.unwrap();
        assert_eq!(receipts.len(), 2);

        assert!(runtime.exists(&unit_a));
        assert!(runtime.exists(&unit_b));

        // Atomicity: bad transaction must fully roll back
        let bad_mutations = vec![
            KnowledgeMutation {
                mutation_id: MutationId("mut_c_valid".to_string()),
                actor: actor.clone(),
                op: MutationOp::Insert {
                    resource_id: ResourceId("unit:module_c".to_string()),
                    payload: "module_c_code".to_string(),
                },
                timestamp: SystemTime::now(),
            },
            KnowledgeMutation {
                mutation_id: MutationId("mut_d_duplicate".to_string()),
                actor: actor.clone(),
                op: MutationOp::Insert {
                    resource_id: unit_a.clone(),
                    payload: "duplicate_a".to_string(),
                },
                timestamp: SystemTime::now(),
            },
        ];

        let bad_tx_result = runtime.commit_transaction(bad_mutations);
        assert!(bad_tx_result.is_err());
        assert!(!runtime.exists(&ResourceId("unit:module_c".to_string())));
    }

    #[test]
    fn test_attested_transaction_via_proof_gate() {
        let runtime = VantageRuntime::new();
        let stats = PEKStats::new();
        let actor = AgentId("developer_01".to_string());

        let unit_a = ResourceId("unit:module_a".to_string());
        let unit_b = ResourceId("unit:module_b".to_string());

        let mutations = vec![
            KnowledgeMutation {
                mutation_id: MutationId("mut_a".to_string()),
                actor: actor.clone(),
                op: MutationOp::Insert { resource_id: unit_a.clone(), payload: "code_a".to_string() },
                timestamp: SystemTime::now(),
            },
            KnowledgeMutation {
                mutation_id: MutationId("mut_b".to_string()),
                actor: actor.clone(),
                op: MutationOp::Insert { resource_id: unit_b.clone(), payload: "code_b".to_string() },
                timestamp: SystemTime::now(),
            },
        ];

        let tx_request = TransactionRequest::new(mutations, SystemProof::Test);
        let receipts_result = ProofGate::commit_transaction(
            tx_request,
            ProofPolicy::StrictCanonical,
            &runtime,
            &stats,
        );

        assert!(receipts_result.is_ok());
        let receipts = receipts_result.unwrap();
        assert_eq!(receipts.len(), 2);

        assert_eq!(stats.admitted_count.load(std::sync::atomic::Ordering::SeqCst), 2);
        assert_eq!(stats.rejected_count.load(std::sync::atomic::Ordering::SeqCst), 0);

        assert!(runtime.exists(&unit_a));
        assert!(runtime.exists(&unit_b));
    }

    // --- TIA-2 temporal enforcement tests ---

    fn make_payload(mutation: KnowledgeMutation, epoch: EpochId, sequence: u64) -> ExecutionPayload {
        let envelope = ExecutionEnvelope::new(epoch, sequence, LogicalTime::new(sequence));
        ExecutionPayload::new(mutation, envelope)
    }

    fn test_mutation(id: &str, resource: &str) -> KnowledgeMutation {
        KnowledgeMutation {
            mutation_id: MutationId(id.to_string()),
            actor: AgentId("temporal_test".to_string()),
            op: MutationOp::Insert {
                resource_id: ResourceId(format!("unit:{}", resource)),
                payload: "test".to_string(),
            },
            timestamp: SystemTime::now(),
        }
    }

    #[test]
    fn test_temporal_valid_payload_accepted() {
        let runtime = VantageRuntime::with_epoch(EpochId(1));
        let payload = make_payload(test_mutation("mut_valid", "t_valid"), EpochId(1), 1);
        let result = runtime.commit_authorized(vec![payload]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_temporal_epoch_mismatch_rejected() {
        let runtime = VantageRuntime::with_epoch(EpochId(1));
        let payload = make_payload(test_mutation("mut_bad_epoch", "t_bad_epoch"), EpochId(2), 1);
        let result = runtime.commit_authorized(vec![payload]);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("epoch mismatch"), "error: {}", err);
    }

    #[test]
    fn test_temporal_sequence_monotonic_violation_rejected() {
        let runtime = VantageRuntime::with_epoch(EpochId(1));
        let p1 = make_payload(test_mutation("mut_seq1", "t_seq1"), EpochId(1), 2);
        let p2 = make_payload(test_mutation("mut_seq1_dup", "t_seq2"), EpochId(1), 1);
        let result = runtime.commit_authorized(vec![p1, p2]);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("sequence violation"), "error: {}", err);
    }

    #[test]
    fn test_temporal_sequence_tracked_across_calls() {
        let runtime = VantageRuntime::with_epoch(EpochId(1));
        let p1 = make_payload(test_mutation("mut_first", "t_first"), EpochId(1), 1);
        assert!(runtime.commit_authorized(vec![p1]).is_ok());

        let p2 = make_payload(test_mutation("mut_second", "t_second"), EpochId(1), 2);
        assert!(runtime.commit_authorized(vec![p2]).is_ok());

        let p3 = make_payload(test_mutation("mut_replay", "t_third"), EpochId(1), 2);
        let result = runtime.commit_authorized(vec![p3]);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("sequence violation"), "error: {}", err);
    }

    // --- PRN-1 epoch lifecycle tests ---

    #[test]
    fn test_epoch_starts_active() {
        let runtime = VantageRuntime::with_epoch(EpochId(1));
        let payload = make_payload(test_mutation("mut_active", "t_active"), EpochId(1), 1);
        assert!(runtime.commit_authorized(vec![payload]).is_ok());
    }

    #[test]
    fn test_epoch_lock_rejects_new_payloads() {
        let runtime = VantageRuntime::with_epoch(EpochId(1));
        // Lock the epoch
        let locked = runtime.lock_epoch().expect("lock should succeed");
        assert_eq!(locked, EpochId(1));

        // New payloads should be rejected
        let payload = make_payload(test_mutation("mut_locked", "t_locked"), EpochId(1), 1);
        let result = runtime.commit_authorized(vec![payload]);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("Locked"), "error: {}", err);
    }

    #[test]
    fn test_epoch_lock_twice_fails() {
        let runtime = VantageRuntime::with_epoch(EpochId(1));
        assert!(runtime.lock_epoch().is_ok());
        let result = runtime.lock_epoch();
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("Cannot lock"), "error: {}", err);
    }

    #[test]
    fn test_epoch_commit_transitions_to_new_epoch() {
        let runtime = VantageRuntime::with_epoch(EpochId(1));
        runtime.lock_epoch().expect("lock");

        let result = runtime.commit_epoch(EpochId(2)).expect("commit");
        assert!(result.success);
        assert_eq!(result.epoch, EpochId(1));

        // New epoch (2) should accept payloads
        let payload = make_payload(test_mutation("mut_epoch2", "t_epoch2"), EpochId(2), 1);
        assert!(runtime.commit_authorized(vec![payload]).is_ok());
    }

    #[test]
    fn test_epoch_commit_without_lock_fails() {
        let runtime = VantageRuntime::with_epoch(EpochId(1));
        let result = runtime.commit_epoch(EpochId(2));
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("Cannot commit"), "error: {}", err);
    }

    #[test]
    fn test_epoch_full_lifecycle() {
        let runtime = VantageRuntime::with_epoch(EpochId(1));

        // Phase 1: Active — payloads accepted
        let p1 = make_payload(test_mutation("mut_phase1", "t_phase1"), EpochId(1), 1);
        assert!(runtime.commit_authorized(vec![p1]).is_ok());

        // Phase 2: Lock
        runtime.lock_epoch().expect("lock");

        // Phase 3: Active epoch payloads rejected
        let p2 = make_payload(test_mutation("mut_phase2", "t_phase2"), EpochId(1), 2);
        assert!(runtime.commit_authorized(vec![p2]).is_err());

        // Phase 4: Commit → transition to epoch 2
        let commit_result = runtime.commit_epoch(EpochId(2)).expect("commit");
        assert!(commit_result.success);

        // Phase 5: Epoch 2 active
        let p3 = make_payload(test_mutation("mut_phase3", "t_phase3"), EpochId(2), 1);
        assert!(runtime.commit_authorized(vec![p3]).is_ok());
    }
}
