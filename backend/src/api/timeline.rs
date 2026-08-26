//! The chronology's READ endpoints (task A3).
//!
//! `GET /api/timeline`                  — phases + events + links + note counts
//! `GET /api/timeline/events/:id`       — one event in full, with notes and history
//!
//! Both are OPEN reads (`Option<AuthUser>`), matching the sibling scenario reads:
//! looking at the chronology is not privileged. The WRITE endpoints are Phase C
//! and nothing here writes anything — `chronology_write` is deliberately not
//! imported by this module.
//!
//! ## CRITICAL — the pipeline pool
//!
//! Every chronology table lives in `colossus_legal_v2`, so every read uses
//! `&state.pipeline_pool`, NOT `state.pg_pool`.
//!
//! ## Which case?
//!
//! From `CASE_SLUG`, the same configuration the rest of the backend reads. When
//! it is unset the endpoint answers 503 naming the variable, rather than
//! inventing a case or returning an empty chronology — an empty timeline and an
//! unconfigured deployment must not look the same (Standing Rule 1).

use std::collections::{HashMap, HashSet};

use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::dto::chronology::{TimelineDto, TimelineEventDetailDto};
use crate::error::AppError;
use crate::repositories::pipeline_repository::chronology::{
    get_event, list_events, list_phases, list_tags, ChronologyLinkRow,
};
use crate::repositories::pipeline_repository::chronology_links::{
    existing_document_ids, list_history_for_event, list_links_for_case, list_links_for_event,
    list_notes_for_event, note_counts_for_case,
};
use crate::repositories::pipeline_repository::PipelineRepoError;
use crate::services::chronology_read::{
    build_event_detail, build_timeline, TimelineSources, CHECKABLE_TARGET_TYPE,
};
use crate::state::AppState;

/// The chronology's routes, merged into the API router by `api::router`.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/timeline", get(get_timeline))
        .route("/timeline/events/:id", get(get_timeline_event))
}

/// What the `case_slug` field says when a read does not belong to one case.
///
/// ## Rust Learning: a named constant so a span and its events cannot disagree
///
/// This value is written in TWO places — the span field and the error event —
/// and they are joined by name in any structured trace processor. Two literals
/// would be two copies of one string, and the day someone edited one, a trace
/// would carry a span saying one thing and its own error saying another. One
/// constant makes that impossible rather than merely unlikely.
const NOT_CASE_SCOPED: &str = "(not case-scoped)";

/// Turn a repository failure into a 500 that names the case, the operation and
/// the cause.
///
/// Every `?` in this module terminates here, so a reader of the logs can always
/// tell WHICH read failed and why — the alternative, one generic "database
/// error", is the silent failure Standing Rule 1 forbids.
///
/// ## Why the case is an `Option` and not a `&str`
///
/// The list read resolves a case before it does anything, so it always has one.
/// The EVENT read does not need one — it looks an event up by its own id — and
/// forcing it to resolve `CASE_SLUG` just to log would turn an unconfigured
/// deployment into a 503 on a read that would otherwise have worked. `None` is
/// the honest field value for "this read did not depend on a case".
fn read_failure(error: PipelineRepoError, what: &str, case: Option<&str>) -> AppError {
    tracing::error!(
        case_slug = case.unwrap_or(NOT_CASE_SCOPED),
        operation = %what,
        error = %error,
        "chronology read failed"
    );
    AppError::Internal {
        message: format!("the chronology could not be read ({what})"),
    }
}

/// The configured case slug, or a 503 naming the variable that is missing.
fn case_slug(state: &AppState) -> Result<String, AppError> {
    state.config.case_slug.clone().ok_or_else(|| {
        tracing::error!("CASE_SLUG is unset; the chronology cannot know which case to read");
        AppError::ServiceUnavailable {
            message: "CASE_SLUG is not configured on this deployment, so the chronology \
                      cannot be read. Set it and restart the backend."
                .to_string(),
        }
    })
}

