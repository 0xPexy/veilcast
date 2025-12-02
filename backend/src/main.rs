mod doc;
mod error;
mod indexer;
mod repo;
mod types;
mod zk;

use crate::doc::ApiDoc;
use crate::error::{AppError, AppResult};
use crate::indexer::{spawn_indexer, IndexerConfig};
#[cfg(test)]
use crate::repo::InMemoryStore;
use crate::repo::{NewPoll, PgStore, PollRecord, PollStore, StoredCommit, StoredVote};
use crate::types::{
    CommitRequest, CommitResponse, CommitStatusResponse, CreatePollRequest, LoginRequest, LoginResponse, MeResponse,
    MembershipStatusResponse, Phase, PollResponse, ProveRequest, RevealRequest, RevealResponse,
};
use crate::zk::{NoopZkBackend, ProofBundle, ProofRequest, ZkBackend};
use async_trait::async_trait;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use ethers::contract::abigen;
use ethers::core::types::{H160, H256, U256};
use ethers::middleware::SignerMiddleware;
use ethers::providers::{Middleware, Provider, Ws};
use ethers::signers::{LocalWallet, Signer};
use hex;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::{error, info, warn};
use tokio::time::Duration;
use sha2::{Digest, Sha256};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use once_cell::sync::OnceCell;

static IDENTITY_SALT: OnceCell<String> = OnceCell::new();

abigen!(
    VeilCastCommitContract,
    r#"[
        function commit(uint256 pollId, bytes32 commitment)
    ]"#
);

#[async_trait]
pub trait OnchainCommitter: Send + Sync {
    async fn submit_commit(&self, poll_id: i64, commitment: &str) -> AppResult<()>;
}

#[derive(Clone, Default)]
pub struct NoopCommitter;

#[async_trait]
impl OnchainCommitter for NoopCommitter {
    async fn submit_commit(&self, poll_id: i64, commitment: &str) -> AppResult<()> {
        info!(
            poll_id,
            commitment,
            "Simulating on-chain submission of commitment"
        );
        Ok(())
    }
}

#[derive(Clone)]
pub struct EthersCommitter {
    contract: VeilCastCommitContract<SignerMiddleware<Provider<Ws>, LocalWallet>>,
}

impl EthersCommitter {
    pub async fn new(ws_url: &str, private_key: &str, contract_address: H160) -> AppResult<Self> {
        let ws = Ws::connect(ws_url)
            .await
            .map_err(|e| AppError::External(format!("ws connect error: {e}")))?;
        let provider = Provider::new(ws);

        let chain_id = provider
            .get_chainid()
            .await
            .map_err(|e| AppError::External(format!("chain id error: {e}")))?;

        let wallet = private_key
            .parse::<LocalWallet>()
            .map_err(|e| AppError::External(format!("invalid relayer key: {e}")))?
            .with_chain_id(chain_id.as_u64());

        let client = SignerMiddleware::new(provider, wallet);
        let client = Arc::new(client);
        let contract = VeilCastCommitContract::new(contract_address, client);
        Ok(Self { contract })
    }
}

fn parse_commitment_hex(value: &str) -> AppResult<H256> {
    let hex_str = value.strip_prefix("0x").unwrap_or(value);
    let bytes = hex::decode(hex_str)
        .map_err(|e| AppError::Validation(format!("invalid commitment hex: {e}")))?;
    if bytes.len() > 32 {
        return Err(AppError::Validation("commitment too long".into()));
    }
    let mut buf = [0u8; 32];
    buf[32 - bytes.len()..].copy_from_slice(&bytes);
    Ok(H256::from(buf))
}

#[async_trait]
impl OnchainCommitter for EthersCommitter {
    async fn submit_commit(&self, poll_id: i64, commitment: &str) -> AppResult<()> {
        let commitment_h256 = parse_commitment_hex(commitment)?;
        let poll_u256 = if poll_id < 0 {
            return Err(AppError::Validation("invalid poll id".into()));
        } else {
            U256::from(poll_id as u64)
        };
        let call = self.contract.clone().commit(poll_u256, commitment_h256.into());
        let pending = call
            .send()
            .await
            .map_err(|e| AppError::External(format!("send commit tx failed: {e}")))?;
        pending
            .await
            .map_err(|e| AppError::External(format!("commit tx pending failed: {e}")))?;
        Ok(())
    }
}

