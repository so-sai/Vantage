use std::collections::HashMap;
use std::sync::Mutex;
use std::time::SystemTime;
use tracing::warn;
use vantage_core::{
    CommitReceipt, EpistemicInvariant, EpistemicReader, InvariantContext,
    InvariantViolation, KnowledgeMutation, MutationOp, ResourceId
};
use vantage_pek::EpistemicExecutor;
use vantage_trust::AuthorizedMutation;
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

pub struct VantageRuntime {
    index: Mutex<TemporalIndex>,
    invariants: Vec<Box<dyn EpistemicInvariant>>,
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
        }
    }

    /// TIA-1: Commit authorized mutations.
    /// Only accepts AuthorizedMutation (verified + authorized).
    /// This is the ONLY public mutation API in TIA-1.
    pub fn commit_authorized(&self, mutations: Vec<AuthorizedMutation>) -> Result<Vec<CommitReceipt>, String> {
        let raw: Vec<KnowledgeMutation> = mutations.into_iter().map(|am| am.mutation).collect();
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
    use vantage_core::{AgentId, MutationId, MutationOp, ResourceId};
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
}
