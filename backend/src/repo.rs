use crate::error::{AppError, AppResult};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres, Row};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::RwLock;
use uuid::Uuid;

const MERKLE_SCRIPT: &str = "./scripts/poseidon_merkle_noir.mjs";
const MERKLE_DEPTH: u32 = 20;

pub(crate) fn hash_members(members: &[String]) -> String {
    if members.is_empty() {
        return "0x0".to_string();
    }
    let mut hasher = Sha256::new();
    for m in members {
        hasher.update(m.as_bytes());
    }
    format!("0x{}", hex::encode(hasher.finalize()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollRecord {
    pub id: i64,
    pub question: String,
    pub options: Vec<String>,
    pub commit_phase_end: DateTime<Utc>,
    pub reveal_phase_end: DateTime<Utc>,
    pub category: String,
    pub membership_root: String,
    pub correct_option: Option<i16>,
    pub resolved: bool,
    pub commit_sync_completed: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct NewPoll<'a> {
    pub question: &'a str,
    pub options: &'a [String],
    pub commit_phase_end: DateTime<Utc>,
    pub reveal_phase_end: DateTime<Utc>,
    pub membership_root: &'a str,
    pub category: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct StoredCommit<'a> {
    pub poll_id: i64,
    pub choice: i16,
    pub commitment: &'a str,
    pub identity_secret: &'a str,
    pub nullifier: &'a str,
    pub proof: &'a str,
    pub public_inputs: &'a [String],
}

#[derive(Debug, Clone)]
pub struct StoredCommitRecord {
    pub id: i64,
    pub poll_id: i64,
    pub choice: i16,
    pub commitment: String,
    pub identity_secret: String,
    pub recorded_at: DateTime<Utc>,
    pub nullifier: String,
    pub proof: String,
    pub public_inputs: Vec<String>,
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

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CommitSyncRow {
    pub id: i64,
    pub poll_id: i64,
    pub choice: i16,
    pub commitment: String,
    pub nullifier: String,
    pub proof: String,
    pub public_inputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerklePath {
    pub bits: Vec<String>,
    pub siblings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleResult {
    pub root: String,
    pub paths: std::collections::HashMap<String, MerklePath>,
    pub depth: u32,
}

#[async_trait]
pub trait PollStore {
    async fn create_poll(&self, poll: NewPoll<'_>) -> AppResult<PollRecord>;
    async fn create_poll_with_id(
        &self,
        poll_id: i64,
        poll: NewPoll<'_>,
        membership_root: String,
        members: Vec<String>,
    ) -> AppResult<PollRecord>;
    async fn list_polls(&self, limit: i64) -> AppResult<Vec<PollRecord>>;
    async fn get_poll(&self, poll_id: i64) -> AppResult<PollRecord>;
    async fn record_commit(&self, commit: StoredCommit<'_>) -> AppResult<StoredCommitRecord>;
    async fn record_vote(&self, vote: StoredVote<'_>) -> AppResult<StoredVoteRecord>;
    async fn membership_root_snapshot(&self) -> AppResult<String>;
    async fn merkle_path_for_member(
        &self,
        poll_id: i64,
        identity_secret: &str,
    ) -> AppResult<Option<MerklePath>>;
    async fn list_members(&self) -> AppResult<Vec<String>>;
    async fn ensure_member(&self, identity_secret: &str) -> AppResult<()>;
    async fn poll_includes_member(&self, poll_id: i64, identity_secret: &str) -> AppResult<bool>;
    async fn nullifier_used(&self, poll_id: i64, nullifier: &str) -> AppResult<bool>;
    async fn has_commit(&self, poll_id: i64, identity_secret: &str) -> AppResult<bool>;
    async fn commits_to_sync(
        &self,
        now: DateTime<Utc>,
        limit: i64,
    ) -> AppResult<Vec<CommitSyncRow>>;
    async fn mark_commit_synced(&self, commit_id: i64) -> AppResult<()>;
    async fn poll_has_pending_commits(&self, poll_id: i64) -> AppResult<bool>;
    async fn mark_poll_sync_complete(&self, poll_id: i64) -> AppResult<()>;
    async fn mark_polls_without_pending_commits(&self, now: DateTime<Utc>) -> AppResult<()>;
}

#[async_trait]
pub trait PollIndexSink {
    async fn upsert_poll_from_chain(&self, poll_id: i64, poll: NewPoll<'_>) -> AppResult<()>;
    async fn upsert_vote_from_chain(
        &self,
        poll_id: i64,
        nullifier: &str,
        choice: u8,
    ) -> AppResult<()>;
    async fn resolve_poll_from_chain(&self, poll_id: i64, correct_option: u8) -> AppResult<()>;
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

    async fn poll_member_list(&self, poll_id: i64) -> AppResult<Vec<String>> {
        let rows = sqlx::query(
            r#"
            SELECT identity_secret
            FROM poll_members
            WHERE poll_id = $1
            ORDER BY identity_secret
            "#,
        )
        .bind(poll_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Db)?;
        Ok(rows
            .into_iter()
            .filter_map(|r| r.try_get::<String, _>("identity_secret").ok())
            .collect())
    }

    async fn run_poseidon_merkle(&self, members: &[String]) -> AppResult<MerkleResult> {
        // Write members to temp file
        let tmp_path = std::env::temp_dir().join(format!("members-{}.json", Uuid::new_v4()));
        let payload = serde_json::json!({
            "members": members,
            "depth": MERKLE_DEPTH,
        });
        tokio::fs::write(&tmp_path, payload.to_string())
            .await
            .map_err(AppError::Io)?;

        let output = Command::new("node")
            .arg(MERKLE_SCRIPT)
            .arg(&tmp_path)
            .output()
            .await
            .map_err(|e| AppError::External(e.to_string()))?;

        // Clean up temp file
        let _ = tokio::fs::remove_file(&tmp_path).await;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::External(format!(
                "poseidon merkle script failed: {stderr}"
            )));
        }
        let res: MerkleResult = serde_json::from_slice(&output.stdout)
            .map_err(|e| AppError::External(e.to_string()))?;
        Ok(res)
    }

    async fn current_members(&self) -> AppResult<Vec<String>> {
        let rows = sqlx::query(
            r#"
            SELECT identity_secret FROM members ORDER BY identity_secret
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Db)?;
        Ok(rows
            .into_iter()
            .filter_map(|r| r.try_get::<String, _>("identity_secret").ok())
            .collect())
    }
    async fn next_poll_sequence(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT nextval(pg_get_serial_sequence('polls','id'))")
            .fetch_one(&self.pool)
            .await
    }

    async fn insert_poll_with_members(
        &self,
        poll_id: i64,
        poll: NewPoll<'_>,
        membership_root: String,
        members: Vec<String>,
        adjust_sequence: bool,
    ) -> AppResult<PollRecord> {
        let mut tx = self.pool.begin().await.map_err(AppError::Db)?;
        let rec = sqlx::query_as::<_, DbPoll>(
            r#"
            INSERT INTO polls (id, question, options, commit_phase_end, reveal_phase_end, category, membership_root, commit_sync_completed)
            VALUES ($1, $2, $3, $4, $5, $6, $7, false)
            ON CONFLICT (id) DO UPDATE SET
                question = EXCLUDED.question,
                options = EXCLUDED.options,
                commit_phase_end = EXCLUDED.commit_phase_end,
                reveal_phase_end = EXCLUDED.reveal_phase_end,
                category = EXCLUDED.category,
                membership_root = EXCLUDED.membership_root
            RETURNING id, question, options, commit_phase_end, reveal_phase_end, category, membership_root, correct_option, resolved, commit_sync_completed
            "#,
        )
        .bind(poll_id)
        .bind(poll.question)
        .bind(serde_json::to_value(poll.options).unwrap())
        .bind(poll.commit_phase_end)
        .bind(poll.reveal_phase_end)
        .bind(poll.category)
        .bind(membership_root)
        .fetch_one(&mut *tx)
        .await
        .map_err(AppError::Db)?;

        for m in members {
            sqlx::query(
                r#"
                INSERT INTO poll_members (poll_id, identity_secret)
                VALUES ($1, $2)
                ON CONFLICT DO NOTHING
                "#,
            )
            .bind(poll_id)
            .bind(m)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Db)?;
        }

        if adjust_sequence {
            sqlx::query(
                r#"
                SELECT setval(
                    pg_get_serial_sequence('polls','id'),
                    GREATEST($1, (SELECT COALESCE(MAX(id),0) + 1 FROM polls))
                )
                "#,
            )
            .bind(poll_id + 1)
            .fetch_one(&mut *tx)
            .await
            .map_err(AppError::Db)?;
        }

        tx.commit().await.map_err(AppError::Db)?;
        Ok(rec.into())
    }
}

#[async_trait]
impl PollStore for PgStore {
    async fn create_poll(&self, poll: NewPoll<'_>) -> AppResult<PollRecord> {
        let members = self.current_members().await?;
        let merkle = self.run_poseidon_merkle(&members).await?;
        let computed_root = merkle.root;
        let poll_id = self.next_poll_sequence().await.map_err(AppError::Db)?;
        self.insert_poll_with_members(poll_id, poll, computed_root, members, false)
            .await
    }

    async fn create_poll_with_id(
        &self,
        poll_id: i64,
        poll: NewPoll<'_>,
        membership_root: String,
        members: Vec<String>,
    ) -> AppResult<PollRecord> {
        self.insert_poll_with_members(poll_id, poll, membership_root, members, true)
            .await
    }

    async fn list_polls(&self, limit: i64) -> AppResult<Vec<PollRecord>> {
        let rows = sqlx::query_as::<_, DbPoll>(
            r#"
            SELECT id, question, options, commit_phase_end, reveal_phase_end, category, membership_root, correct_option, resolved, commit_sync_completed
            FROM polls
            ORDER BY id DESC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Db)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn get_poll(&self, poll_id: i64) -> AppResult<PollRecord> {
        let rec = sqlx::query_as::<_, DbPoll>(
            r#"
            SELECT id, question, options, commit_phase_end, reveal_phase_end, category, membership_root, correct_option, resolved, commit_sync_completed
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
            INSERT INTO commitments (poll_id, choice, commitment, identity_secret, nullifier, proof, public_inputs)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, poll_id, choice, commitment, identity_secret, nullifier, proof, public_inputs, recorded_at
            "#,
        )
        .bind(commit.poll_id)
        .bind(commit.choice)
        .bind(commit.commitment)
        .bind(commit.identity_secret)
        .bind(commit.nullifier)
        .bind(commit.proof)
        .bind(commit.public_inputs)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Db)?;
        Ok(rec.into())
    }

    async fn record_vote(&self, vote: StoredVote<'_>) -> AppResult<StoredVoteRecord> {
        if self.nullifier_used(vote.poll_id, vote.nullifier).await? {
            return Err(AppError::Validation("nullifier already used".into()));
        }
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

    async fn membership_root_snapshot(&self) -> AppResult<String> {
        let members = self.current_members().await?;
        let merkle = self.run_poseidon_merkle(&members).await?;
        Ok(merkle.root)
    }

    async fn list_members(&self) -> AppResult<Vec<String>> {
        self.current_members().await
    }

    async fn merkle_path_for_member(
        &self,
        poll_id: i64,
        identity_secret: &str,
    ) -> AppResult<Option<MerklePath>> {
        let members = self.poll_member_list(poll_id).await?;
        if members.is_empty() {
            return Ok(None);
        }
        if !members.iter().any(|m| m == identity_secret) {
            return Ok(None);
        }
        let merkle = self.run_poseidon_merkle(&members).await?;
        Ok(merkle.paths.get(identity_secret).cloned())
    }

    async fn ensure_member(&self, identity_secret: &str) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO members (identity_secret)
            VALUES ($1)
            ON CONFLICT (identity_secret) DO NOTHING
            "#,
        )
        .bind(identity_secret)
        .execute(&self.pool)
        .await
        .map_err(AppError::Db)?;
        Ok(())
    }

    async fn poll_includes_member(&self, poll_id: i64, identity_secret: &str) -> AppResult<bool> {
        let row = sqlx::query_scalar::<_, i32>(
            r#"
            SELECT 1 FROM poll_members WHERE poll_id = $1 AND identity_secret = $2 LIMIT 1
            "#,
        )
        .bind(poll_id)
        .bind(identity_secret)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Db)?;
        Ok(row.is_some())
    }

    async fn nullifier_used(&self, poll_id: i64, nullifier: &str) -> AppResult<bool> {
        let row = sqlx::query_scalar::<_, i32>(
            r#"
            SELECT 1 FROM votes WHERE poll_id = $1 AND nullifier = $2 LIMIT 1
            "#,
        )
        .bind(poll_id)
        .bind(nullifier)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Db)?;
        Ok(row.is_some())
    }

    async fn has_commit(&self, poll_id: i64, identity_secret: &str) -> AppResult<bool> {
        let row = sqlx::query_scalar::<_, i32>(
            r#"
            SELECT 1 FROM commitments WHERE poll_id = $1 AND identity_secret = $2 LIMIT 1
            "#,
        )
        .bind(poll_id)
        .bind(identity_secret)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Db)?;
        Ok(row.is_some())
    }

    async fn commits_to_sync(
        &self,
        now: DateTime<Utc>,
        limit: i64,
    ) -> AppResult<Vec<CommitSyncRow>> {
        let rows = sqlx::query_as::<_, CommitSyncRow>(
            r#"
            SELECT c.id::BIGINT as id, c.poll_id, c.choice, c.commitment, c.nullifier, c.proof, c.public_inputs
            FROM commitments c
            JOIN polls p ON p.id = c.poll_id
            WHERE p.commit_phase_end <= $1
              AND p.reveal_phase_end > $1
              AND p.commit_sync_completed = false
              AND c.onchain_submitted = false
            ORDER BY c.id
            LIMIT $2
            "#,
        )
        .bind(now)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Db)?;
        Ok(rows)
    }

    async fn mark_commit_synced(&self, commit_id: i64) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE commitments SET onchain_submitted = true WHERE id = $1
            "#,
        )
        .bind(commit_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::Db)?;
        Ok(())
    }

    async fn poll_has_pending_commits(&self, poll_id: i64) -> AppResult<bool> {
        let row = sqlx::query_scalar::<_, i32>(
            r#"
            SELECT 1 FROM commitments WHERE poll_id = $1 AND onchain_submitted = false LIMIT 1
            "#,
        )
        .bind(poll_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Db)?;
        Ok(row.is_some())
    }

    async fn mark_poll_sync_complete(&self, poll_id: i64) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE polls SET commit_sync_completed = true WHERE id = $1
            "#,
        )
        .bind(poll_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::Db)?;
        Ok(())
    }

    async fn mark_polls_without_pending_commits(&self, now: DateTime<Utc>) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE polls
            SET commit_sync_completed = true
            WHERE commit_phase_end <= $1
              AND commit_sync_completed = false
              AND NOT EXISTS (
                    SELECT 1 FROM commitments c
                    WHERE c.poll_id = polls.id
                      AND c.onchain_submitted = false
                )
            "#,
        )
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(AppError::Db)?;
        Ok(())
    }
}

