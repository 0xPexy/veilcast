#![allow(dead_code)]
//! OpenAPI / Swagger documentation definitions.
use crate::zk::ProofBundle;
use crate::types::{
    CommitRequest, CommitResponse, CommitStatusResponse, CreatePollRequest, LoginRequest, LoginResponse, MeResponse,
    MembershipStatusResponse, PollResponse, ProveRequest, RevealRequest, RevealResponse,
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
        reveal_vote_doc,
        membership_status_doc,
        commit_status_doc,
        login_doc,
        me_doc
    ),
    components(
        schemas(
            CreatePollRequest,
            PollResponse,
            CommitRequest,
            CommitResponse,
            CommitStatusResponse,
            ProveRequest,
            RevealRequest,
            RevealResponse,
            ProofBundle,
            LoginRequest,
            LoginResponse,
            MeResponse,
            MembershipStatusResponse
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

#[utoipa::path(
    get,
    path = "/polls/{id}/membership",
    params(("id" = i64, Path, description = "Poll id")),
    responses((status = 200, body = MembershipStatusResponse))
)]
pub async fn membership_status_doc() {}

#[utoipa::path(
    get,
    path = "/polls/{id}/commit_status",
    params(("id" = i64, Path, description = "Poll id")),
    responses((status = 200, body = CommitStatusResponse))
)]
pub async fn commit_status_doc() {}

#[utoipa::path(
    post,
    path = "/auth/login",
    request_body = LoginRequest,
    responses((status = 200, body = LoginResponse))
)]
pub async fn login_doc() {}

#[utoipa::path(
    get,
    path = "/auth/me",
    responses((status = 200, body = MeResponse))
)]
pub async fn me_doc() {}
