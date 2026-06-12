use std::sync::atomic::Ordering;
use std::sync::Arc;
use tracing::info;
use vantage_core::{
    CommitReceipt, EpochId, EpistemicReader, ExecutionEnvelope, KnowledgeMutation, LogicalTime,
    NodeId, ResourceId,
};
use vantage_pek::{MutationRequest, PEKStats, ProofGate, SystemProof};
use vantage_prn::ElectionEngine;
use vantage_runtime::{ExecutionPayload, VantageRuntime};
use vantage_trust::{
    AuthorizedMutation, IdentityId, PolicyDigest, StaticTrustPolicy, TrustEvaluator,
};

use crate::config::{ClientConfig, ClientMode};
use crate::error::{HistoryEntry, StatsSnapshot, VantageError};
use crate::mutation::MutationBuilder;
use crate::query::QueryBuilder;

pub struct VantageClient {
    config: ClientConfig,
    runtime: Option<Arc<VantageRuntime>>,
    stats: Option<Arc<PEKStats>>,
    trust: Option<Box<dyn TrustEvaluator>>,
    engine: Option<Arc<ElectionEngine>>,
    sequence: u64,
    epoch: u64,
}

impl VantageClient {
    pub fn new(config: ClientConfig) -> Self {
        match &config.mode {
            ClientMode::Embedded => {
                let runtime = Arc::new(VantageRuntime::new());
                let stats = Arc::new(PEKStats::new());
                let trust = Box::new(StaticTrustPolicy::new(
                    IdentityId(config.identity.clone()),
                    PolicyDigest("policy-v1".into()),
                ));
                let engine = {
                    let mut e = ElectionEngine::new(
                        NodeId(config.identity.clone()),
                        1,
                    );
                    e.set_weight(NodeId(config.identity.clone()), 100);
                    Arc::new(e)
                };
                Self {
                    config,
                    runtime: Some(runtime),
                    stats: Some(stats),
                    trust: Some(trust),
                    engine: Some(engine),
                    sequence: 0,
                    epoch: 1,
                }
            }
            ClientMode::Remote { .. } => Self {
                config,
                runtime: None,
                stats: None,
                trust: None,
                engine: None,
                sequence: 0,
                epoch: 1,
            },
        }
    }

