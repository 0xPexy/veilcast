use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Commit,
    Reveal,
    Resolved,
}

impl Phase {
    pub fn from_times(
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
pub struct CreatePollRequest {
    pub question: String,
    pub options: Vec<String>,
    pub commit_phase_end: DateTime<Utc>,
    pub reveal_phase_end: DateTime<Utc>,
    pub membership_root: String,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PollResponse {
    pub id: i64,
    pub question: String,
    pub options: Vec<String>,
    pub commit_phase_end: DateTime<Utc>,
    pub reveal_phase_end: DateTime<Utc>,
    pub membership_root: String,
    pub correct_option: Option<i16>,
    pub resolved: bool,
    pub phase: Phase,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CommitRequest {
    pub commitment: String,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CommitResponse {
    pub poll_id: i64,
    pub commitment: String,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProveRequest {
    pub choice: u8,
    pub secret: String,
    pub identity_secret: String,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RevealRequest {
    pub proof: String,
    pub public_inputs: Vec<String>,
    pub commitment: String,
    pub nullifier: String,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RevealResponse {
    pub poll_id: i64,
    pub nullifier: String,
    pub recorded_at: DateTime<Utc>,
}
