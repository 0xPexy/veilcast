use crate::error::{AppError, AppResult};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollRecord {
    pub id: i64,
    pub question: String,
    pub options: Vec<String>,
    pub commit_phase_end: DateTime<Utc>,
    pub reveal_phase_end: DateTime<Utc>,
    pub membership_root: String,
    pub correct_option: Option<i16>,
    pub resolved: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct NewPoll<'a> {
    pub question: &'a str,
    pub options: &'a [String],
    pub commit_phase_end: DateTime<Utc>,
    pub reveal_phase_end: DateTime<Utc>,
    pub membership_root: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct StoredCommit<'a> {
    pub poll_id: i64,
    pub commitment: &'a str,
}

#[derive(Debug, Clone)]
pub struct StoredCommitRecord {
    pub poll_id: i64,
    pub commitment: String,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy)]
pub struct StoredVote<'a> {
    pub poll_id: i64,
    pub nullifier: &'a str,
    pub choice: u8,
}

#[derive(Debug, Clone)]
pub struct StoredVoteRecord {
    pub poll_id: i64,
    pub nullifier: String,
    pub recorded_at: DateTime<Utc>,
}

#[async_trait]
pub trait PollStore {
    async fn create_poll(&self, poll: NewPoll<'_>) -> AppResult<PollRecord>;
    async fn get_poll(&self, poll_id: i64) -> AppResult<PollRecord>;
    async fn record_commit(&self, commit: StoredCommit<'_>) -> AppResult<StoredCommitRecord>;
    async fn record_vote(&self, vote: StoredVote<'_>) -> AppResult<StoredVoteRecord>;
}

/// Postgres-backed store.
#[derive(Clone)]
pub struct PgStore {
    pool: Pool<Postgres>,
}

impl PgStore {
    pub async fn connect(url: &str) -> AppResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(url)
            .await
            .map_err(AppError::Db)?;
        init_schema(&pool).await?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl PollStore for PgStore {
    async fn create_poll(&self, poll: NewPoll<'_>) -> AppResult<PollRecord> {
        let rec = sqlx::query_as::<_, DbPoll>(
            r#"
            INSERT INTO polls (question, options, commit_phase_end, reveal_phase_end, membership_root)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, question, options, commit_phase_end, reveal_phase_end, membership_root, correct_option, resolved
            "#,
        )
        .bind(poll.question)
        .bind(serde_json::to_value(poll.options).unwrap())
        .bind(poll.commit_phase_end)
        .bind(poll.reveal_phase_end)
        .bind(poll.membership_root)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Db)?;

        Ok(rec.into())
    }

    async fn get_poll(&self, poll_id: i64) -> AppResult<PollRecord> {
        let rec = sqlx::query_as::<_, DbPoll>(
            r#"
            SELECT id, question, options, commit_phase_end, reveal_phase_end, membership_root, correct_option, resolved
            FROM polls
            WHERE id = $1
            "#,
        )
        .bind(poll_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Db)?;

        match rec {
            Some(row) => Ok(row.into()),
            None => Err(AppError::NotFound),
        }
    }

    async fn record_commit(&self, commit: StoredCommit<'_>) -> AppResult<StoredCommitRecord> {
        let rec = sqlx::query_as::<_, DbCommit>(
            r#"
            INSERT INTO commitments (poll_id, commitment)
            VALUES ($1, $2)
            RETURNING poll_id, commitment, recorded_at
            "#,
        )
        .bind(commit.poll_id)
        .bind(commit.commitment)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Db)?;
        Ok(rec.into())
    }

    async fn record_vote(&self, vote: StoredVote<'_>) -> AppResult<StoredVoteRecord> {
        let rec = sqlx::query_as::<_, DbVote>(
            r#"
            INSERT INTO votes (poll_id, nullifier, choice)
            VALUES ($1, $2, $3)
            RETURNING poll_id, nullifier, recorded_at
            "#,
        )
        .bind(vote.poll_id)
        .bind(vote.nullifier)
        .bind(vote.choice as i16)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Db)?;
        Ok(rec.into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
struct DbPoll {
    id: i64,
    question: String,
    options: serde_json::Value,
    commit_phase_end: DateTime<Utc>,
    reveal_phase_end: DateTime<Utc>,
    membership_root: String,
    correct_option: Option<i16>,
    resolved: bool,
}

impl From<DbPoll> for PollRecord {
    fn from(value: DbPoll) -> Self {
        let opts: Vec<String> = serde_json::from_value(value.options).unwrap_or_default();
        PollRecord {
            id: value.id,
            question: value.question,
            options: opts,
            commit_phase_end: value.commit_phase_end,
            reveal_phase_end: value.reveal_phase_end,
            membership_root: value.membership_root,
            correct_option: value.correct_option,
            resolved: value.resolved,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
struct DbCommit {
    poll_id: i64,
    commitment: String,
    recorded_at: DateTime<Utc>,
}

impl From<DbCommit> for StoredCommitRecord {
    fn from(value: DbCommit) -> Self {
        StoredCommitRecord {
            poll_id: value.poll_id,
            commitment: value.commitment,
            recorded_at: value.recorded_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
struct DbVote {
    poll_id: i64,
    nullifier: String,
    recorded_at: DateTime<Utc>,
}

impl From<DbVote> for StoredVoteRecord {
    fn from(value: DbVote) -> Self {
        StoredVoteRecord {
            poll_id: value.poll_id,
            nullifier: value.nullifier,
            recorded_at: value.recorded_at,
        }
    }
}

/// Simple in-memory store for tests.
#[derive(Default, Clone)]
pub struct InMemoryStore {
    polls: Arc<RwLock<HashMap<i64, PollRecord>>>,
    commits: Arc<RwLock<Vec<StoredCommitRecord>>>,
    votes: Arc<RwLock<Vec<StoredVoteRecord>>>,
}

#[async_trait]
impl PollStore for InMemoryStore {
    async fn create_poll(&self, poll: NewPoll<'_>) -> AppResult<PollRecord> {
        let mut polls = self.polls.write().await;
        let id = polls.len() as i64;
        let record = PollRecord {
            id,
            question: poll.question.to_string(),
            options: poll.options.to_vec(),
            commit_phase_end: poll.commit_phase_end,
            reveal_phase_end: poll.reveal_phase_end,
            membership_root: poll.membership_root.to_string(),
            correct_option: None,
            resolved: false,
        };
        polls.insert(id, record.clone());
        Ok(record)
    }

    async fn get_poll(&self, poll_id: i64) -> AppResult<PollRecord> {
        let polls = self.polls.read().await;
        polls.get(&poll_id).cloned().ok_or(AppError::NotFound)
    }

    async fn record_commit(&self, commit: StoredCommit<'_>) -> AppResult<StoredCommitRecord> {
        let rec = StoredCommitRecord {
            poll_id: commit.poll_id,
            commitment: commit.commitment.to_string(),
            recorded_at: Utc::now(),
        };
        self.commits.write().await.push(rec.clone());
        Ok(rec)
    }

    async fn record_vote(&self, vote: StoredVote<'_>) -> AppResult<StoredVoteRecord> {
        let rec = StoredVoteRecord {
            poll_id: vote.poll_id,
            nullifier: vote.nullifier.to_string(),
            recorded_at: Utc::now(),
        };
        self.votes.write().await.push(rec.clone());
        Ok(rec)
    }
}

async fn init_schema(pool: &Pool<Postgres>) -> AppResult<()> {
    // Minimal schema for metadata + bookkeeping
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS polls (
            id SERIAL PRIMARY KEY,
            question TEXT NOT NULL,
            options JSONB NOT NULL,
            commit_phase_end TIMESTAMPTZ NOT NULL,
            reveal_phase_end TIMESTAMPTZ NOT NULL,
            membership_root TEXT NOT NULL,
            correct_option SMALLINT,
            resolved BOOLEAN NOT NULL DEFAULT false,
            created_at TIMESTAMPTZ NOT NULL DEFAULT now()
        );
        CREATE TABLE IF NOT EXISTS commitments (
            id SERIAL PRIMARY KEY,
            poll_id BIGINT NOT NULL REFERENCES polls(id) ON DELETE CASCADE,
            commitment TEXT NOT NULL,
            recorded_at TIMESTAMPTZ NOT NULL DEFAULT now()
        );
        CREATE UNIQUE INDEX IF NOT EXISTS commitments_poll_commitment_idx ON commitments(poll_id, commitment);
        CREATE TABLE IF NOT EXISTS votes (
            id SERIAL PRIMARY KEY,
            poll_id BIGINT NOT NULL REFERENCES polls(id) ON DELETE CASCADE,
            nullifier TEXT NOT NULL,
            choice SMALLINT NOT NULL,
            recorded_at TIMESTAMPTZ NOT NULL DEFAULT now()
        );
        CREATE UNIQUE INDEX IF NOT EXISTS votes_nullifier_idx ON votes(poll_id, nullifier);
        "#,
    )
    .execute(pool)
    .await
    .map_err(AppError::Db)?;
    Ok(())
}
