//! `GET /api/cases/:slug/case-health/inventory` — Pane 1 of the Case Health
//! dashboard ("Graph Inventory"). Neo4j only, read-only, no Postgres.
//!
//! The handler is deliberately thin, following `proof_matrix.rs`: run the four
//! reads via [`crate::repositories::case_health_repository`], hand the rows to
//! the pure [`crate::repositories::case_health_builder`], return JSON. All
//! arithmetic lives in the builder; all Cypher lives in the repository; this
//! file owns only the HTTP surface and the error mapping.
//!
//! ## Why this pane exists
//!
//! Eight documents were processed and judged on grounding rate while the metric
//! that actually matters — how much of the extracted Evidence is wired into an
//! Allegation — was displayed nowhere and had to be hand-queried. This endpoint
//! makes that class of blindness structurally impossible: the connection rate is
//! now a page, live, and it cannot be forgotten because it is the headline.
//!
//! ## Why there is no 404
//!
//! Unlike `proof_matrix.rs`, an empty result is NOT an error here. A graph with
//! no Evidence is a perfectly legitimate reading — it is, in fact, the reading a
//! brand-new case starts from — and the honest response is a payload of zeros
//! with `null` rates, not a 404 telling the operator something is broken. The
//! only failure mode this endpoint has is a backend one.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Utc;
use serde::Serialize;
use tracing::{error, info, instrument};

use crate::auth::AuthUser;
use crate::domain::connection_tier::CONNECTION_TIER_LOOKUP_V;
use crate::dto::case_health::CaseHealthInventoryResponse;
use crate::repositories::case_health_builder as builder;
use crate::repositories::case_health_repository as repo;
use crate::state::AppState;

/// Error type for this endpoint.
///
/// ## Why a dedicated error instead of the shared `AppError`
///
/// Matching `proof_matrix.rs` (its nearest neighbour), this read uses its own
/// error so the 500 body stays deliberately bland — Cypher and decode detail are
/// logged for operators, never returned to the client (Standing Rule 1). The
/// shared `AppError` body (`{"error","message","details"}`) would echo a
/// `message`, so it is intentionally not used here.
///
/// One variant only, because one thing can go wrong: the graph read. There is no
/// not-found state (see the module note) and no client input to reject — the
/// slug is not validated against a case because the Neo4j graph is single-case
/// and not slug-namespaced, exactly as in `proof_matrix.rs`.
pub enum CaseHealthEndpointError {
    /// Graph error or row-decode failure → HTTP 500 (logged, not returned).
    Internal,
}

/// 500 body: exactly `{"error":"internal server error"}` — nothing else.
#[derive(Serialize)]
struct InternalErrorBody {
    error: &'static str,
}

impl IntoResponse for CaseHealthEndpointError {
    fn into_response(self) -> Response {
        match self {
            CaseHealthEndpointError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(InternalErrorBody {
                    error: "internal server error",
                }),
            )
                .into_response(),
        }
    }
}

