mod doc;
mod error;
mod indexer;
mod repo;
mod types;
mod zk;

use crate::doc::ApiDoc;
use crate::error::{AppError, AppResult};
use crate::indexer::{spawn_indexer, IndexerConfig, PollCreatedEvent};
#[cfg(test)]
use crate::repo::InMemoryStore;
use crate::repo::{
    CommitSyncRow, NewPoll, PgStore, PollRecord, PollStore, StoredCommit, StoredVote,
    UserStatsRecord,
};
use crate::types::{
    CommitRequest, CommitResponse, CommitStatusResponse, CreatePollRequest, CreatePollResponse,
    LoginRequest, LoginResponse, MeResponse, MembershipStatusResponse, Phase, PollResponse,
    ProveRequest, ResolveRequest, RevealRequest, RevealResponse, SecretResponse, UserStatsResponse,
};
use crate::zk::{NoopZkBackend, ProofBundle, ProofRequest, ZkBackend};
use async_trait::async_trait;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use ethers::contract::{abigen, EthLogDecode};
use ethers::core::types::{Bytes, H160, H256, U256};
use ethers::middleware::SignerMiddleware;
use ethers::providers::{Http, Middleware, Provider};
use ethers::signers::{LocalWallet, Signer};
use hex;
use num_bigint::BigUint;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::time::Duration;
use tower_http::cors::CorsLayer;
use tracing::{debug, error, info, warn};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

static IDENTITY_SALT: OnceCell<String> = OnceCell::new();
const BN254_FR_MODULUS: &str =
    "21888242871839275222246405745257275088548364400416034343698204186575808495617";

abigen!(
    VeilCastContract,
    r#"[
        function commit(uint256 pollId, bytes32 commitment)
        function createPoll(string question, string[] options, uint256 commitPhaseEnd, uint256 revealPhaseEnd, uint256 membershipRoot)
        function batchReveal(uint256 pollId, uint8[] choiceIndices, uint256[] commitments, uint256[] nullifiers, bytes[] proofs, bytes32[][] publicInputs)
    ]"#
);

#[async_trait]
pub trait OnchainRevealer: Send + Sync {
    async fn submit_batch_reveal(
        &self,
        poll_id: i64,
        items: &[CommitSyncRow],
    ) -> AppResult<Option<H256>>;
}

#[derive(Clone, Default)]
pub struct NoopRevealer;

#[async_trait]
impl OnchainRevealer for NoopRevealer {
    async fn submit_batch_reveal(
        &self,
        poll_id: i64,
        items: &[CommitSyncRow],
    ) -> AppResult<Option<H256>> {
        info!(
            poll_id,
            count = items.len(),
            "Simulating on-chain batch reveal"
        );
        Ok(None)
    }
}

#[derive(Clone)]
pub struct PollsContractClient {
    contract: VeilCastContract<SignerMiddleware<Provider<Http>, LocalWallet>>,
}

pub struct CreatePollTxResult {
    pub poll_id: i64,
    pub tx_hash: H256,
}

impl PollsContractClient {
    pub async fn new(rpc_url: &str, private_key: &str, contract_address: H160) -> AppResult<Self> {
        let provider = Provider::<Http>::try_from(rpc_url)
            .map_err(|e| AppError::External(format!("rpc provider error: {e}")))?;

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
        let contract = VeilCastContract::new(contract_address, client);
        Ok(Self { contract })
    }

    pub async fn create_poll_onchain(
        &self,
        question: &str,
        options: &[String],
        commit_phase_end: chrono::DateTime<Utc>,
        reveal_phase_end: chrono::DateTime<Utc>,
        membership_root: &str,
    ) -> AppResult<CreatePollTxResult> {
        let commit_u256 = to_unix_u256(commit_phase_end)?;
        let reveal_u256 = to_unix_u256(reveal_phase_end)?;
        let membership_u256 = parse_field_u256(membership_root)?;

        let call = self.contract.create_poll(
            question.to_string(),
            options.to_vec(),
            commit_u256,
            reveal_u256,
            membership_u256,
        );
        let pending = call
            .send()
            .await
            .map_err(|e| AppError::External(format!("send createPoll tx failed: {e}")))?;
        let receipt = pending
            .await
            .map_err(|e| AppError::External(format!("createPoll pending failed: {e}")))?
            .ok_or_else(|| AppError::External("createPoll tx dropped".into()))?;

        let poll_id = receipt
            .logs
            .iter()
            .find_map(|log| PollCreatedEvent::decode_log(&log.clone().into()).ok())
            .map(|ev| ev.poll_id.as_u64() as i64)
            .ok_or_else(|| AppError::External("PollCreated event not found".into()))?;

        Ok(CreatePollTxResult {
            poll_id,
            tx_hash: receipt.transaction_hash,
        })
    }
}