#[async_trait]
impl PollIndexSink for PgStore {
    async fn upsert_poll_from_chain(&self, poll_id: i64, poll: NewPoll<'_>) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO polls (id, question, options, commit_phase_end, reveal_phase_end, membership_root, category, resolved)
            VALUES ($1, $2, $3, $4, $5, $6, $7, false)
            ON CONFLICT (id) DO UPDATE SET
              question = EXCLUDED.question,
              options = EXCLUDED.options,
              commit_phase_end = EXCLUDED.commit_phase_end,
              reveal_phase_end = EXCLUDED.reveal_phase_end,
              membership_root = EXCLUDED.membership_root,
              category = EXCLUDED.category
            "#,
        )
        .bind(poll_id)
        .bind(poll.question)
        .bind(serde_json::to_value(poll.options).unwrap())
        .bind(poll.commit_phase_end)
        .bind(poll.reveal_phase_end)
        .bind(poll.membership_root)
        .bind(poll.category)
        .execute(&self.pool)
        .await
        .map_err(AppError::Db)?;
        Ok(())
    }

    async fn upsert_vote_from_chain(
        &self,
        poll_id: i64,
        nullifier: &str,
        choice: u8,
    ) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO votes (poll_id, nullifier, choice)
            VALUES ($1, $2, $3)
            ON CONFLICT (poll_id, nullifier) DO NOTHING
            "#,
        )
        .bind(poll_id)
        .bind(nullifier)
        .bind(choice as i16)
        .execute(&self.pool)
        .await
        .map_err(AppError::Db)?;
        Ok(())
    }

    async fn resolve_poll_from_chain(&self, poll_id: i64, correct_option: u8) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE polls
            SET resolved = true, correct_option = $2
            WHERE id = $1
            "#,
        )
        .bind(poll_id)
        .bind(correct_option as i16)
        .execute(&self.pool)
        .await
        .map_err(AppError::Db)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