/// `GET /api/cases/:slug/case-health/inventory`. No auth required, matching the
/// other case-scoped reads; the user is logged when present.
///
/// ## Rust Learning: `Option<AuthUser>` as an extractor
///
/// `AuthUser` is an axum extractor that fails the request if the identity headers
/// are absent. Wrapping it in `Option` makes extraction infallible: `Some(user)`
/// when authenticated, `None` otherwise. This matches the sibling case-scoped
/// reads, where auth is observed-and-logged but not enforced at the handler
/// (enforcement is upstream at Traefik/Authentik for UI routes).
///
/// ## Rust Learning: `tokio::try_join!` for concurrent fallible reads
///
/// The four reads are independent, so running them sequentially would pay four
/// round-trips end to end. `try_join!` polls all four futures on the same task
/// and returns as soon as they all succeed — or short-circuits on the FIRST
/// error, which is then logged with its operation name by the shared mapper
/// below. Each future borrows `&state.graph` immutably, and neo4rs' `Graph` is a
/// cheap connection-pool handle, so concurrent use is exactly what it is for.
/// (`try_join!`, not `join!`: we want the first failure to abort, not four
/// separate `Result`s to unwrap afterwards.)
#[instrument(skip(state, user), fields(slug = %slug))]
pub async fn get_case_health_inventory(
    user: Option<AuthUser>,
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<CaseHealthInventoryResponse>, CaseHealthEndpointError> {
    // Log unconditionally so an anonymous caller still leaves an operator-visible
    // trail; the anonymous fallback keeps `username` present in every span so log
    // queries can group on it without special-casing a missing value.
    let username = user
        .as_ref()
        .map(|u| u.username.as_str())
        .unwrap_or("<anonymous>");
    info!(
        username = username,
        "GET /api/cases/{slug}/case-health/inventory"
    );

    let (label_rows, edge_rows, corpus_row, document_rows) = tokio::try_join!(
        repo::fetch_label_counts(&state.graph),
        repo::fetch_edge_triples(&state.graph),
        repo::fetch_corpus(&state.graph),
        repo::fetch_document_rows(&state.graph),
    )
    .map_err(internal("case-health inventory read"))?;

    // The measurement's timestamp is stamped once, here, and shared by every
    // figure in the payload — so the four reads present as one reading rather
    // than four slightly-different moments.
    let computed_at = Utc::now().to_rfc3339();

    let current = builder::build_inventory(
        computed_at,
        label_rows,
        edge_rows,
        corpus_row,
        document_rows,
    );

    info!(
        evidence_total = current.corpus.evidence_total,
        probative_connected = current.corpus.probative_connected,
        topical_connected = current.corpus.topical_connected,
        documents = current.documents.len(),
        unlabeled_nodes = current.unlabeled_node_count,
        "case-health inventory composed"
    );

    Ok(Json(CaseHealthInventoryResponse {
        case_slug: slug,
        connection_tier_lookup_v: CONNECTION_TIER_LOOKUP_V,
        edge_classes: builder::edge_class_names(),
        current,
        // No history store exists yet (out of scope for this chunk); the field is
        // present in the shape so a snapshot attaches later without a breaking
        // change. See `dto::case_health`.
        previous: None,
    }))
}

/// Map any displayable error to a logged `Internal` (500). `op` names the failed
/// step for the operator log; the client only ever sees the bland 500 body.
///
/// ## Rust Learning: returning `impl Fn(E) -> _` for `.map_err`
///
/// This returns a closure capturing `op`, so the call site reads
/// `.map_err(internal("case-health inventory read"))?`. The closure logs with
/// full context — the repository error's `Display` already names WHICH of the
/// four reads failed and carries the underlying neo4rs cause via `#[source]` —
/// and collapses every failure into the single opaque `Internal` variant. This
/// is the place the `?` chain terminates at a handler that logs (Standing
/// Rule 1).
fn internal<E: std::fmt::Display>(op: &'static str) -> impl Fn(E) -> CaseHealthEndpointError {
    move |e| {
        error!(error = %e, operation = op, "case-health inventory request failed");
        CaseHealthEndpointError::Internal
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The 500 body must never carry Cypher, an error message, or a `details`
    /// object — exactly one key, `error`. Guards Standing Rule 1's "no leak to
    /// the client" requirement at the serialization boundary.
    #[test]
    fn internal_server_error_body_does_not_leak_detail() {
        let body = InternalErrorBody {
            error: "internal server error",
        };
        let value = serde_json::to_value(&body).expect("body serializes");
        assert_eq!(value, json!({"error": "internal server error"}));
        assert_eq!(value.as_object().expect("object body").len(), 1);
    }

    /// `Internal` must map to HTTP 500. The body-shape test above cannot catch a
    /// regression that swapped the status code in the `IntoResponse` match arm.
    #[test]
    fn internal_variant_maps_to_500() {
        let response = CaseHealthEndpointError::Internal.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    /// The shipped column list is the tier vocabulary, and the shipped version
    /// stamp is the one this build computed under. Pins the two payload fields
    /// the handler contributes on top of the builder's output.
    #[test]
    fn handler_ships_the_tier_vocabulary_and_its_version() {
        assert_eq!(CONNECTION_TIER_LOOKUP_V, 1);
        assert_eq!(
            builder::edge_class_names(),
            vec!["CORROBORATES", "REBUTS", "CHARACTERIZES", "ABOUT"]
        );
    }
}