/// Every document id the given links point at, so resolution is ONE query.
///
/// ## Rust Learning: collecting borrowed strs into owned Strings for a query
///
/// The repository takes `&[String]` because sqlx binds an owned `text[]`. The
/// links own their ids already, so this clones only the ones that are actually
/// checkable — a link to a target type this build cannot check is never asked
/// about, because the answer would be meaningless.
fn checkable_target_ids(links: &[ChronologyLinkRow]) -> Vec<String> {
    let mut ids: Vec<String> = links
        .iter()
        .filter(|l| l.target_type == CHECKABLE_TARGET_TYPE)
        .map(|l| l.target_id.clone())
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

/// Log every degradation the composition reported, one line each.
fn log_warnings(warnings: &[String], surface: &str) {
    for warning in warnings {
        tracing::warn!(surface = %surface, "{warning}");
    }
}

/// `GET /api/timeline` — the whole chronology in one read.
///
/// Four queries, not four-per-event: phases, events, every link for the case,
/// and the note counts. The links are then resolved against `documents` in one
/// more query. At the design's volume (100–200 events) this is five round trips
/// for the entire page.
// `case_slug` is Empty at entry and recorded below, because the value comes
// from configuration rather than from an argument the attribute could name.
// Same shape as `api::proof_review`'s `document_id`.
#[tracing::instrument(skip(state, user), fields(case_slug = tracing::field::Empty))]
pub async fn get_timeline(
    user: Option<AuthUser>,
    State(state): State<AppState>,
) -> Result<Json<TimelineDto>, AppError> {
    if let Some(ref u) = user {
        tracing::info!("{} GET /timeline", u.username);
    }
    let case = case_slug(&state)?;
    tracing::Span::current().record("case_slug", tracing::field::display(&case));

    let phases = list_phases(&state.pipeline_pool)
        .await
        .map_err(|e| read_failure(e, "the phase list", Some(&case)))?;
    // main's tag read (Phase B) keeps its place; this branch's third argument —
    // the case slug — is applied to it as to every other read on this handler,
    // so a tag-vocabulary failure names the case like the rest of them.
    let tags = list_tags(&state.pipeline_pool)
        .await
        .map_err(|e| read_failure(e, "the tag vocabulary", Some(&case)))?;
    let events = list_events(&state.pipeline_pool, &case)
        .await
        .map_err(|e| read_failure(e, "the event list", Some(&case)))?;
    let links = list_links_for_case(&state.pipeline_pool, &case)
        .await
        .map_err(|e| read_failure(e, "this case's event links", Some(&case)))?;
    let counts = note_counts_for_case(&state.pipeline_pool, &case)
        .await
        .map_err(|e| read_failure(e, "the per-event note counts", Some(&case)))?;

    let resolved = resolve_targets(&state, &links).await?;
    let note_counts: HashMap<Uuid, i64> = counts.into_iter().collect();

    // ONE snapshot read, held for the whole composition: the words and the
    // window size must describe the same configuration as each other.
    let settings = state.settings.current();
    let composed = build_timeline(TimelineSources {
        phases: &phases,
        tags: &tags,
        events: &events,
        links: &links,
        note_counts: &note_counts,
        resolved_documents: &resolved,
        wording: &settings.chronology_wording,
        phase_window_events: settings.chronology_phase_window_events,
    });
    log_warnings(&composed.warnings, "/timeline");
    tracing::info!(
        phases = composed.payload.phases.len(),
        tags = composed.payload.tags.len(),
        events = composed.payload.events.len(),
        links = links.len(),
        "chronology read"
    );
    Ok(Json(composed.payload))
}

/// `GET /api/timeline/events/:id` — one event, with notes and history.
#[tracing::instrument(skip(state, user), fields(event_id = %id, case_slug = tracing::field::Empty))]
pub async fn get_timeline_event(
    user: Option<AuthUser>,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<TimelineEventDetailDto>, AppError> {
    if let Some(ref u) = user {
        tracing::info!("{} GET /timeline/events/{}", u.username, id);
    }
    // Recorded UNCONDITIONALLY, with the same sentinel the error event uses.
    // Recording it only when a case is configured left the span field Empty
    // while every error event on the same trace carried the sentinel — one field
    // name, two values, on one trace. This read does not depend on a case (it
    // looks an event up by its own id), so "not scoped" is the honest value
    // rather than a reason to resolve CASE_SLUG and risk a 503.
    tracing::Span::current().record(
        "case_slug",
        tracing::field::display(state.config.case_slug.as_deref().unwrap_or(NOT_CASE_SCOPED)),
    );
    let event_id = Uuid::parse_str(&id).map_err(|_| AppError::BadRequest {
        message: format!("'{id}' is not a chronology event id"),
        details: serde_json::json!({ "expected": "a UUID" }),
    })?;

    let event = get_event(&state.pipeline_pool, event_id)
        .await
        .map_err(|e| read_failure(e, "the event", state.config.case_slug.as_deref()))?
        .ok_or_else(|| AppError::NotFound {
            message: format!("no chronology event {event_id}"),
        })?;

    let links = list_links_for_event(&state.pipeline_pool, event_id)
        .await
        .map_err(|e| read_failure(e, "this event's links", state.config.case_slug.as_deref()))?;
    let notes = list_notes_for_event(&state.pipeline_pool, event_id)
        .await
        .map_err(|e| read_failure(e, "this event's notes", state.config.case_slug.as_deref()))?;
    let history = list_history_for_event(&state.pipeline_pool, event_id)
        .await
        .map_err(|e| read_failure(e, "this event's history", state.config.case_slug.as_deref()))?;

    let resolved = resolve_targets(&state, &links).await?;
    let composed = build_event_detail(&event, &links, &notes, &history, &resolved);
    log_warnings(&composed.warnings, "/timeline/events/:id");
    Ok(Json(composed.payload))
}

/// Which of these links' document targets actually exist.
async fn resolve_targets(
    state: &AppState,
    links: &[ChronologyLinkRow],
) -> Result<HashSet<String>, AppError> {
    let ids = checkable_target_ids(links);
    existing_document_ids(&state.pipeline_pool, &ids)
        .await
        .map_err(|e| {
            read_failure(
                e,
                "which linked documents exist",
                state.config.case_slug.as_deref(),
            )
        })
}

#[cfg(test)]
#[path = "timeline_tests.rs"]
mod tests;
