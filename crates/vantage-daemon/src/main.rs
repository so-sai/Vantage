use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tower_http::cors::CorsLayer;
use vantage_core::{
    CommitReceipt, ElectionResult, EpochId, EpochProposal, ExecutionEnvelope, KnowledgeMutation,
    LogicalTime, NodeId,
};
use vantage_pek::{
    MutationRequest, PEKStats, ProofCertificate, ProofGate, ProofPolicy, PEKError,
    SystemProof, TransactionRequest,
};
use vantage_prn::{AttractorMonitor, EpochSnapshot, ElectionEngine, TrustDynamics};
use vantage_runtime::{ExecutionPayload, VantageRuntime};
use vantage_trust::{
    AuthorizedMutation, IdentityId, PolicyDigest, StaticTrustPolicy, TrustEvaluator,
};

struct MonitorState {
    trust_dynamics: TrustDynamics,
    monitor: AttractorMonitor,
    current_phase: PhaseSnapshot,
}

#[derive(Clone, Serialize)]
struct PhaseSnapshot {
    epoch: u64,
    state: String,
}

struct AppState {
    runtime: VantageRuntime,
    stats: PEKStats,
    trust: Box<dyn TrustEvaluator>,
    engine: ElectionEngine,
    monitor_state: Mutex<MonitorState>,
    sequence: AtomicU64,
    epoch: AtomicU64,
    http_client: reqwest::Client,
}

impl AppState {
    fn current_epoch(&self) -> EpochId {
        EpochId(self.epoch.load(Ordering::SeqCst))
    }
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum AttestationPayload {
    System { proof: SystemProof },
    Certificate { cert: ProofCertificate },
}

fn next_envelope(state: &AppState) -> ExecutionEnvelope {
    let seq = state.sequence.fetch_add(1, Ordering::SeqCst) + 1;
    ExecutionEnvelope::new(state.current_epoch(), seq, LogicalTime::new(seq))
}

fn try_advance_epoch(state: &AppState) -> Result<serde_json::Value, String> {
    let current = state.current_epoch();
    let next_epoch = EpochId(current.0 + 1);
    let proposal = EpochProposal {
        epoch: next_epoch,
        policy_snapshot: 1,
        min_sequence: state.sequence.load(Ordering::SeqCst),
        cutoff_time: LogicalTime::new(state.sequence.load(Ordering::SeqCst)),
        proposer: state.engine.node_id().clone(),
        trust_weight: 100,
    };

    match state.engine.run_election(vec![proposal], current) {
        ElectionResult::Candidate(quorum) => {
            let locked = state.runtime.lock_epoch()?;
            state.runtime.commit_epoch(next_epoch)?;
            state.epoch.store(next_epoch.0, Ordering::SeqCst);

            // Record epoch snapshot in monitor (read-only observability side-channel)
            if let Ok(mut ms) = state.monitor_state.lock() {
                let agreement = vantage_core::GlobalEpochAgreement {
                    epoch: next_epoch,
                    quorum: quorum.clone(),
                    supporting_nodes: quorum.supporters.clone(),
                    global_score: quorum.aggregate_score,
                };
                ms.trust_dynamics.update(Some(&agreement), &quorum.supporters);
                let trust_data: std::collections::HashMap<NodeId, f64> =
                    ms.trust_dynamics.trust_snapshot().into_iter().collect();
                ms.monitor.record(EpochSnapshot {
                    epoch: next_epoch.0,
                    trust_values: trust_data,
                    winning_nodes: quorum.supporters.clone(),
                });
                ms.current_phase = PhaseSnapshot {
                    epoch: next_epoch.0,
                    state: format!("{:?}", ms.monitor.classify()),
                };
            }

            Ok(json!({
                "message": format!(
                    "Epoch transition: {} → {} (quorum: {})",
                    locked.0, next_epoch.0, quorum.supporters.len()
                ),
                "epoch": next_epoch.0,
            }))
        }
        ElectionResult::NoConsensus => {
            Err("No epoch consensus reached".to_string())
        }
    }
}

impl AttestationPayload {
    fn commit_single(
        self,
        mutation: KnowledgeMutation,
        policy: ProofPolicy,
        runtime: &VantageRuntime,
        stats: &PEKStats,
        trust: &dyn TrustEvaluator,
        state: &AppState,
    ) -> Result<CommitReceipt, PEKError> {
        match self {
            AttestationPayload::System { proof } => {
                let req = MutationRequest::new(mutation, proof);
                ProofGate::commit(req, policy, runtime, stats)
            }
            AttestationPayload::Certificate { cert } => {
                let verified = cert.verify()?;
                let authorized = trust.authorize(verified).map_err(|e| PEKError::PolicyViolation(e))?;
                // Construct AuthorizedMutation (type-safe binding) then
                // convert to ExecutionPayload — runtime does NOT see governance internals
                let auth_mutation = AuthorizedMutation::new(mutation, authorized);
                let payload = ExecutionPayload::new(auth_mutation.mutation, next_envelope(state));
                runtime.commit_authorized(vec![payload])
                    .map(|mut v| v.pop().unwrap())
                    .map_err(|e| PEKError::RuntimeError(e))
            }
        }
    }