async fn sync_commits_once<S>(
    store: Arc<S>,
    committer: Arc<dyn OnchainCommitter + Send + Sync>,
) -> AppResult<()>
where
    S: PollStore + Send + Sync + 'static,
{
    let pending = store.commits_to_sync(Utc::now(), 50).await?;
    info!(pending = pending.len(), "commit sync tick");
    for item in pending {
        if let Err(err) = committer.submit_commit(item.poll_id, &item.commitment).await {
            error!(poll_id = item.poll_id, ?err, "Failed to submit commitment");
            continue;
        }
        store.mark_commit_synced(item.id).await?;
        if !store.poll_has_pending_commits(item.poll_id).await? {
            store.mark_poll_sync_complete(item.poll_id).await?;
        }
    }
    store.mark_polls_without_pending_commits(Utc::now()).await?;
    Ok(())
}

fn spawn_commit_sync<S>(
    store: Arc<S>,
    committer: Arc<dyn OnchainCommitter + Send + Sync>,
    interval: Duration,
) where
    S: PollStore + Send + Sync + 'static,
{
    let store_clone = store.clone();
    let committer_clone = committer.clone();
    tokio::spawn(async move {
        if let Err(err) = sync_commits_once(store_clone, committer_clone).await {
            warn!(?err, "initial commit sync failed");
        }
    });
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            info!("running commit sync job");
            if let Err(err) = sync_commits_once(store.clone(), committer.clone()).await {
                warn!(?err, "commit sync job failed");
            }
        }
    });
}

#[derive(Clone)]
struct AppState<S, B> {
    store: Arc<S>,
    zk: Arc<B>,
    identity_salt: String,
}

