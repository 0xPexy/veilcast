mod doc;
mod error;
mod indexer;
mod repo;
mod zk;

use crate::doc::ApiDoc;
use crate::error::{AppError, AppResult};
use crate::indexer::{spawn_indexer, IndexerConfig};
use crate::repo::{
    InMemoryStore, NewPoll, PgStore, PollRecord, PollStore, StoredCommit, StoredVote,
};
use crate::zk::{NoopZkBackend, ProofBundle, ProofRequest, ZkBackend};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use ethers::core::types::H160;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(Clone)]
struct AppState<S, B> {
    store: Arc<S>,
    zk: Arc<B>,
}

impl<S, B> AppState<S, B> {
    fn new(store: Arc<S>, zk: Arc<B>) -> Self {
        Self { store, zk }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    Commit,
    Reveal,
    Resolved,
}

impl Phase {
    fn from_times(
        now: DateTime<Utc>,
        commit_end: DateTime<Utc>,
        reveal_end: DateTime<Utc>,
        resolved: bool,
    ) -> Self {
        if resolved || now >= reveal_end {
            Phase::Resolved
        } else if now >= commit_end {
            Phase::Reveal
        } else {
            Phase::Commit
        }
    }
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
struct CreatePollRequest {
    question: String,
    options: Vec<String>,
    commit_phase_end: DateTime<Utc>,
    reveal_phase_end: DateTime<Utc>,
    membership_root: String,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
struct PollResponse {
    id: i64,
    question: String,
    options: Vec<String>,
    commit_phase_end: DateTime<Utc>,
    reveal_phase_end: DateTime<Utc>,
    membership_root: String,
    correct_option: Option<i16>,
    resolved: bool,
    phase: Phase,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
struct CommitRequest {
    commitment: String,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
struct CommitResponse {
    poll_id: i64,
    commitment: String,
    recorded_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
struct ProveRequest {
    choice: u8,
    secret: String,
    identity_secret: String,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
struct RevealRequest {
    proof: String,
    public_inputs: Vec<String>,
    commitment: String,
    nullifier: String,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
struct RevealResponse {
    poll_id: i64,
    nullifier: String,
    recorded_at: DateTime<Utc>,
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let cfg = Config::from_env();
    let pool = PgStore::connect(&cfg.database_url).await?;
    let store = Arc::new(pool);
    let zk = Arc::new(NoopZkBackend::default());

    let app_state = AppState::new(store, zk);
    let app = app_router(app_state.clone())
        .merge(SwaggerUi::new("/docs").url("/docs/openapi.json", ApiDoc::openapi()));

    let addr: SocketAddr = cfg.bind.parse().expect("invalid bind addr");
    info!("Starting VeilCast backend on {}", addr);
    let server = axum::serve(
        tokio::net::TcpListener::bind(addr).await?,
        app.into_make_service(),
    );

    if let (Some(rpc_ws), Some(contract)) = (cfg.rpc_ws.clone(), cfg.contract_address) {
        let idx_cfg = IndexerConfig {
            rpc_ws,
            contract_address: contract,
            from_block: cfg.indexer_from_block,
        };
        let _indexer = spawn_indexer(idx_cfg, app_state.store.clone());
        info!("Indexer spawned");
    } else {
        info!("Indexer not started (missing RPC_WS or CONTRACT_ADDRESS)");
    }

