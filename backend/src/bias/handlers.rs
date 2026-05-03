//! Bias Explorer — Axum HTTP handlers.
//!
//! Two endpoints, both registered on the non-admin authenticated route
//! group:
//!
//! - `GET  /api/bias/available-filters` — dropdown contents
//! - `POST /api/bias/query`             — filtered Evidence list
//!
//! The shape of these handlers mirrors `api::persons` (the canonical
//! example of a non-admin Neo4j-backed endpoint): take `Option<AuthUser>`
//! for audit logging, accept `State<AppState>` for the graph handle, run
//! the repository, map errors to a 500 with a logged cause.

use axum::{extract::State, http::StatusCode, Json};

use crate::auth::AuthUser;
use crate::state::AppState;

use super::dto::{AvailableFilters, BiasQueryFilters, BiasQueryResult};
use super::repository::{BiasRepository, BiasRepositoryError};

/// `GET /api/bias/available-filters`
///
/// Returns the actor list, pattern-tag list, subject list, and the
/// server-resolved default-subject id used to populate the Bias Explorer
/// dropdowns. Called once on page mount.
///
/// The default-subject id is resolved here from `CASE_DEFAULT_SUBJECT_NAME`
/// (an Ansible-managed env var) rather than letting the frontend match
/// names — this keeps case-specific data (the plaintiff's name) out of
/// the JS bundle (Standing Rule 2).
///
/// The endpoint is intentionally safe to call by any authenticated user —
/// it only reads names and counts, not statement content. An anonymous
/// caller (no `AuthUser`) reaches this handler when Authentik forwards no
/// identity headers; we still serve the data because the route lives
/// behind the same Traefik gate as the rest of the case-data API.
pub async fn get_available_filters(
    user: Option<AuthUser>,
    State(state): State<AppState>,
) -> Result<Json<AvailableFilters>, StatusCode> {
    if let Some(ref u) = user {
        tracing::info!("{} GET /api/bias/available-filters", u.username);
    }

    let repo = BiasRepository::new(state.graph.clone());
    let default_name = state.config.case_default_subject_name.as_deref();

    match repo.available_filters(default_name).await {
        Ok(filters) => Ok(Json(filters)),
        Err(e) => {
            // ## Rust Learning: structured tracing fields
            // `error = ?e` uses Debug; the `BiasRepositoryError` Debug impl
            // walks down the source chain, so we get the full underlying
            // cause (network error, syntax error, etc.) without having to
            // enumerate each variant here.
            tracing::error!(
                operation = "bias.available_filters",
                error = ?e,
                "Failed to fetch available bias filters"
            );
            Err(map_error(&e))
        }
    }
}

/// `POST /api/bias/query`
///
/// Runs the structured bias query and returns matching Evidence
/// instances. The request body is a `BiasQueryFilters` JSON object;
/// every field is optional, and an empty body `{}` returns the
/// unfiltered result.
pub async fn post_bias_query(
    user: Option<AuthUser>,
    State(state): State<AppState>,
    Json(filters): Json<BiasQueryFilters>,
) -> Result<Json<BiasQueryResult>, StatusCode> {
    if let Some(ref u) = user {
        tracing::info!(
            actor_id = ?filters.actor_id,
            pattern_tag = ?filters.pattern_tag,
            subject_id = ?filters.subject_id,
            "{} POST /api/bias/query",
            u.username
        );
    }

    let repo = BiasRepository::new(state.graph.clone());

    match repo.run_query(&filters).await {
        Ok((total_count, total_unfiltered, instances)) => Ok(Json(BiasQueryResult {
            total_count,
            total_unfiltered,
            instances,
            applied_filters: filters,
        })),
        Err(e) => {
            tracing::error!(
                operation = "bias.run_query",
                actor_id = ?filters.actor_id,
                pattern_tag = ?filters.pattern_tag,
                subject_id = ?filters.subject_id,
                error = ?e,
                "Failed to run bias query"
            );
            Err(map_error(&e))
        }
    }
}

/// Map a repository error to an HTTP status code.
///
/// All bias-repository errors are server-side (Neo4j down, schema drift,
/// driver bug), so they collapse to 500. We preserve the distinction in
/// logs via the typed `BiasRepositoryError` variants — the wire response
/// is opaque to callers either way.
fn map_error(_err: &BiasRepositoryError) -> StatusCode {
    StatusCode::INTERNAL_SERVER_ERROR
}