impl<S, B> AppState<S, B> {
    fn new(store: Arc<S>, zk: Arc<B>, identity_salt: String) -> Self {
        Self {
            store,
            zk,
            identity_salt,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    dotenvy::dotenv().ok();
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .init();

    let cfg = Config::from_env();
    let _ = IDENTITY_SALT.set(cfg.identity_salt.clone());
    let pool = PgStore::connect(&cfg.database_url).await?;
    let store = Arc::new(pool);
    let zk = Arc::new(NoopZkBackend::default());

    let app_state = AppState::new(store, zk, cfg.identity_salt.clone());
    let rpc_ws_for_committer = cfg.rpc_ws.clone();
    let committer: Arc<dyn OnchainCommitter> =
        if let (Some(ref pk), Some(addr), Some(ref ws)) =
            (&cfg.relayer_private_key, cfg.contract_address, rpc_ws_for_committer.as_ref())
        {
            match EthersCommitter::new(ws, pk, addr).await {
                Ok(c) => {
                    info!("On-chain commit sync enabled");
                    Arc::new(c)
                }
                Err(err) => {
                    warn!(?err, "Failed to init on-chain committer, falling back to noop");
                    Arc::new(NoopCommitter::default())
                }
            }
        } else {
            warn!("RELAYER_PRIVATE_KEY or CONTRACT_ADDRESS missing, commit sync noop");
            Arc::new(NoopCommitter::default())
        };
    spawn_commit_sync(
        app_state.store.clone(),
        committer,
        Duration::from_millis(cfg.commit_sync_interval_ms),
    );
    let cors = CorsLayer::very_permissive();
    let app = app_router(app_state.clone())
        .merge(SwaggerUi::new("/docs").url("/docs/openapi.json", ApiDoc::openapi()))
        .layer(cors);

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
        .route("/polls/:id/membership", get(membership_status::<S, B>))
        .route("/polls/:id/commit_status", get(commit_status::<S, B>))
        .route("/polls/:id/commit", post(record_commit::<S, B>))
        .route("/polls/:id/prove", post(generate_proof::<S, B>))
        .route("/polls/:id/reveal", post(reveal_vote::<S, B>))
        .route("/auth/login", post(login::<S, B>))
        .route("/auth/me", get(me))
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
    let membership_root = state.store.membership_root_snapshot().await?;
    let category = body.category.clone();
    let record = state
        .store
        .create_poll(NewPoll {
            question: &body.question,
            options: &body.options,
            commit_phase_end: body.commit_phase_end,
            reveal_phase_end: body.reveal_phase_end,
            membership_root: &membership_root,
            category: &category,
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
    headers: axum::http::HeaderMap,
    Json(body): Json<CommitRequest>,
) -> Result<Json<CommitResponse>, AppError>
where
    S: PollStore + Send + Sync,
{
    let poll = state.store.get_poll(poll_id).await?;
    if Utc::now() >= poll.commit_phase_end {
        return Err(AppError::Validation("commit phase over".into()));
    }
    let username = extract_username(&headers)?
        .ok_or_else(|| AppError::Validation("missing auth header".into()))?;
    let identity_secret = derive_identity_secret(&username, &state.identity_salt);
    if !state
        .store
        .poll_includes_member(poll_id, &identity_secret)
        .await?
    {
        return Err(AppError::Validation("not a member of this poll".into()));
    }
    let stored = state
        .store
        .record_commit(StoredCommit {
            poll_id,
            commitment: &body.commitment,
            identity_secret: &identity_secret,
        })
        .await?;
    Ok(Json(CommitResponse {
        poll_id: stored.poll_id,
        commitment: stored.commitment,
        recorded_at: stored.recorded_at,
        identity_secret: stored.identity_secret,
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

async fn membership_status<S, B>(
    State(state): State<AppState<S, B>>,
    Path(poll_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<MembershipStatusResponse>, AppError>
where
    S: PollStore + Send + Sync,
{
    let poll = state.store.get_poll(poll_id).await?;
    let username = extract_username(&headers)?;
    let is_member = if let Some(u) = username {
        let id = derive_identity_secret(&u, &state.identity_salt);
        state.store.poll_includes_member(poll_id, &id).await?
    } else {
        false
    };
    Ok(Json(MembershipStatusResponse {
        poll_id,
        membership_root: poll.membership_root,
        is_member,
    }))
}

async fn commit_status<S, B>(
    State(state): State<AppState<S, B>>,
    Path(poll_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<CommitStatusResponse>, AppError>
where
    S: PollStore + Send + Sync,
{
    let username = extract_username(&headers)?
        .ok_or_else(|| AppError::Validation("missing auth header".into()))?;
    let identity = derive_identity_secret(&username, &state.identity_salt);
    let already = state.store.has_commit(poll_id, &identity).await?;
    Ok(Json(CommitStatusResponse {
        poll_id,
        already_committed: already,
    }))
}

async fn login<S, B>(
    State(state): State<AppState<S, B>>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError>
where
    S: PollStore + Send + Sync,
    B: ZkBackend + Send + Sync,
{
    // Demo: accept any username/password and issue a simple token
    if body.username.is_empty() || body.password.is_empty() {
        return Err(AppError::Validation("username/password required".into()));
    }
    // Derive identity_secret from username + salt, upsert into members.
    let identity = derive_identity_secret(&body.username, &state.identity_salt);
    state.store.ensure_member(&identity).await?;
    let token = format!("token:{}", body.username);
    Ok(Json(LoginResponse {
        token,
        username: body.username,
        identity_secret: identity,
    }))
}

async fn me(headers: axum::http::HeaderMap) -> Result<Json<MeResponse>, AppError> {
    let username = extract_username(&headers)?
        .ok_or_else(|| AppError::Validation("missing auth header".into()))?;
    // We don't have state here; reuse the same salt used at app init
    let identity = derive_identity_secret(&username, IDENTITY_SALT.get().unwrap());
    Ok(Json(MeResponse {
        username: username.clone(),
        identity_secret: identity,
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

fn extract_username(headers: &HeaderMap) -> AppResult<Option<String>> {
    let Some(raw) = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
    else {
        return Ok(None);
    };
    let prefix = "Bearer ";
    if !raw.starts_with(prefix) {
        return Err(AppError::Validation("invalid auth header".into()));
    }
    let token = raw.trim_start_matches(prefix);
    let username = token.trim_start_matches("token:");
    if username.is_empty() {
        return Err(AppError::Validation("invalid token".into()));
    }
    Ok(Some(username.to_string()))
}

fn derive_identity_secret(username: &str, salt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(username.as_bytes());
    let out = hasher.finalize();
    format!("0x{}", hex::encode(out))
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
        category: record.category,
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
    identity_salt: String,
    commit_sync_interval_ms: u64,
    relayer_private_key: Option<String>,
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
        let identity_salt = std::env::var("IDENTITY_SALT").unwrap_or_else(|_| "demo-salt".to_string());
        let commit_sync_interval_ms = std::env::var("COMMIT_SYNC_INTERVAL_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30_000);
        let relayer_private_key = std::env::var("RELAYER_PRIVATE_KEY").ok().filter(|s| !s.is_empty());
        Self {
            database_url,
            bind,
            rpc_ws,
            contract_address,
            indexer_from_block,
            identity_salt,
            commit_sync_interval_ms,
            relayer_private_key,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::hash_members;
    use axum::body::to_bytes;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use std::sync::Mutex;
    use tower::ServiceExt;

    fn test_app() -> Router {
        let store = Arc::new(InMemoryStore::default());
        let zk = Arc::new(NoopZkBackend::default());
        let state = AppState::new(store, zk, "test-salt".to_string());
        app_router(state)
    }

    #[tokio::test]
    async fn create_and_get_poll() {
        let store = Arc::new(InMemoryStore::default());
        // seed two members so membership_root is non-zero
        store.add_member("alice_secret").await;
        store.add_member("bob_secret").await;
        let expected_root =
            hash_members(&vec!["alice_secret".to_string(), "bob_secret".to_string()]);
        let zk = Arc::new(NoopZkBackend::default());
        let app = app_router(AppState::new(store, zk, "test-salt".to_string()));

        let body = serde_json::json!({
            "question": "Will it rain?",
            "options": ["Yes", "No"],
            "commit_phase_end": Utc::now(),
            "reveal_phase_end": Utc::now() + chrono::Duration::minutes(30)
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
        let body_bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let created: PollResponse = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(created.membership_root, expected_root);

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
        // login to seed membership
        let login_body = serde_json::json!({
            "username": "alice",
            "password": "pw"
        });
        let login_res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(login_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(login_res.status(), StatusCode::OK);
        let token = "Bearer token:alice";

        let commit_end = Utc::now() + chrono::Duration::milliseconds(50);
        let reveal_end = commit_end + chrono::Duration::minutes(5);
        let create_body = serde_json::json!({
            "question": "Q",
            "options": ["A", "B"],
            "commit_phase_end": commit_end,
            "reveal_phase_end": reveal_end
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
                    .header("authorization", token)
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

    #[derive(Default, Clone)]
    struct RecordingCommitter {
        calls: Arc<Mutex<Vec<(i64, String)>>>,
    }

    #[async_trait]
    impl OnchainCommitter for RecordingCommitter {
        async fn submit_commit(&self, poll_id: i64, commitment: &str) -> AppResult<()> {
            self.calls
                .lock()
                .unwrap()
                .push((poll_id, commitment.to_string()));
            Ok(())
        }
    }

    #[tokio::test]
    async fn commit_sync_submits_pending_commits() {
        let store = Arc::new(InMemoryStore::default());
        let poll = store
            .create_poll(NewPoll {
                question: "Sync test",
                options: &vec!["Yes".into(), "No".into()],
                commit_phase_end: Utc::now() - chrono::Duration::minutes(1),
                reveal_phase_end: Utc::now() + chrono::Duration::minutes(5),
                membership_root: "root",
                category: "General",
            })
            .await
            .unwrap();
        store
            .record_commit(StoredCommit {
                poll_id: poll.id,
                commitment: "commit-1",
                identity_secret: "id1",
            })
            .await
            .unwrap();
        let committer = Arc::new(RecordingCommitter::default());
        sync_commits_once(store.clone(), committer.clone())
            .await
            .unwrap();
        assert_eq!(committer.calls.lock().unwrap().len(), 1);
        sync_commits_once(store, committer.clone()).await.unwrap();
        assert_eq!(committer.calls.lock().unwrap().len(), 1);
    }
}