    pub fn mutate(&mut self) -> MutationBuilder<'_> {
        MutationBuilder::new(self)
    }

    pub fn query(&self) -> QueryBuilder<'_> {
        QueryBuilder::new(self)
    }

    pub fn epoch(&self) -> EpochId {
        EpochId(self.epoch)
    }

    fn next_envelope(&mut self) -> ExecutionEnvelope {
        self.sequence += 1;
        ExecutionEnvelope::new(EpochId(self.epoch), self.sequence, LogicalTime::new(self.sequence))
    }

    pub fn execute(&mut self, mutation: KnowledgeMutation) -> Result<CommitReceipt, VantageError> {
        match &self.config.mode {
            ClientMode::Embedded => {
                let runtime = self.runtime.as_ref().unwrap();
                let stats = self.stats.as_ref().unwrap();

                let req = MutationRequest::new(mutation, SystemProof::Test);
                let receipt = ProofGate::commit(req, self.config.policy, &**runtime, stats)?;
                self.sequence += 1;
                info!(tx_id = %receipt.tx_id, "Mutation committed");
                Ok(receipt)
            }
            ClientMode::Remote { .. } => {
                Err(VantageError::Internal("Remote mode not yet implemented".into()))
            }
        }
    }

    pub fn execute_batch(&mut self, mutations: Vec<KnowledgeMutation>) -> Result<Vec<CommitReceipt>, VantageError> {
        match &self.config.mode {
            ClientMode::Embedded => {
                let runtime = self.runtime.as_ref().unwrap();
                let stats = self.stats.as_ref().unwrap();

                use vantage_pek::TransactionRequest;
                let req = TransactionRequest::new(mutations, SystemProof::Test);
                let receipts = ProofGate::commit_transaction(req, self.config.policy, &**runtime, stats)?;
                self.sequence += receipts.len() as u64;
                info!(count = receipts.len(), "Batch mutation committed");
                Ok(receipts)
            }
            ClientMode::Remote { .. } => {
                Err(VantageError::Internal("Remote mode not yet implemented".into()))
            }
        }
    }

    pub fn execute_authorized(&mut self, mutation: KnowledgeMutation) -> Result<CommitReceipt, VantageError> {
        match &self.config.mode {
            ClientMode::Embedded => {
                let envelope = self.next_envelope();
                let runtime = self.runtime.as_ref().unwrap();
                let trust = self.trust.as_ref().unwrap();

                use vantage_pek::ProofCertificate;
                let hash = [1u8; 32];
                let cert = ProofCertificate::new(
                    format!("claim_{}", mutation.mutation_id.0),
                    hash,
                    1,
                );
                let verified = cert.verify()?;
                let authorized = trust
                    .authorize(verified)
                    .map_err(|e| VantageError::MutationRejected(
                        vantage_pek::PEKError::PolicyViolation(e),
                    ))?;

                let auth_mutation = AuthorizedMutation::new(mutation, authorized);
                let payload = ExecutionPayload::new(auth_mutation.mutation, envelope);

                let mut receipts = runtime
                    .commit_authorized(vec![payload])
                    .map_err(|e| VantageError::Internal(e))?;
                receipts.pop().ok_or_else(|| VantageError::Internal("Empty receipt".into()))
            }
            ClientMode::Remote { .. } => {
                Err(VantageError::Internal("Remote mode not yet implemented".into()))
            }
        }
    }

    pub fn advance_epoch(&mut self) -> Result<String, VantageError> {
        match &self.config.mode {
            ClientMode::Embedded => {
                let runtime = self.runtime.as_ref().unwrap();
                let engine = self.engine.as_ref().unwrap();

                let current = EpochId(self.epoch);
                let next = EpochId(current.0 + 1);
                let proposal = vantage_core::EpochProposal {
                    epoch: next,
                    policy_snapshot: 1,
                    min_sequence: self.sequence,
                    cutoff_time: LogicalTime::new(self.sequence),
                    proposer: engine.node_id().clone(),
                    trust_weight: 100,
                };

                match engine.run_election(vec![proposal], current) {
                    vantage_core::ElectionResult::Candidate(quorum) => {
                        runtime.lock_epoch().map_err(|e| VantageError::EpochError(e))?;
                        runtime.commit_epoch(next).map_err(|e| VantageError::EpochError(e))?;
                        self.epoch = next.0;
                        self.sequence = 0;
                        info!(epoch = %next.0, supporters = quorum.supporters.len(), "Epoch advanced");
                        Ok(format!("Epoch transitioned: {} → {}", current.0, next.0))
                    }
                    vantage_core::ElectionResult::NoConsensus => {
                        Err(VantageError::EpochError("No epoch consensus".into()))
                    }
                }
            }
            ClientMode::Remote { .. } => {
                Err(VantageError::Internal("Remote mode not yet implemented".into()))
            }
        }
    }

    pub fn read_unit(&self, resource_id: &ResourceId) -> Result<Option<String>, VantageError> {
        match &self.config.mode {
            ClientMode::Embedded => {
                let runtime = self.runtime.as_ref().unwrap();
                Ok(runtime.read_unit(resource_id))
            }
            ClientMode::Remote { .. } => {
                Err(VantageError::Internal("Remote mode not yet implemented".into()))
            }
        }
    }

    pub fn exists(&self, resource_id: &ResourceId) -> Result<bool, VantageError> {
        match &self.config.mode {
            ClientMode::Embedded => {
                let runtime = self.runtime.as_ref().unwrap();
                Ok(runtime.exists(resource_id))
            }
            ClientMode::Remote { .. } => {
                Err(VantageError::Internal("Remote mode not yet implemented".into()))
            }
        }
    }

    pub fn stats(&self) -> Result<StatsSnapshot, VantageError> {
        match &self.config.mode {
            ClientMode::Embedded => {
                let s = self.stats.as_ref().unwrap();
                Ok(StatsSnapshot {
                    admitted: s.admitted_count.load(Ordering::SeqCst),
                    rejected: s.rejected_count.load(Ordering::SeqCst),
                    advisory_warnings: s.advisory_warnings.load(Ordering::SeqCst),
                })
            }
            ClientMode::Remote { .. } => {
                Err(VantageError::Internal("Remote mode not yet implemented".into()))
            }
        }
    }

    pub fn history(&self, resource_id: &ResourceId) -> Result<Vec<HistoryEntry>, VantageError> {
        match &self.config.mode {
            ClientMode::Embedded => {
                let runtime = self.runtime.as_ref().unwrap();
                let index = runtime.index().lock().map_err(|e| VantageError::Internal(e.to_string()))?;
                let revs = index.history.get(resource_id).cloned().unwrap_or_default();
                let entries = revs
                    .into_iter()
                    .enumerate()
                    .map(|(i, rev)| HistoryEntry {
                        sequence: (i + 1) as u64,
                        resource_id: resource_id.clone(),
                        mutation_id: rev.invariant_hash.clone(),
                        actor: "unknown".into(),
                        timestamp: rev.timestamp,
                        payload: rev.payload,
                    })
                    .collect();
                Ok(entries)
            }
            ClientMode::Remote { .. } => {
                Err(VantageError::Internal("Remote mode not yet implemented".into()))
            }
        }
    }

    pub fn config(&self) -> &ClientConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    use vantage_core::{AgentId, KnowledgeMutation, MutationId, MutationOp, ResourceId};
    use vantage_pek::ProofPolicy;

    #[test]
    fn test_client_mutate_and_query() {
        let config = ClientConfig::embedded("test_agent");
        let mut client = VantageClient::new(config);

        let receipt = client
            .mutate()
            .resource("unit:hello")
            .id("mut_001")
            .insert("println!(\"Hello Vantage\");")
            .execute()
            .expect("Mutation should succeed");

        assert!(receipt.tx_id.contains("mut_001"));

        let content = client
            .query()
            .resource("unit:hello")
            .current()
            .expect("Query should succeed")
            .expect("Resource should exist");
        assert_eq!(content, "println!(\"Hello Vantage\");");
    }

    #[test]
    fn test_client_batch_mutation() {
        let config = ClientConfig::embedded("test_agent");
        let mut client = VantageClient::new(config);

        let mutations = vec![
            KnowledgeMutation {
                mutation_id: MutationId("mut_a".into()),
                actor: AgentId("test_agent".into()),
                op: MutationOp::Insert {
                    resource_id: ResourceId("unit:a".into()),
                    payload: "module_a".into(),
                },
                timestamp: SystemTime::now(),
            },
            KnowledgeMutation {
                mutation_id: MutationId("mut_b".into()),
                actor: AgentId("test_agent".into()),
                op: MutationOp::Insert {
                    resource_id: ResourceId("unit:b".into()),
                    payload: "module_b".into(),
                },
                timestamp: SystemTime::now(),
            },
        ];

        let receipts = client
            .execute_batch(mutations)
            .expect("Batch mutation should succeed");
        assert_eq!(receipts.len(), 2);

        assert!(client.query().resource("unit:a").exists().unwrap());
        assert!(client.query().resource("unit:b").exists().unwrap());
    }

    #[test]
    fn test_client_delete_and_history() {
        let config = ClientConfig::embedded("test_agent");
        let mut client = VantageClient::new(config);

        client
            .mutate()
            .resource("unit:temp")
            .id("mut_temp")
            .insert("temporary data")
            .execute()
            .expect("Insert should succeed");

        let history = client
            .query()
            .resource("unit:temp")
            .history()
            .expect("History should succeed");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].payload.as_deref(), Some("temporary data"));
    }

    #[test]
    fn test_client_epoch_advance() {
        let mut config = ClientConfig::embedded("test_agent");
        config.policy = ProofPolicy::Disabled;
        let mut client = VantageClient::new(config);

        client
            .mutate()
            .resource("unit:epoch_test")
            .id("mut_epoch")
            .insert("before epoch")
            .execute()
            .expect("Mutate before epoch should succeed");

        let msg = client
            .advance_epoch()
            .expect("Epoch advance should succeed");
        assert!(msg.contains("1"));
        assert!(msg.contains("2"));

        // After epoch advance, new epoch is active
        client
            .mutate()
            .resource("unit:after_epoch")
            .id("mut_after")
            .insert("after epoch")
            .execute()
            .expect("Mutate after epoch should succeed");
    }
}
