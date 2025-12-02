use crate::error::{AppError, AppResult};
use crate::repo::{NewPoll, PollIndexSink};
use chrono::{DateTime, Utc};
use ethers::abi::RawLog;
use ethers::contract::EthEvent;
use ethers::core::types::{Filter, Log, H160, U256, U64};
use ethers::providers::{Middleware, Provider, StreamExt, Ws};
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{error, info};

#[derive(Debug, Clone, EthEvent)]
#[ethevent(
    name = "PollCreated",
    abi = "PollCreated(uint256,string,string[],uint256,uint256,uint256)"
)]
pub struct PollCreatedEvent {
    #[ethevent(indexed)]
    pub poll_id: U256,
    pub question: String,
    pub options: Vec<String>,
    pub commit_phase_end: U256,
    pub reveal_phase_end: U256,
    pub membership_root: U256,
}

#[derive(Debug, Clone, EthEvent)]
#[ethevent(name = "VoteRevealed", abi = "VoteRevealed(uint256,uint8,uint256)")]
pub struct VoteRevealedEvent {
    pub poll_id: U256,
    pub choice_index: u8,
    pub nullifier: U256,
}

#[derive(Debug, Clone, EthEvent)]
#[ethevent(name = "PollResolved", abi = "PollResolved(uint256,uint8)")]
pub struct PollResolvedEvent {
    pub poll_id: U256,
    pub correct_option: u8,
}

#[derive(Clone, Debug)]
pub struct IndexerConfig {
    pub rpc_ws: String,
    pub contract_address: H160,
    pub from_block: Option<u64>,
}

pub async fn spawn_indexer<S>(cfg: IndexerConfig, store: Arc<S>) -> JoinHandle<()>
where
    S: PollIndexSink + Send + Sync + 'static,
{
    tokio::spawn(async move {
        if let Err(e) = run_indexer(cfg, store.clone()).await {
            error!("indexer exited with error: {:?}", e);
        }
    })
}

async fn run_indexer<S>(cfg: IndexerConfig, store: Arc<S>) -> AppResult<()>
where
    S: PollIndexSink + Send + Sync + 'static,
{
    let provider = Provider::<Ws>::connect(cfg.rpc_ws.clone())
        .await
        .map_err(|e| AppError::Validation(format!("ws connect failed: {e}")))?;

    let from_block = cfg.from_block.map(U64::from);
    let filter = Filter::new()
        .address(cfg.contract_address)
        .from_block(from_block.unwrap_or_else(|| U64::from(0u64)));

    let mut stream = provider
        .subscribe_logs(&filter)
        .await
        .map_err(|e| AppError::Validation(format!("subscribe failed: {e}")))?;

    info!(
        "Indexer listening on {} for contract {:?}, from_block={:?}",
        cfg.rpc_ws, cfg.contract_address, from_block
    );

    while let Some(log) = stream.next().await {
        if let Err(err) = handle_log(&store, log).await {
            error!("indexer handle_log error: {err:?}");
        }
    }

    Ok(())
}

pub async fn handle_log<S>(store: &Arc<S>, log: Log) -> AppResult<()>
where
    S: PollIndexSink + Send + Sync + 'static,
{
    let raw: RawLog = log.clone().into();
    if let Ok(ev) = PollCreatedEvent::decode_log(&raw) {
        let poll_id = ev.poll_id.as_u64() as i64;
        let commit_end = to_ts(ev.commit_phase_end)?;
        let reveal_end = to_ts(ev.reveal_phase_end)?;
        let question_owned = ev.question.clone();
        let options_owned = ev.options.clone();
        let membership_owned = ev.membership_root.to_string();
        let category_owned = "General".to_string();
        let np = NewPoll {
            question: &question_owned,
            options: &options_owned,
            commit_phase_end: commit_end,
            reveal_phase_end: reveal_end,
            membership_root: &membership_owned,
            category: &category_owned,
        };
        store.upsert_poll_from_chain(poll_id, np).await?;
        info!("Indexed PollCreated poll_id={}", poll_id);
        return Ok(());
    }

    if let Ok(ev) = VoteRevealedEvent::decode_log(&raw) {
        let poll_id = ev.poll_id.as_u64() as i64;
        store
            .upsert_vote_from_chain(poll_id, &ev.nullifier.to_string(), ev.choice_index)
            .await?;
        info!(
            "Indexed VoteRevealed poll_id={} nullifier={}",
            poll_id, ev.nullifier
        );
        return Ok(());
    }

    if let Ok(ev) = PollResolvedEvent::decode_log(&raw) {
        let poll_id = ev.poll_id.as_u64() as i64;
        store
            .resolve_poll_from_chain(poll_id, ev.correct_option)
            .await?;
        info!(
            "Indexed PollResolved poll_id={} correct={}",
            poll_id, ev.correct_option
        );
        return Ok(());
    }

    Ok(())
}

fn to_ts(ts: U256) -> AppResult<DateTime<Utc>> {
    let secs = ts.as_u64() as i64;
    DateTime::from_timestamp(secs, 0)
        .ok_or_else(|| AppError::Validation("invalid timestamp".into()))
}