fn parse_field_h256(value: &str) -> AppResult<H256> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(H256::zero());
    }
    let bytes = if let Some(hex_str) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        let hex_clean = if hex_str.len() % 2 == 1 {
            format!("0{hex_str}")
        } else {
            hex_str.to_string()
        };
        hex::decode(hex_clean)
            .map_err(|e| AppError::Validation(format!("invalid bytes32 hex: {e}")))?
    } else {
        let big = BigUint::from_str(trimmed)
            .map_err(|e| AppError::Validation(format!("invalid decimal: {e}")))?;
        big.to_bytes_be()
    };
    if bytes.len() > 32 {
        return Err(AppError::Validation("bytes32 must be <= 32 bytes".into()));
    }
    let mut buf = [0u8; 32];
    let offset = 32 - bytes.len();
    buf[offset..].copy_from_slice(&bytes);
    Ok(H256(buf))
}

fn parse_field_u256(value: &str) -> AppResult<U256> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(U256::zero());
    }
    if let Some(hex_str) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        if hex_str.is_empty() {
            return Ok(U256::zero());
        }
        return U256::from_str_radix(hex_str, 16)
            .map_err(|e| AppError::Validation(format!("invalid hex: {e}")));
    }
    let big = BigUint::from_str(trimmed)
        .map_err(|e| AppError::Validation(format!("invalid decimal: {e}")))?;
    let bytes = big.to_bytes_be();
    Ok(U256::from_big_endian(&bytes))
}

fn to_unix_u256(ts: chrono::DateTime<Utc>) -> AppResult<U256> {
    let seconds = ts.timestamp();
    if seconds < 0 {
        return Err(AppError::Validation(
            "timestamp must be non-negative".into(),
        ));
    }
    Ok(U256::from(seconds as u64))
}

#[async_trait]
impl OnchainRevealer for PollsContractClient {
    async fn submit_batch_reveal(
        &self,
        poll_id: i64,
        items: &[CommitSyncRow],
    ) -> AppResult<Option<H256>> {
        let poll_u256 = if poll_id < 0 {
            return Err(AppError::Validation("invalid poll id".into()));
        } else {
            U256::from(poll_id as u64)
        };
        let mut choices: Vec<u8> = Vec::with_capacity(items.len());
        let mut commitments: Vec<U256> = Vec::with_capacity(items.len());
        let mut nullifiers: Vec<U256> = Vec::with_capacity(items.len());
        let mut proofs: Vec<Bytes> = Vec::with_capacity(items.len());
        let mut publics: Vec<Vec<[u8; 32]>> = Vec::with_capacity(items.len());

        for it in items {
            choices.push(it.choice as u8);
            commitments.push(parse_field_u256(&it.commitment)?);
            nullifiers.push(parse_field_u256(&it.nullifier)?);
            let proof_bytes = hex::decode(it.proof.trim_start_matches("0x"))
                .map_err(|e| AppError::Validation(format!("invalid proof hex: {e}")))?;
            proofs.push(Bytes::from(proof_bytes));
            let mut arr: Vec<[u8; 32]> = Vec::with_capacity(it.public_inputs.len());
            for p in &it.public_inputs {
                let h = parse_field_h256(p)?;
                arr.push(h.0);
            }
            publics.push(arr);
        }

        let call = self.contract.clone().batch_reveal(
            poll_u256,
            choices,
            commitments,
            nullifiers,
            proofs,
            publics,
        );
        let pending = call
            .send()
            .await
            .map_err(|e| AppError::External(format!("send batchReveal failed: {e}")))?;
        let receipt = pending
            .await
            .map_err(|e| AppError::External(format!("batchReveal pending failed: {e}")))?;
        Ok(receipt.map(|r| r.transaction_hash))
    }
}

