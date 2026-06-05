use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use vantage_core::{CommitReceipt, KnowledgeMutation};
use vantage_pek::{
    MutationRequest, PEKStats, ProofCertificate, ProofGate, ProofPolicy, PEKError,
    SystemProof, TransactionRequest, VerifiedCertificate,
};
use vantage_runtime::VantageRuntime;

struct AppState {
    runtime: VantageRuntime,
    stats: PEKStats,
    http_client: reqwest::Client,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum AttestationPayload {
    System { proof: SystemProof },
    Certificate { cert: ProofCertificate },
}

impl AttestationPayload {
    fn commit_single(
        self,
        mutation: KnowledgeMutation,
        policy: ProofPolicy,
        runtime: &VantageRuntime,
        stats: &PEKStats,
    ) -> Result<CommitReceipt, PEKError> {
        match self {
            AttestationPayload::System { proof } => {
                let req = MutationRequest::new(mutation, proof);
                ProofGate::commit(req, policy, runtime, stats)
            }
            AttestationPayload::Certificate { cert } => {
                let verified: VerifiedCertificate = cert.verify()?;
                let req = MutationRequest::new(mutation, verified);
                ProofGate::commit(req, policy, runtime, stats)
            }
        }
    }

    fn commit_batch(
        self,
        mutations: Vec<KnowledgeMutation>,
        policy: ProofPolicy,
        runtime: &VantageRuntime,
        stats: &PEKStats,
    ) -> Result<Vec<CommitReceipt>, PEKError> {
        match self {
            AttestationPayload::System { proof } => {
                let req = TransactionRequest::new(mutations, proof);
                ProofGate::commit_transaction(req, policy, runtime, stats)
            }
            AttestationPayload::Certificate { cert } => {
                let verified: VerifiedCertificate = cert.verify()?;
                let req = TransactionRequest::new(mutations, verified);
                ProofGate::commit_transaction(req, policy, runtime, stats)
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let runtime = VantageRuntime::new();
    let stats = PEKStats::new();

    let state = Arc::new(AppState {
        runtime,
        stats,
        http_client: reqwest::Client::new(),
    });

    let app = Router::new()
        .route("/v1/mutate", post(handle_mutate))
        .route("/v1/transaction", post(handle_transaction))
        .route("/v1/stats", get(handle_get_stats))
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

    match payload.attestation.commit_single(payload.mutation, policy, &state.runtime, &state.stats) {
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

    match payload.attestation.commit_batch(payload.mutations, policy, &state.runtime, &state.stats) {
        Ok(receipts) => (StatusCode::OK, Json(receipts)).into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": format!("PEK-1 Rejected Transaction: {:?}", err) })),
        ).into_response(),
    }
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