    server.await?;
    Ok(())
}

fn app_router<S, B>(state: AppState<S, B>) -> Router
where
    S: PollStore + Clone + Send + Sync + 'static,
    B: ZkBackend + Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/health", get(health))
        .route("/polls", post(create_poll::<S, B>).get(list_polls::<S, B>))
        .route("/polls/:id", get(get_poll::<S, B>))
        .route("/polls/:id/commit", post(record_commit::<S, B>))
        .route("/polls/:id/prove", post(generate_proof::<S, B>))
        .route("/polls/:id/reveal", post(reveal_vote::<S, B>))
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    StatusCode::OK
}

async fn create_poll<S, B>(
    State(state): State<AppState<S, B>>,
    Json(body): Json<CreatePollRequest>,
) -> Result<Json<PollResponse>, AppError>
where
    S: PollStore + Send + Sync,
{
    if body.options.len() < 2 {
        return Err(AppError::Validation("options must be >= 2".into()));
    }
    if body.commit_phase_end >= body.reveal_phase_end {
        return Err(AppError::Validation(
            "commit end must be before reveal end".into(),
        ));
    }
    let record = state
        .store
        .create_poll(NewPoll {
            question: &body.question,
            options: &body.options,
            commit_phase_end: body.commit_phase_end,
            reveal_phase_end: body.reveal_phase_end,
            membership_root: &body.membership_root,
        })
        .await?;

    Ok(Json(to_response(record)))
}

async fn get_poll<S, B>(
    State(state): State<AppState<S, B>>,
    Path(poll_id): Path<i64>,
) -> Result<Json<PollResponse>, AppError>
where
    S: PollStore + Send + Sync,
{
    let record = state.store.get_poll(poll_id).await?;
    Ok(Json(to_response(record)))
}

async fn list_polls<S, B>(
    State(state): State<AppState<S, B>>,
) -> Result<Json<Vec<PollResponse>>, AppError>
where
    S: PollStore + Send + Sync,
{
    let records = state.store.list_polls(50).await?;
    Ok(Json(records.into_iter().map(to_response).collect()))
}

async fn record_commit<S, B>(
    State(state): State<AppState<S, B>>,
    Path(poll_id): Path<i64>,
    Json(body): Json<CommitRequest>,
) -> Result<Json<CommitResponse>, AppError>
where
    S: PollStore + Send + Sync,
{
    let poll = state.store.get_poll(poll_id).await?;
    if Utc::now() >= poll.commit_phase_end {
        return Err(AppError::Validation("commit phase over".into()));
    }
    let stored = state
        .store
        .record_commit(StoredCommit {
            poll_id,
            commitment: &body.commitment,
        })
        .await?;
    Ok(Json(CommitResponse {
        poll_id: stored.poll_id,
        commitment: stored.commitment,
        recorded_at: stored.recorded_at,
    }))
}

async fn generate_proof<S, B>(
    State(state): State<AppState<S, B>>,
    Path(poll_id): Path<i64>,
    Json(body): Json<ProveRequest>,
) -> Result<Json<ProofBundle>, AppError>
where
    S: PollStore + Send + Sync,
    B: ZkBackend + Send + Sync,
{
    let poll = state.store.get_poll(poll_id).await?;
    if Utc::now() >= poll.reveal_phase_end {
        return Err(AppError::Validation("poll already resolved".into()));
    }
    let req = ProofRequest {
        poll_id,
        choice: body.choice,
        secret: &body.secret,
        identity_secret: &body.identity_secret,
        membership_root: &poll.membership_root,
    };
    let bundle = state.zk.prove(req).await?;
    Ok(Json(bundle))
}

async fn reveal_vote<S, B>(
    State(state): State<AppState<S, B>>,
    Path(poll_id): Path<i64>,
    Json(body): Json<RevealRequest>,
) -> Result<Json<RevealResponse>, AppError>
where
    S: PollStore + Send + Sync,
    B: ZkBackend + Send + Sync,
{
    let poll = state.store.get_poll(poll_id).await?;
    let now = Utc::now();
    if now < poll.commit_phase_end || now >= poll.reveal_phase_end {
        return Err(AppError::Validation("not in reveal window".into()));
    }
    let bundle = ProofBundle {
        proof: body.proof,
        public_inputs: body.public_inputs,
        commitment: body.commitment,
        nullifier: body.nullifier,
    };
    state.zk.verify(&poll, &bundle).await?;
    let vote = state
        .store
        .record_vote(StoredVote {
            poll_id,
            nullifier: &bundle.nullifier,
            choice: extract_choice(&bundle)?,
        })
        .await?;
    Ok(Json(RevealResponse {
        poll_id: vote.poll_id,
        nullifier: vote.nullifier,
        recorded_at: vote.recorded_at,
    }))
}

fn extract_choice(bundle: &ProofBundle) -> AppResult<u8> {
    // public_inputs format is backend-defined; for the noop backend we encode choice in first element.
    if let Some(first) = bundle.public_inputs.first() {
        first
            .parse::<u8>()
            .map_err(|_| AppError::Validation("invalid choice in public_inputs[0]".into()))
    } else {
        Err(AppError::Validation("missing public_inputs".into()))
    }
}

fn to_response(record: PollRecord) -> PollResponse {
    let phase = Phase::from_times(
        Utc::now(),
        record.commit_phase_end,
        record.reveal_phase_end,
        record.resolved,
    );
    PollResponse {
        id: record.id,
        question: record.question,
        options: record.options,
        commit_phase_end: record.commit_phase_end,
        reveal_phase_end: record.reveal_phase_end,
        membership_root: record.membership_root,
        correct_option: record.correct_option,
        resolved: record.resolved,
        phase,
    }
}

#[derive(Clone, Debug)]
struct Config {
    database_url: String,
    bind: String,
    rpc_ws: Option<String>,
    contract_address: Option<H160>,
    indexer_from_block: Option<u64>,
}

impl Config {
    fn from_env() -> Self {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://veilcast:veilcast@localhost:5432/veilcast".to_string());
        let bind = std::env::var("BIND").unwrap_or_else(|_| "0.0.0.0:8000".to_string());
        let rpc_ws = std::env::var("RPC_WS").ok();
        let contract_address = std::env::var("CONTRACT_ADDRESS")
            .ok()
            .and_then(|s| H160::from_str(&s).ok());
        let indexer_from_block = std::env::var("INDEXER_FROM_BLOCK")
            .ok()
            .and_then(|s| s.parse().ok());
        Self {
            database_url,
            bind,
            rpc_ws,
            contract_address,
            indexer_from_block,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    fn test_app() -> Router {
        let store = Arc::new(InMemoryStore::default());
        let zk = Arc::new(NoopZkBackend::default());
        let state = AppState::new(store, zk);
        app_router(state)
    }

    #[tokio::test]
    async fn create_and_get_poll() {
        let app = test_app();
        let body = serde_json::json!({
            "question": "Will it rain?",
            "options": ["Yes", "No"],
            "commit_phase_end": Utc::now(),
            "reveal_phase_end": Utc::now() + chrono::Duration::minutes(30),
            "membership_root": "123"
        });

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/polls")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let res = app
            .oneshot(
                Request::builder()
                    .uri("/polls/0")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn commit_and_reveal_flow() {
        let app = test_app();
        let commit_end = Utc::now() + chrono::Duration::milliseconds(50);
        let reveal_end = commit_end + chrono::Duration::minutes(5);
        let create_body = serde_json::json!({
            "question": "Q",
            "options": ["A", "B"],
            "commit_phase_end": commit_end,
            "reveal_phase_end": reveal_end,
            "membership_root": "root"
        });
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/polls")
                    .header("content-type", "application/json")
                    .body(Body::from(create_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let commit_body = serde_json::json!({ "commitment": "c1" });
        let commit_res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/polls/0/commit")
                    .header("content-type", "application/json")
                    .body(Body::from(commit_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(commit_res.status(), StatusCode::OK);

        // Move into reveal window
        tokio::time::sleep(std::time::Duration::from_millis(60)).await;

        let prove_body = serde_json::json!({
            "choice": 1,
            "secret": "42",
            "identity_secret": "99"
        });
        let prove_res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/polls/0/prove")
                    .header("content-type", "application/json")
                    .body(Body::from(prove_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(prove_res.status(), StatusCode::OK);

        let body_bytes = to_bytes(prove_res.into_body(), usize::MAX).await.unwrap();
        let proof: ProofBundle = serde_json::from_slice(&body_bytes).unwrap();
        let reveal_body = serde_json::json!({
            "proof": proof.proof,
            "public_inputs": proof.public_inputs,
            "commitment": proof.commitment,
            "nullifier": proof.nullifier
        });
        let reveal_res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/polls/0/reveal")
                    .header("content-type", "application/json")
                    .body(Body::from(reveal_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(reveal_res.status(), StatusCode::OK);
    }
}