const REVEAL_BATCH_SIZE: usize = 20;

async fn sync_reveals_once<S>(
    store: Arc<S>,
    revealer: Arc<dyn OnchainRevealer + Send + Sync>,
) -> AppResult<()>
where
    S: PollStore + Send + Sync + 'static,
{
    let pending = store.commits_to_sync(Utc::now(), 200).await?;
    info!(pending = pending.len(), "reveal sync tick");

    // group by poll_id
    let mut by_poll: std::collections::HashMap<i64, Vec<CommitSyncRow>> =
        std::collections::HashMap::new();
    for item in pending {
        by_poll.entry(item.poll_id).or_default().push(item);
    }

    for (poll_id, mut items) in by_poll {
        // chunk by batch size
        while !items.is_empty() {
            let chunk: Vec<CommitSyncRow> =
                items.drain(0..items.len().min(REVEAL_BATCH_SIZE)).collect();
            match revealer.submit_batch_reveal(poll_id, &chunk).await {
                Ok(tx_opt) => {
                    for it in &chunk {
                        store.mark_commit_synced(it.id).await?;
                    }
                    if let Some(tx) = tx_opt {
                        let _ = store
                            .set_reveal_tx_hash(poll_id, &format!("{:#x}", tx))
                            .await;
                    }
                }
                Err(err) => {
                    error!(poll_id, ?err, "Failed to submit batch reveal");
                    break;
                }
            }
        }
        if !store.poll_has_pending_commits(poll_id).await? {
            store.mark_poll_sync_complete(poll_id).await?;
        }
    }
    store.mark_polls_without_pending_commits(Utc::now()).await?;
    Ok(())
}

fn spawn_reveal_sync<S>(
    store: Arc<S>,
    revealer: Arc<dyn OnchainRevealer + Send + Sync>,
    interval: Duration,
) where
    S: PollStore + Send + Sync + 'static,
{
    let store_clone = store.clone();
    let revealer_clone = revealer.clone();
    tokio::spawn(async move {
        if let Err(err) = sync_reveals_once(store_clone, revealer_clone).await {
            warn!(?err, "initial reveal sync failed");
        }
    });
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            info!("running reveal sync job");
            if let Err(err) = sync_reveals_once(store.clone(), revealer.clone()).await {
                warn!(?err, "reveal sync job failed");
            }
        }
    });
}

#[derive(Clone)]
struct AppState<S, B> {
    store: Arc<S>,
    zk: Arc<B>,
    identity_salt: String,
    contract: Option<Arc<PollsContractClient>>,
}