struct DbPoll {
    id: i64,
    question: String,
    options: serde_json::Value,
    commit_phase_end: DateTime<Utc>,
    reveal_phase_end: DateTime<Utc>,
    category: String,
    membership_root: String,
    correct_option: Option<i16>,
    resolved: bool,
    commit_sync_completed: bool,
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
            category: value.category,
            membership_root: value.membership_root,
            correct_option: value.correct_option,
            resolved: value.resolved,
            commit_sync_completed: value.commit_sync_completed,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
struct DbCommit {
    id: i32,
    poll_id: i64,
    choice: i16,
    commitment: String,
    recorded_at: DateTime<Utc>,
    identity_secret: String,
    nullifier: String,
    proof: String,
    public_inputs: Vec<String>,
}

impl From<DbCommit> for StoredCommitRecord {
    fn from(value: DbCommit) -> Self {
        StoredCommitRecord {
            id: value.id as i64,
            poll_id: value.poll_id,
            choice: value.choice,
            commitment: value.commitment,
            recorded_at: value.recorded_at,
            identity_secret: value.identity_secret,
            nullifier: value.nullifier,
            proof: value.proof,
            public_inputs: value.public_inputs,
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
#[derive(Clone)]
#[allow(dead_code)]
pub struct InMemoryStore {
    polls: Arc<RwLock<HashMap<i64, PollRecord>>>,
    commits: Arc<RwLock<Vec<StoredCommitRecord>>>,
    votes: Arc<RwLock<Vec<StoredVoteRecord>>>,
    members: Arc<RwLock<Vec<String>>>,
    poll_members: Arc<RwLock<HashMap<i64, Vec<String>>>>,
    vote_nullifiers: Arc<RwLock<HashMap<(i64, String), ()>>>,
    commits_by_identity: Arc<RwLock<HashMap<(i64, String), ()>>>,
    synced_commits: Arc<RwLock<HashSet<i64>>>,
    commit_seq: Arc<RwLock<i64>>,
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self {
            polls: Arc::new(RwLock::new(HashMap::new())),
            commits: Arc::new(RwLock::new(Vec::new())),
            votes: Arc::new(RwLock::new(Vec::new())),
            members: Arc::new(RwLock::new(Vec::new())),
            poll_members: Arc::new(RwLock::new(HashMap::new())),
            vote_nullifiers: Arc::new(RwLock::new(HashMap::new())),
            commits_by_identity: Arc::new(RwLock::new(HashMap::new())),
            synced_commits: Arc::new(RwLock::new(HashSet::new())),
            commit_seq: Arc::new(RwLock::new(0)),
        }
    }
}

impl InMemoryStore {
    /// Test helper: pre-seed allowed members for membership_root calculation.
    pub async fn add_member(&self, identity_secret: &str) {
        let mut members = self.members.write().await;
        if !members.contains(&identity_secret.to_string()) {
            members.push(identity_secret.to_string());
        }
    }
}

#[async_trait]
impl PollStore for InMemoryStore {
    async fn create_poll(&self, poll: NewPoll<'_>) -> AppResult<PollRecord> {
        let members = self.members.read().await.clone();
        let root = hash_members(&members);
        let id = self.polls.read().await.len() as i64;
        self.create_poll_with_id(id, poll, root, members).await
    }

    async fn create_poll_with_id(
        &self,
        poll_id: i64,
        poll: NewPoll<'_>,
        membership_root: String,
        members: Vec<String>,
    ) -> AppResult<PollRecord> {
        let mut polls = self.polls.write().await;
        let record = PollRecord {
            id: poll_id,
            question: poll.question.to_string(),
            options: poll.options.to_vec(),
            commit_phase_end: poll.commit_phase_end,
            reveal_phase_end: poll.reveal_phase_end,
            category: poll.category.to_string(),
            membership_root: membership_root.clone(),
            correct_option: None,
            resolved: false,
            commit_sync_completed: false,
        };
        polls.insert(poll_id, record.clone());
        self.poll_members.write().await.insert(poll_id, members);
        Ok(record)
    }

    async fn list_polls(&self, limit: i64) -> AppResult<Vec<PollRecord>> {
        let polls = self.polls.read().await;
        let mut vals: Vec<_> = polls.values().cloned().collect();
        vals.sort_by_key(|p| -(p.id as i64));
        vals.truncate(limit as usize);
        Ok(vals)
    }

    async fn get_poll(&self, poll_id: i64) -> AppResult<PollRecord> {
        let polls = self.polls.read().await;
        polls.get(&poll_id).cloned().ok_or(AppError::NotFound)
    }

    async fn record_commit(&self, commit: StoredCommit<'_>) -> AppResult<StoredCommitRecord> {
        {
            let commits = self.commits.read().await;
            if commits
                .iter()
                .any(|c| c.poll_id == commit.poll_id && c.identity_secret == commit.identity_secret)
            {
                return Err(AppError::Validation(
                    "already committed for this poll".into(),
                ));
            }
        }
        let mut seq = self.commit_seq.write().await;
        let id = *seq;
        *seq += 1;
        let rec = StoredCommitRecord {
            id,
            poll_id: commit.poll_id,
            choice: commit.choice,
            commitment: commit.commitment.to_string(),
            identity_secret: commit.identity_secret.to_string(),
            recorded_at: Utc::now(),
            nullifier: commit.nullifier.to_string(),
            proof: commit.proof.to_string(),
            public_inputs: commit.public_inputs.to_vec(),
        };
        self.commits.write().await.push(rec.clone());
        self.commits_by_identity
            .write()
            .await
            .insert((commit.poll_id, commit.identity_secret.to_string()), ());
        Ok(rec)
    }

    async fn record_vote(&self, vote: StoredVote<'_>) -> AppResult<StoredVoteRecord> {
        {
            let seen = self.vote_nullifiers.read().await;
            if seen.contains_key(&(vote.poll_id, vote.nullifier.to_string())) {
                return Err(AppError::Validation("nullifier already used".into()));
            }
        }
        let rec = StoredVoteRecord {
            poll_id: vote.poll_id,
            nullifier: vote.nullifier.to_string(),
            recorded_at: Utc::now(),
        };
        self.votes.write().await.push(rec.clone());
        self.vote_nullifiers
            .write()
            .await
            .insert((vote.poll_id, vote.nullifier.to_string()), ());
        Ok(rec)
    }

    async fn membership_root_snapshot(&self) -> AppResult<String> {
        let members = self.members.read().await;
        Ok(hash_members(&members))
    }

    async fn list_members(&self) -> AppResult<Vec<String>> {
        Ok(self.members.read().await.clone())
    }

    async fn merkle_path_for_member(
        &self,
        _poll_id: i64,
        _identity_secret: &str,
    ) -> AppResult<Option<MerklePath>> {
        Ok(None)
    }

    async fn ensure_member(&self, identity_secret: &str) -> AppResult<()> {
        let mut members = self.members.write().await;
        if !members.contains(&identity_secret.to_string()) {
            members.push(identity_secret.to_string());
        }
        Ok(())
    }

    async fn poll_includes_member(&self, poll_id: i64, identity_secret: &str) -> AppResult<bool> {
        let pm = self.poll_members.read().await;
        if let Some(list) = pm.get(&poll_id) {
            Ok(list.contains(&identity_secret.to_string()))
        } else {
            Ok(false)
        }
    }

    async fn nullifier_used(&self, poll_id: i64, nullifier: &str) -> AppResult<bool> {
        let seen = self.vote_nullifiers.read().await;
        Ok(seen.contains_key(&(poll_id, nullifier.to_string())))
    }

    async fn has_commit(&self, poll_id: i64, identity_secret: &str) -> AppResult<bool> {
        let seen = self.commits_by_identity.read().await;
        Ok(seen.contains_key(&(poll_id, identity_secret.to_string())))
    }

    async fn commits_to_sync(
        &self,
        now: DateTime<Utc>,
        limit: i64,
    ) -> AppResult<Vec<CommitSyncRow>> {
        let polls = self.polls.read().await;
        let commits = self.commits.read().await;
        let synced = self.synced_commits.read().await;
        let mut items = Vec::new();
        for commit in commits.iter() {
            if items.len() as i64 >= limit {
                break;
            }
            if synced.contains(&commit.id) {
                continue;
            }
            if let Some(poll) = polls.get(&commit.poll_id) {
                if poll.commit_phase_end <= now && poll.reveal_phase_end > now {
                    items.push(CommitSyncRow {
                        id: commit.id,
                        poll_id: commit.poll_id,
                        choice: commit.choice,
                        commitment: commit.commitment.clone(),
                        nullifier: commit.nullifier.clone(),
                        proof: commit.proof.clone(),
                        public_inputs: commit.public_inputs.clone(),
                    });
                }
            }
        }
        Ok(items)
    }

    async fn mark_commit_synced(&self, commit_id: i64) -> AppResult<()> {
        self.synced_commits.write().await.insert(commit_id);
        Ok(())
    }

    async fn poll_has_pending_commits(&self, poll_id: i64) -> AppResult<bool> {
        let commits = self.commits.read().await;
        let synced = self.synced_commits.read().await;
        let pending = commits
            .iter()
            .any(|c| c.poll_id == poll_id && !synced.contains(&c.id));
        Ok(pending)
    }

    async fn mark_poll_sync_complete(&self, poll_id: i64) -> AppResult<()> {
        let mut polls = self.polls.write().await;
        if let Some(p) = polls.get_mut(&poll_id) {
            p.commit_sync_completed = true;
        }
        Ok(())
    }

    async fn mark_polls_without_pending_commits(&self, now: DateTime<Utc>) -> AppResult<()> {
        let commits = self.commits.read().await;
        let synced = self.synced_commits.read().await;
        let mut polls = self.polls.write().await;
        for poll in polls.values_mut() {
            if poll.commit_phase_end <= now && !poll.commit_sync_completed {
                let pending = commits
                    .iter()
                    .any(|c| c.poll_id == poll.id && !synced.contains(&c.id));
                if !pending {
                    poll.commit_sync_completed = true;
                }
            }
        }
        Ok(())
    }
}

#[async_trait]
impl PollIndexSink for InMemoryStore {
    async fn upsert_poll_from_chain(&self, poll_id: i64, poll: NewPoll<'_>) -> AppResult<()> {
        let mut polls = self.polls.write().await;
        polls.insert(
            poll_id,
            PollRecord {
                id: poll_id,
                question: poll.question.to_string(),
                options: poll.options.to_vec(),
                commit_phase_end: poll.commit_phase_end,
                reveal_phase_end: poll.reveal_phase_end,
                category: poll.category.to_string(),
                membership_root: poll.membership_root.to_string(),
                correct_option: None,
                resolved: false,
                commit_sync_completed: false,
            },
        );
        Ok(())
    }

    async fn upsert_vote_from_chain(
        &self,
        poll_id: i64,
        nullifier: &str,
        _choice: u8,
    ) -> AppResult<()> {
        self.votes.write().await.push(StoredVoteRecord {
            poll_id,
            nullifier: nullifier.to_string(),
            recorded_at: Utc::now(),
        });
        Ok(())
    }

    async fn resolve_poll_from_chain(&self, poll_id: i64, correct_option: u8) -> AppResult<()> {
        let mut polls = self.polls.write().await;
        if let Some(p) = polls.get_mut(&poll_id) {
            p.resolved = true;
            p.correct_option = Some(correct_option as i16);
        }
        Ok(())
    }
}

async fn init_schema(pool: &Pool<Postgres>) -> AppResult<()> {
    // Minimal schema for metadata + bookkeeping
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS polls (
            id BIGSERIAL PRIMARY KEY,
            question TEXT NOT NULL,
            options JSONB NOT NULL,
            commit_phase_end TIMESTAMPTZ NOT NULL,
            reveal_phase_end TIMESTAMPTZ NOT NULL,
            category TEXT NOT NULL DEFAULT 'General',
            membership_root TEXT NOT NULL,
            correct_option SMALLINT,
            resolved BOOLEAN NOT NULL DEFAULT false,
            commit_sync_completed BOOLEAN NOT NULL DEFAULT false,
            created_at TIMESTAMPTZ NOT NULL DEFAULT now()
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(AppError::Db)?;

    sqlx::query(
        r#"
        ALTER TABLE polls
        ADD COLUMN IF NOT EXISTS category TEXT NOT NULL DEFAULT 'General';
        "#,
    )
    .execute(pool)
    .await
    .map_err(AppError::Db)?;

    sqlx::query(
        r#"
        UPDATE polls
        SET category = 'General'
        WHERE category IS NULL OR category = '';
        "#,
    )
    .execute(pool)
    .await
    .map_err(AppError::Db)?;

    sqlx::query(
        r#"
        UPDATE polls
        SET commit_sync_completed = false
        WHERE commit_sync_completed IS NULL;
        "#,
    )
    .execute(pool)
    .await
    .map_err(AppError::Db)?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS members (
            id SERIAL PRIMARY KEY,
            identity_secret TEXT NOT NULL UNIQUE,
            created_at TIMESTAMPTZ NOT NULL DEFAULT now()
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(AppError::Db)?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS poll_members (
            poll_id BIGINT NOT NULL REFERENCES polls(id) ON DELETE CASCADE,
            identity_secret TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
            UNIQUE(poll_id, identity_secret)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(AppError::Db)?;

    // Safety upgrade path: ensure polls.id is BIGINT (existing DBs created before BIGSERIAL)
    sqlx::query(
        r#"
        DO $$
        BEGIN
            IF EXISTS (
                SELECT 1
                FROM information_schema.columns
                WHERE table_name = 'polls' AND column_name = 'id' AND data_type = 'integer'
            ) THEN
                ALTER TABLE polls ALTER COLUMN id TYPE BIGINT;
            END IF;
        END$$;
        "#,
    )
    .execute(pool)
    .await
    .map_err(AppError::Db)?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS commitments (
            id SERIAL PRIMARY KEY,
            poll_id BIGINT NOT NULL REFERENCES polls(id) ON DELETE CASCADE,
            commitment TEXT NOT NULL,
            identity_secret TEXT NOT NULL,
            choice SMALLINT NOT NULL DEFAULT 0,
            nullifier TEXT NOT NULL DEFAULT '',
            proof TEXT NOT NULL DEFAULT '',
            public_inputs TEXT[] NOT NULL DEFAULT '{}',
            recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
            onchain_submitted BOOLEAN NOT NULL DEFAULT false
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(AppError::Db)?;

    sqlx::query(
        r#"
        DO $$
        BEGIN
            IF NOT EXISTS (
                SELECT 1 FROM information_schema.columns
                WHERE table_name = 'commitments' AND column_name = 'id'
            ) THEN
                ALTER TABLE commitments ADD COLUMN id BIGSERIAL PRIMARY KEY;
            END IF;
        END$$;
        "#,
    )
    .execute(pool)
    .await
    .map_err(AppError::Db)?;

    sqlx::query(
        r#"
        ALTER TABLE commitments
        ADD COLUMN IF NOT EXISTS onchain_submitted BOOLEAN NOT NULL DEFAULT false;
        "#,
    )
    .execute(pool)
    .await
    .map_err(AppError::Db)?;

    sqlx::query(
        r#"
        ALTER TABLE commitments
        ADD COLUMN IF NOT EXISTS choice SMALLINT NOT NULL DEFAULT 0;
        "#,
    )
    .execute(pool)
    .await
    .map_err(AppError::Db)?;

    sqlx::query(
        r#"
        ALTER TABLE commitments
        ADD COLUMN IF NOT EXISTS nullifier TEXT NOT NULL DEFAULT '';
        "#,
    )
    .execute(pool)
    .await
    .map_err(AppError::Db)?;

    sqlx::query(
        r#"
        ALTER TABLE commitments
        ADD COLUMN IF NOT EXISTS proof TEXT NOT NULL DEFAULT '';
        "#,
    )
    .execute(pool)
    .await
    .map_err(AppError::Db)?;

    sqlx::query(
        r#"
        ALTER TABLE commitments
        ADD COLUMN IF NOT EXISTS public_inputs TEXT[] NOT NULL DEFAULT '{}';
        "#,
    )
    .execute(pool)
    .await
    .map_err(AppError::Db)?;

    sqlx::query(
        r#"
        UPDATE commitments
        SET onchain_submitted = false
        WHERE onchain_submitted IS NULL;
        "#,
    )
    .execute(pool)
    .await
    .map_err(AppError::Db)?;

    // Backfill legacy rows: set empty identity_secret to commitment to avoid dup on index creation
    sqlx::query(
        r#"
        UPDATE commitments
        SET identity_secret = commitment
        WHERE identity_secret IS NULL OR identity_secret = '';
        "#,
    )
    .execute(pool)
    .await
    .map_err(AppError::Db)?;

    // Drop duplicate (poll_id, identity_secret), keep latest recorded_at
    sqlx::query(
        r#"
        DELETE FROM commitments c
        USING (
            SELECT ctid, ROW_NUMBER() OVER (PARTITION BY poll_id, identity_secret ORDER BY recorded_at DESC, id DESC) AS rn
            FROM commitments
        ) d
        WHERE c.ctid = d.ctid AND d.rn > 1;
        "#,
    )
    .execute(pool)
    .await
    .map_err(AppError::Db)?;

    sqlx::query(
        r#"
        DROP INDEX IF EXISTS commitments_poll_commitment_idx;
        "#,
    )
    .execute(pool)
    .await
    .map_err(AppError::Db)?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS commitments_poll_commitment_idx ON commitments(poll_id, commitment)
        "#,
    )
    .execute(pool)
    .await
    .map_err(AppError::Db)?;

    sqlx::query(
        r#"
        CREATE UNIQUE INDEX IF NOT EXISTS commitments_poll_identity_idx ON commitments(poll_id, identity_secret)
        "#,
    )
    .execute(pool)
    .await
    .map_err(AppError::Db)?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS votes (
            id SERIAL PRIMARY KEY,
            poll_id BIGINT NOT NULL REFERENCES polls(id) ON DELETE CASCADE,
            nullifier TEXT NOT NULL,
            choice SMALLINT NOT NULL,
            recorded_at TIMESTAMPTZ NOT NULL DEFAULT now()
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(AppError::Db)?;

    sqlx::query(
        r#"
        CREATE UNIQUE INDEX IF NOT EXISTS votes_poll_nullifier_idx ON votes(poll_id, nullifier)
        "#,
    )
    .execute(pool)
    .await
    .map_err(AppError::Db)?;

    sqlx::query(
        r#"
        CREATE UNIQUE INDEX IF NOT EXISTS votes_nullifier_idx ON votes(poll_id, nullifier)
        "#,
    )
    .execute(pool)
    .await
    .map_err(AppError::Db)?;
    Ok(())
}
