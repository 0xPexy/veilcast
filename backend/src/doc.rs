//! OpenAPI / Swagger documentation definitions.
use crate::zk::ProofBundle;
use crate::{
    CommitRequest, CommitResponse, CreatePollRequest, PollResponse, ProveRequest, RevealRequest,
    RevealResponse,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        health_doc,
        create_poll_doc,
        list_polls_doc,
        get_poll_doc,
        record_commit_doc,
        generate_proof_doc,
        reveal_vote_doc
    ),
    components(
        schemas(
            CreatePollRequest,
            PollResponse,
            CommitRequest,
            CommitResponse,
            ProveRequest,
            RevealRequest,
            RevealResponse,
            ProofBundle
        )
    ),
    tags(
        (name = "veilcast", description = "VeilCast poll API")
    )
)]
pub struct ApiDoc;

// Doc-only shim functions so utoipa can pick up signatures.
#[utoipa::path(
    get,
    path = "/health",
    responses((status = 200, description = "OK"))
)]
pub async fn health_doc() {}

#[utoipa::path(
    post,
    path = "/polls",
    request_body = CreatePollRequest,
    responses((status = 200, body = PollResponse))
)]
pub async fn create_poll_doc() {}

#[utoipa::path(
    get,
    path = "/polls",
    responses((status = 200, body = [PollResponse]))
)]
pub async fn list_polls_doc() {}

#[utoipa::path(
    get,
    path = "/polls/{id}",
    params(
        ("id" = i64, Path, description = "Poll id")
    ),
    responses((status = 200, body = PollResponse))
)]
pub async fn get_poll_doc() {}

#[utoipa::path(
    post,
    path = "/polls/{id}/commit",
    params(("id" = i64, Path, description = "Poll id")),
    request_body = CommitRequest,
    responses((status = 200, body = CommitResponse))
)]
pub async fn record_commit_doc() {}

#[utoipa::path(
    post,
    path = "/polls/{id}/prove",
    params(("id" = i64, Path, description = "Poll id")),
    request_body = ProveRequest,
    responses((status = 200, body = ProofBundle))
)]
pub async fn generate_proof_doc() {}

#[utoipa::path(
    post,
    path = "/polls/{id}/reveal",
    params(("id" = i64, Path, description = "Poll id")),
    request_body = RevealRequest,
    responses((status = 200, body = RevealResponse))
)]
pub async fn reveal_vote_doc() {}
