use crate::error::{AppError, AppResult};
use crate::repo::PollRecord;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofBundle {
    pub proof: String,
    pub public_inputs: Vec<String>,
    pub commitment: String,
    pub nullifier: String,
}

#[derive(Debug, Clone, Copy)]
pub struct ProofRequest<'a> {
    pub poll_id: i64,
    pub choice: u8,
    pub secret: &'a str,
    pub identity_secret: &'a str,
    pub membership_root: &'a str,
}

#[async_trait]
pub trait ZkBackend {
    async fn prove(&self, req: ProofRequest<'_>) -> AppResult<ProofBundle>;
    async fn verify(&self, poll: &PollRecord, bundle: &ProofBundle) -> AppResult<()>;
}

/// No-op backend: hashes inputs to simulate a proof.
#[derive(Clone, Default)]
pub struct NoopZkBackend;

#[async_trait]
impl ZkBackend for NoopZkBackend {
    async fn prove(&self, req: ProofRequest<'_>) -> AppResult<ProofBundle> {
        if req.choice > 1 {
            return Err(AppError::Validation("choice must be 0 or 1".into()));
        }
        let commitment = hex_sha256(&format!("{}:{}", req.choice, req.secret));
        let nullifier = hex_sha256(&format!("{}:{}", req.identity_secret, req.poll_id));
        let proof = hex_sha256(&format!(
            "{}:{}:{}:{}",
            req.poll_id, req.membership_root, commitment, nullifier
        ));
        Ok(ProofBundle {
            proof,
            public_inputs: vec![
                req.choice.to_string(),
                commitment.clone(),
                nullifier.clone(),
            ],
            commitment,
            nullifier,
        })
    }

    async fn verify(&self, poll: &PollRecord, bundle: &ProofBundle) -> AppResult<()> {
        if poll.options.len() < 2 {
            return Err(AppError::Validation("poll options invalid".into()));
        }
        if bundle.proof.is_empty() || bundle.public_inputs.is_empty() {
            return Err(AppError::Validation("proof/public inputs empty".into()));
        }
        // In this mock backend we simply ensure the commitment/nullifier match the payload.
        if bundle.commitment != *bundle.public_inputs.get(1).unwrap_or(&"".to_string()) {
            return Err(AppError::Validation("commitment mismatch".into()));
        }
        if bundle.nullifier != *bundle.public_inputs.get(2).unwrap_or(&"".to_string()) {
            return Err(AppError::Validation("nullifier mismatch".into()));
        }
        Ok(())
    }
}

fn hex_sha256(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}