    fn commit_batch(
        self,
        mutations: Vec<KnowledgeMutation>,
        policy: ProofPolicy,
        runtime: &VantageRuntime,
        stats: &PEKStats,
        trust: &dyn TrustEvaluator,
        state: &AppState,
    ) -> Result<Vec<CommitReceipt>, PEKError> {
        match self {
            AttestationPayload::System { proof } => {
                let req = TransactionRequest::new(mutations, proof);
                ProofGate::commit_transaction(req, policy, runtime, stats)
            }
            AttestationPayload::Certificate { cert } => {
                let verified = cert.verify()?;
                let authorized = trust.authorize(verified).map_err(|e| PEKError::PolicyViolation(e))?;
                let payloads: Vec<ExecutionPayload> = mutations.into_iter()
                    .map(|m| {
                        let auth_mutation = AuthorizedMutation::new(m, authorized.clone());
                        ExecutionPayload::new(auth_mutation.mutation, next_envelope(state))
                    })
                    .collect();
                runtime.commit_authorized(payloads)
                    .map_err(|e| PEKError::RuntimeError(e))
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let runtime = VantageRuntime::new();
    let stats = PEKStats::new();
    let trust = Box::new(StaticTrustPolicy::new(
        IdentityId("vantage-daemon".into()),
        PolicyDigest("policy-v1".into()),
    ));

    let state = Arc::new(AppState {
        runtime,
        stats,
        trust,
        engine: ElectionEngine::new(NodeId("vantage-daemon".into()), 1),
        monitor_state: Mutex::new(MonitorState {
            trust_dynamics: TrustDynamics::new(1.0, 0.5, 0.3, 0.01),
            monitor: AttractorMonitor::new(5),
            current_phase: PhaseSnapshot { epoch: 1, state: "Forming".to_string() },
        }),
        sequence: AtomicU64::new(0),
        epoch: AtomicU64::new(1),
        http_client: reqwest::Client::new(),
    });

    let app = Router::new()
        .route("/v1/mutate", post(handle_mutate))
        .route("/v1/transaction", post(handle_transaction))
        .route("/v1/stats", get(handle_get_stats))
        .route("/v1/epoch/advance", post(handle_epoch_advance))
        .route("/v1/epoch/phase", get(handle_epoch_phase))
        .route("/v1/chat/completions", post(handle_chat_proxy))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = "127.0.0.1:8080";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("Vantage Knowledge Runtime Daemon running at http://{}", addr);

    axum::serve(listener, app).await.unwrap();
}

#[derive(Deserialize)]
struct MutatePayload {
    mutation: KnowledgeMutation,
    attestation: AttestationPayload,
    policy: Option<String>,
}

async fn handle_mutate(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<MutatePayload>,
) -> Response {
    let policy = parse_policy(payload.policy.as_deref());

    match payload.attestation.commit_single(payload.mutation, policy, &state.runtime, &state.stats, &*state.trust, &state) {
        Ok(receipt) => (StatusCode::OK, Json(receipt)).into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": format!("PEK-1 Rejected Mutation: {:?}", err) })),
        ).into_response(),
    }
}

#[derive(Deserialize)]
struct TransactionPayload {
    mutations: Vec<KnowledgeMutation>,
    attestation: AttestationPayload,
    policy: Option<String>,
}

async fn handle_transaction(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<TransactionPayload>,
) -> Response {
    let policy = parse_policy(payload.policy.as_deref());

    match payload.attestation.commit_batch(payload.mutations, policy, &state.runtime, &state.stats, &*state.trust, &state) {
        Ok(receipts) => (StatusCode::OK, Json(receipts)).into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": format!("PEK-1 Rejected Transaction: {:?}", err) })),
        ).into_response(),
    }
}

async fn handle_epoch_advance(
    State(state): State<Arc<AppState>>,
) -> Response {
    match try_advance_epoch(&state) {
        Ok(val) => (StatusCode::OK, Json(val)).into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": err })),
        ).into_response(),
    }
}

async fn handle_epoch_phase(
    State(state): State<Arc<AppState>>,
) -> Response {
    let phase = state.monitor_state.lock()
        .map(|ms| ms.current_phase.clone())
        .unwrap_or(PhaseSnapshot { epoch: 0, state: "Unknown".to_string() });
    (StatusCode::OK, Json(json!(phase))).into_response()
}

#[derive(Serialize)]
struct StatsResponse {
    admitted: u64,
    rejected: u64,
    advisory_warnings: u64,
}

async fn handle_get_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(StatsResponse {
        admitted: state.stats.admitted_count.load(Ordering::SeqCst),
        rejected: state.stats.rejected_count.load(Ordering::SeqCst),
        advisory_warnings: state.stats.advisory_warnings.load(Ordering::SeqCst),
    })
}

#[derive(Deserialize, Serialize, Clone)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<serde_json::Value>,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

async fn handle_chat_proxy(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<ChatCompletionRequest>,
) -> Response {
    let upstream_url = if headers.contains_key("Authorization") {
        "https://api.deepseek.com/v1/chat/completions"
    } else {
        "http://localhost:11434/v1/chat/completions"
    };

    let mut req_builder = state.http_client.post(upstream_url).json(&payload);

    if let Some(auth) = headers.get("Authorization") {
        req_builder = req_builder.header("Authorization", auth.clone());
    }

    match req_builder.send().await {
        Ok(upstream_response) => {
            let status = StatusCode::from_u16(upstream_response.status().as_u16())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            let body_bytes = upstream_response.bytes().await.unwrap_or_default();
            (status, body_bytes).into_response()
        }
        Err(err) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({
                "error": format!("Vantage Daemon: Upstream connection error: {}", err)
            })),
        ).into_response(),
    }
}

fn parse_policy(policy_str: Option<&str>) -> ProofPolicy {
    match policy_str {
        // Disabled and Advisory are blocked from external HTTP clients.
        // Only Enforced and StrictCanonical are allowed over the network.
        Some("StrictCanonical") => ProofPolicy::StrictCanonical,
        _ => ProofPolicy::Enforced,
    }
}