impl<S, B> AppState<S, B> {
    fn new(
        store: Arc<S>,
        zk: Arc<B>,
        identity_salt: String,
        contract: Option<Arc<PollsContractClient>>,
    ) -> Self {
        Self {
            store,
            zk,
            identity_salt,
            contract,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    dotenvy::dotenv().ok();
    let default_level = "debug";
    let base_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(default_level));
    let env_filter = base_filter
        .add_directive("sqlx=warn".parse().unwrap())
        .add_directive("sqlx::query=off".parse().unwrap());
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .init();

    let cfg = Config::from_env();
    let _ = IDENTITY_SALT.set(cfg.identity_salt.clone());
    let pool = PgStore::connect(&cfg.database_url).await?;
    let store = Arc::new(pool);
    let zk = Arc::new(NoopZkBackend::default());

    let contract_client = if let (Some(ref pk), Some(addr), Some(ref rpc_url)) = (
        &cfg.relayer_private_key,
        cfg.contract_address,
        cfg.rpc_url.as_ref(),
    ) {
        match PollsContractClient::new(rpc_url, pk, addr).await {
            Ok(client) => Some(Arc::new(client)),
            Err(err) => {
                warn!(?err, "Failed to init polls contract client");
                None
            }
        }
    } else {
        warn!("RELAYER_PRIVATE_KEY or CONTRACT_ADDRESS missing, contract calls disabled");
        None
    };

    let revealer: Arc<dyn OnchainRevealer> = if let Some(client) = contract_client.clone() {
        info!("On-chain reveal sync enabled");
        client
    } else {
        Arc::new(NoopRevealer::default())
    };
    let app_state = AppState::new(
        store.clone(),
        zk.clone(),
        cfg.identity_salt.clone(),
        contract_client.clone(),
    );

    if std::env::var("XP_BACKFILL").is_ok() {
        info!("XP_BACKFILL flag detected, rebuilding user stats...");
        store.backfill_user_stats().await?;
        info!("XP backfill completed. Exiting.");
        return Ok(());
    }

    info!(
        "VeilCast backend initialized (rpc_url set: {}, contract set: {})",
        cfg.rpc_url.is_some(),
        cfg.contract_address.is_some()
    );
    spawn_reveal_sync(
        app_state.store.clone(),
        revealer,
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
        .route("/polls/:id/secret", get(fetch_secret::<S, B>))
        .route("/polls/:id/commit", post(record_commit::<S, B>))
        .route("/polls/:id/prove", post(generate_proof::<S, B>))
        .route("/polls/:id/reveal", post(reveal_vote::<S, B>))
        .route("/polls/:id/resolve", post(resolve_poll::<S, B>))
        .route("/users/me/stats", get(me_stats::<S, B>))
        .route("/leaderboard", get(leaderboard::<S, B>))
        .route("/auth/login", post(login::<S, B>))
        .route("/auth/me", get(me))
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    StatusCode::OK
}

async fn create_poll<S, B>(
    State(state): State<AppState<S, B>>,
    headers: HeaderMap,
    Json(body): Json<CreatePollRequest>,
) -> Result<Json<CreatePollResponse>, AppError>
where
    S: PollStore + Send + Sync,
{
    debug!(
        question = %body.question,
        options = body.options.len(),
        commit_end = %body.commit_phase_end,
        reveal_end = %body.reveal_phase_end,
        "create_poll request"
    );
    if body.options.len() < 2 {
        return Err(AppError::Validation("options must be >= 2".into()));
    }
    if body.commit_phase_end >= body.reveal_phase_end {
        return Err(AppError::Validation(
            "commit end must be before reveal end".into(),
        ));
    }
    let owner = extract_username(&headers)?
        .ok_or_else(|| AppError::Validation("missing auth header".into()))?;
    let membership_root = state.store.membership_root_snapshot().await?;
    let options_owned = body.options.clone();
    let new_poll = NewPoll {
        question: &body.question,
        options: &options_owned,
        commit_phase_end: body.commit_phase_end,
        reveal_phase_end: body.reveal_phase_end,
        membership_root: &membership_root,
        category: &body.category,
        owner: &owner,
    };

    if let Some(contract) = state.contract.as_ref() {
        let members = state.store.list_members().await?;
        if members.is_empty() {
            return Err(AppError::Validation(
                "cannot create poll without any allowlisted members".into(),
            ));
        }

        let onchain = contract
            .create_poll_onchain(
                &body.question,
                &body.options,
                body.commit_phase_end,
                body.reveal_phase_end,
                &membership_root,
            )
            .await?;

        let record = state
            .store
            .create_poll_with_id(onchain.poll_id, new_poll, membership_root.clone(), members)
            .await?;
        info!(
            poll_id = record.id,
            tx_hash = ?onchain.tx_hash,
            commit_end = %record.commit_phase_end,
            reveal_end = %record.reveal_phase_end,
            "Poll created on-chain"
        );

        Ok(Json(CreatePollResponse {
            poll: to_response(record),
            tx_hash: format!("{:#x}", onchain.tx_hash),
        }))
    } else {
        warn!("contract client unavailable; storing poll off-chain only");
        let record = state.store.create_poll(new_poll).await?;
        info!(
            poll_id = record.id,
            commit_end = %record.commit_phase_end,
            reveal_end = %record.reveal_phase_end,
            "Poll created off-chain only"
        );
        Ok(Json(CreatePollResponse {
            poll: to_response(record),
            tx_hash: String::new(),
        }))
    }
}

async fn get_poll<S, B>(
    State(state): State<AppState<S, B>>,
    Path(poll_id): Path<i64>,
) -> Result<Json<PollResponse>, AppError>
where
    S: PollStore + Send + Sync,
{
    debug!(poll_id, "get_poll request");
    let record = state.store.get_poll(poll_id).await?;
    Ok(Json(to_response(record)))
}

async fn list_polls<S, B>(
    State(state): State<AppState<S, B>>,
) -> Result<Json<Vec<PollResponse>>, AppError>
where
    S: PollStore + Send + Sync,
{
    debug!("list_polls request");
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
    debug!(poll_id, "record_commit request start");
    let poll = state.store.get_poll(poll_id).await?;
    let now = Utc::now();
    if now >= poll.commit_phase_end {
        return Err(AppError::Validation("commit phase over".into()));
    }
    if body.choice as usize >= poll.options.len() {
        return Err(AppError::Validation("invalid choice".into()));
    }
    let username = extract_username(&headers)?
        .ok_or_else(|| AppError::Validation("missing auth header".into()))?;
    let identity_secret = derive_identity_secret(&username, &state.identity_salt);
    // Fetch or mint per-poll secret server-side
    let server_secret = state
        .store
        .get_or_create_secret(poll_id, &identity_secret)
        .await?;
    if body.secret != server_secret {
        return Err(AppError::Validation("secret mismatch".into()));
    }
    if !state
        .store
        .poll_includes_member(poll_id, &identity_secret)
        .await?
    {
        return Err(AppError::Validation("not a member of this poll".into()));
    }
    let path = state
        .store
        .merkle_path_for_member(poll_id, &identity_secret)
        .await?;
    tracing::debug!(
        poll_id,
        username,
        identity = %identity_secret,
        choice = body.choice,
        commitment = %body.commitment,
        nullifier = %body.nullifier,
        membership_root = %poll.membership_root,
        path_bits = ?path.as_ref().map(|p| &p.bits),
        path_siblings = ?path.as_ref().map(|p| &p.siblings),
        "record_commit inputs"
    );
    let stored = state
        .store
        .record_commit(StoredCommit {
            poll_id,
            choice: body.choice as i16,
            commitment: &body.commitment,
            identity_secret: &identity_secret,
            secret: &body.secret,
            nullifier: &body.nullifier,
            proof: &body.proof,
            public_inputs: &body.public_inputs,
        })
        .await?;
    Ok(Json(CommitResponse {
        poll_id: stored.poll_id,
        commitment: stored.commitment,
        recorded_at: stored.recorded_at,
        identity_secret: stored.identity_secret,
        nullifier: stored.nullifier,
        proof: stored.proof,
        public_inputs: stored.public_inputs,
        choice: stored.choice,
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
    debug!(poll_id, "generate_proof request");
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
    debug!(poll_id, "reveal_vote request");
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

async fn resolve_poll<S, B>(
    State(state): State<AppState<S, B>>,
    Path(poll_id): Path<i64>,
    headers: HeaderMap,
    Json(body): Json<ResolveRequest>,
) -> Result<Json<PollResponse>, AppError>
where
    S: PollStore + Send + Sync,
{
    let username = extract_username(&headers)?
        .ok_or_else(|| AppError::Validation("missing auth header".into()))?;
    let poll = state.store.get_poll(poll_id).await?;
    if poll.owner != username {
        return Err(AppError::Validation("not poll owner".into()));
    }
    if poll.resolved {
        return Err(AppError::Validation("poll already resolved".into()));
    }
    if Utc::now() < poll.reveal_phase_end {
        return Err(AppError::Validation(
            "cannot resolve before reveal phase ends".into(),
        ));
    }
    if body.correct_option as usize >= poll.options.len() {
        return Err(AppError::Validation("invalid correct option".into()));
    }
    let updated = state
        .store
        .resolve_poll(poll_id, body.correct_option)
        .await?;
    Ok(Json(to_response(updated)))
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
    debug!(poll_id, username, "membership_status request");
    let (is_member, path) = if let Some(ref u) = username {
        let id = derive_identity_secret(&u, &state.identity_salt);
        let m = state.store.merkle_path_for_member(poll_id, &id).await?;
        (m.is_some(), m)
    } else {
        (false, None)
    };
    if let Some(path) = path.as_ref() {
        tracing::debug!(
            poll_id,
            username,
            bits = ?path.bits,
            siblings = ?path.siblings,
            root = %poll.membership_root,
            "membership path response"
        );
    } else {
        tracing::debug!(poll_id, username, "membership path absent");
    }
    Ok(Json(MembershipStatusResponse {
        poll_id,
        membership_root: poll.membership_root,
        is_member,
        path_bits: path.as_ref().map(|p| p.bits.clone()),
        path_siblings: path.as_ref().map(|p| p.siblings.clone()),
    }))
}

async fn fetch_secret<S, B>(
    State(state): State<AppState<S, B>>,
    Path(poll_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<SecretResponse>, AppError>
where
    S: PollStore + Send + Sync,
{
    let username = extract_username(&headers)?
        .ok_or_else(|| AppError::Validation("missing auth header".into()))?;
    let _ = state.store.get_poll(poll_id).await?;
    let identity_secret = derive_identity_secret(&username, &state.identity_salt);
    if !state
        .store
        .poll_includes_member(poll_id, &identity_secret)
        .await?
    {
        return Err(AppError::Validation("not a member of this poll".into()));
    }
    let secret = state
        .store
        .get_or_create_secret(poll_id, &identity_secret)
        .await?;
    Ok(Json(SecretResponse { poll_id, secret }))
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
    debug!(poll_id, username, "commit_status request");
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
    debug!(user = %body.username, "login request");
    // Demo: accept any username/password and issue a simple token
    if body.username.is_empty() || body.password.is_empty() {
        return Err(AppError::Validation("username/password required".into()));
    }
    // Derive identity_secret from username + salt, upsert into members.
    let identity = derive_identity_secret(&body.username, &state.identity_salt);
    state.store.ensure_member(&body.username, &identity).await?;
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
    debug!(username, "me request");
    // We don't have state here; reuse the same salt used at app init
    let identity = derive_identity_secret(&username, IDENTITY_SALT.get().unwrap());
    Ok(Json(MeResponse {
        username: username.clone(),
        identity_secret: identity,
    }))
}

#[derive(Debug, Deserialize)]
struct LeaderboardParams {
    limit: Option<i64>,
}

async fn leaderboard<S, B>(
    State(state): State<AppState<S, B>>,
    Query(params): Query<LeaderboardParams>,
) -> Result<Json<Vec<UserStatsResponse>>, AppError>
where
    S: PollStore + Send + Sync,
    B: ZkBackend + Send + Sync,
{
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let records = state.store.leaderboard(limit).await?;
    let responses = records
        .into_iter()
        .enumerate()
        .map(|(idx, rec)| to_user_stats_response(rec, Some(idx + 1)))
        .collect();
    Ok(Json(responses))
}

async fn me_stats<S, B>(
    State(state): State<AppState<S, B>>,
    headers: HeaderMap,
) -> Result<Json<UserStatsResponse>, AppError>
where
    S: PollStore + Send + Sync,
    B: ZkBackend + Send + Sync,
{
    let username = extract_username(&headers)?
        .ok_or_else(|| AppError::Validation("missing auth header".into()))?;
    let identity = derive_identity_secret(&username, &state.identity_salt);
    let stats = state.store.user_stats(&identity).await?;
    Ok(Json(to_user_stats_response(stats, None)))
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
    let value = BigUint::from_bytes_be(&out);
    let modulus =
        BigUint::parse_bytes(BN254_FR_MODULUS.as_bytes(), 10).expect("valid BN254 modulus");
    let reduced = value % modulus;
    reduced.to_str_radix(10)
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
        owner: record.owner,
        reveal_tx_hash: record.reveal_tx_hash,
        correct_option: record.correct_option,
        resolved: record.resolved,
        commit_sync_completed: record.commit_sync_completed,
        phase,
        vote_counts: record.vote_counts,
    }
}

fn to_user_stats_response(record: UserStatsRecord, rank: Option<usize>) -> UserStatsResponse {
    let accuracy = if record.total_votes > 0 {
        (record.correct_votes as f64 / record.total_votes as f64) * 100.0
    } else {
        0.0
    };
    UserStatsResponse {
        username: record.username,
        tier: record.tier,
        xp: record.xp,
        total_votes: record.total_votes,
        correct_votes: record.correct_votes,
        accuracy,
        rank: rank.map(|r| r as u32),
    }
}

#[derive(Clone, Debug)]
struct Config {
    database_url: String,
    bind: String,
    rpc_url: Option<String>,
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
        let rpc_url = std::env::var("RPC_URL").ok();
        let rpc_ws = std::env::var("RPC_WS").ok();
        let contract_address = std::env::var("CONTRACT_ADDRESS")
            .ok()
            .and_then(|s| H160::from_str(&s).ok());
        let indexer_from_block = std::env::var("INDEXER_FROM_BLOCK")
            .ok()
            .and_then(|s| s.parse().ok());
        let identity_salt =
            std::env::var("IDENTITY_SALT").unwrap_or_else(|_| "demo-salt".to_string());
        let commit_sync_interval_ms = std::env::var("COMMIT_SYNC_INTERVAL_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30_000);
        let relayer_private_key = std::env::var("RELAYER_PRIVATE_KEY")
            .ok()
            .filter(|s| !s.is_empty());
        Self {
            database_url,
            bind,
            rpc_url,
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
        let state = AppState::new(store, zk, "test-salt".to_string(), None);
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
        let app = app_router(AppState::new(store, zk, "test-salt".to_string(), None));

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
                    .header("authorization", "Bearer token:owner")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body_bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let created: CreatePollResponse = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(created.poll.membership_root, expected_root);

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
        let identity = derive_identity_secret("alice", "test-salt");

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
                    .header("authorization", token)
                    .body(Body::from(create_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // fetch per-poll secret
        let secret_res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/polls/0/secret")
                    .header("authorization", token)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(secret_res.status(), StatusCode::OK);
        let secret_body: SecretResponse =
            serde_json::from_slice(&to_bytes(secret_res.into_body(), usize::MAX).await.unwrap())
                .unwrap();

        // generate proof client-side equivalent via endpoint for test convenience
        let prove_body = serde_json::json!({
            "choice": 1,
            "secret": secret_body.secret,
            "identity_secret": identity
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
        let bundle: ProofBundle =
            serde_json::from_slice(&to_bytes(prove_res.into_body(), usize::MAX).await.unwrap())
                .unwrap();

        let commit_body = serde_json::json!({
            "choice": 1,
            "secret": secret_body.secret,
            "commitment": bundle.commitment,
            "nullifier": bundle.nullifier,
            "proof": bundle.proof,
            "public_inputs": bundle.public_inputs
        });
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

        let reveal_body = serde_json::json!({
            "proof": bundle.proof,
            "public_inputs": bundle.public_inputs,
            "commitment": bundle.commitment,
            "nullifier": bundle.nullifier
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
    struct RecordingRevealer {
        calls: Arc<Mutex<Vec<(i64, usize)>>>,
    }

    #[async_trait]
    impl OnchainRevealer for RecordingRevealer {
        async fn submit_batch_reveal(
            &self,
            poll_id: i64,
            items: &[CommitSyncRow],
        ) -> AppResult<Option<H256>> {
            self.calls.lock().unwrap().push((poll_id, items.len()));
            Ok(None)
        }
    }

    #[tokio::test]
    async fn reveal_sync_submits_pending_batches() {
        let store = Arc::new(InMemoryStore::default());
        let poll = store
            .create_poll(NewPoll {
                question: "Sync test",
                options: &vec!["Yes".into(), "No".into()],
                commit_phase_end: Utc::now() - chrono::Duration::minutes(1),
                reveal_phase_end: Utc::now() + chrono::Duration::minutes(5),
                membership_root: "root",
                category: "General",
                owner: "tester",
            })
            .await
            .unwrap();
        store
            .record_commit(StoredCommit {
                poll_id: poll.id,
                choice: 0,
                commitment: "0x1",
                identity_secret: "id1",
                secret: "server-secret",
                nullifier: "0x2",
                proof: "0x00",
                public_inputs: &vec!["0x0".to_string()],
            })
            .await
            .unwrap();
        let revealer = Arc::new(RecordingRevealer::default());
        sync_reveals_once(store.clone(), revealer.clone())
            .await
            .unwrap();
        assert_eq!(revealer.calls.lock().unwrap().len(), 1);
        sync_reveals_once(store, revealer.clone()).await.unwrap();
        assert_eq!(revealer.calls.lock().unwrap().len(), 1);
    }
}
