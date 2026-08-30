//! The subset's own five mutations (T1.3).
//!
//! ```text
//! POST   /api/timeline/subsets              create, with its events
//! PUT    /api/timeline/subsets/:id          name and/or description
//! PUT    /api/timeline/subsets/:id/events   REPLACE the ordered set
//! DELETE /api/timeline/subsets/:id          SOFT delete (chronology R10)
//! POST   /api/timeline/subsets/:id/undelete the Undo
//! ```
//!
//! ## ⚑ EVERY HANDLER HERE TAKES `user: AuthUser`, NOT `Option<AuthUser>`
//!
//! That is the write guard's first half, enforced by axum's extractor: an
//! anonymous request is a 401 before a handler body runs. Reads stay open
//! because looking at a story is not privileged; writing one is. The test at the
//! foot of this file scans this directory and fails if a handler is ever
//! declared with the optional extractor.
//!
//! The second half is `services::chronology_subset_write`: every handler below
//! makes exactly ONE call into it, and that module's every function ends at the
//! seal, so no write can land without its history row.
//!
//! ## Editing rights
//!
//! All three named users are equal — enforcement is "authenticated", not
//! role-gated, the same reading the chronology's own writes take (design R2).
//!
//! ## CRITICAL — the pipeline pool
//!
//! Every table here lives in `colossus_legal_v2`.

use axum::{
    extract::{Path, State},
    Json,
};

use crate::auth::AuthUser;
use crate::dto::chronology_subset::{
    CreateSubsetRequest, SubsetDetailDto, SubsetEventRef, UpdateSubsetRequest,
};
use crate::error::AppError;
use crate::repositories::pipeline_repository::chronology_subsets::live_subset_named;
use crate::services::chronology_guard::open_write;
use crate::services::chronology_subset_validate::validate_name;
use crate::services::chronology_subset_write as subset_write;
use crate::state::AppState;

use super::support::{
    parse_subset_id, refusal, require_subset, subset_failure, subset_response, validated_events,
    write_error,
};
use crate::api::timeline::case_slug;

/// `POST /api/timeline/subsets` — create one subset and the events it arrived
/// with, in ONE transaction and ONE history row.
///
/// # Errors
/// 400 for a blank name; 409 naming the clash when a live subset in this case
/// already has that name; 422 naming the first event id that is not in this
/// case; 400 for a duplicated event or position; 503 when `CASE_SLUG` is unset;
/// 500 for a database failure.
#[tracing::instrument(skip(state, user, body), fields(by = %user.username))]
pub async fn post_subset(
    user: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<CreateSubsetRequest>,
) -> Result<Json<SubsetDetailDto>, AppError> {
    let case = case_slug(&state)?;
    let writer = open_write(&user);
    let name = judge_name(&state, &case, &body.name, None).await?;
    let description = body.description.unwrap_or_default().trim().to_string();
    // Events are judged BEFORE the transaction opens, so a bad id refuses the
    // whole create rather than leaving a named story behind with a dead pointer.
    let submitted: Vec<SubsetEventRef> = body.events.unwrap_or_default();
    let events = validated_events(&state, &case, &submitted).await?;

    let written = subset_write::create(
        &state.pipeline_pool,
        &case,
        &name,
        &description,
        &events,
        &writer,
    )
    .await
    .map_err(|e| write_error(e, "creating a subset"))?;

    tracing::info!(
        subset_id = %written.subset.id,
        events = written.events.len(),
        by = %writer.by,
        "timeline: a subset was created"
    );
    Ok(Json(subset_response(&state, &written.subset).await?))
}

/// `PUT /api/timeline/subsets/:id` — the name and/or the description.
///
/// An absent field means "leave it alone"; sending neither is a legal no-op that
/// still lands a history row, because somebody pressed Save and the record is of
/// acts rather than of diffs.
///
/// # Errors
/// 400 when the id is not a UUID or the supplied name is blank; 404 when there
/// is no such subset; 409 for a name clash or for a deleted subset (restore it
/// with Undo first — a different answer from "gone", because it is one press
/// away); 500 for a database failure.
#[tracing::instrument(skip(state, user, body), fields(by = %user.username, subset_id = %id))]
pub async fn put_subset(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateSubsetRequest>,
) -> Result<Json<SubsetDetailDto>, AppError> {
    let subset_id = parse_subset_id(&id)?;
    let writer = open_write(&user);
    let existing = require_subset(&state, subset_id, "the subset being edited").await?;

    let name = match body.name.as_deref() {
        Some(supplied) => {
            Some(judge_name(&state, &existing.case_slug, supplied, Some(subset_id)).await?)
        }
        None => None,
    };
    let description = body.description.map(|d| d.trim().to_string());

    let written = subset_write::rename(
        &state.pipeline_pool,
        subset_id,
        name.as_deref(),
        description.as_deref(),
        &writer,
    )
    .await
    .map_err(|e| write_error(e, "editing a subset"))?;

    tracing::info!(subset_id = %written.subset.id, by = %writer.by, "timeline: a subset was edited");
    Ok(Json(subset_response(&state, &written.subset).await?))
}

/// `PUT /api/timeline/subsets/:id/events` — REPLACE the ordered event set.
///
/// This is the picker's Save. There is no per-row add/remove endpoint: one human
/// act is one write and one snapshot, and modelling a minute of ticking and
/// dragging as a stream of small writes would put a dozen history rows behind it.
///
/// # Errors
/// 400 when the id is not a UUID, or an event or a position is listed twice; 404
/// when there is no such subset; 409 when it is deleted; 422 naming the first
/// event id that is not in this case; 500 for a database failure.
#[tracing::instrument(skip(state, user, body), fields(by = %user.username, subset_id = %id))]
pub async fn put_subset_events(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<Vec<SubsetEventRef>>,
) -> Result<Json<SubsetDetailDto>, AppError> {
    let subset_id = parse_subset_id(&id)?;
    let writer = open_write(&user);
    let existing =
        require_subset(&state, subset_id, "the subset whose events are being set").await?;
    let events = validated_events(&state, &existing.case_slug, &body).await?;

    let written = subset_write::replace_events(&state.pipeline_pool, subset_id, &events, &writer)
        .await
        .map_err(|e| write_error(e, "replacing a subset's events"))?;

    tracing::info!(
        subset_id = %written.subset.id,
        events = written.events.len(),
        by = %writer.by,
        "timeline: a subset's events were replaced"
    );
    Ok(Json(subset_response(&state, &written.subset).await?))
}

/// `DELETE /api/timeline/subsets/:id` — SOFT delete (chronology R10).
///
/// Nothing is removed and there is NO confirm dialog: the undo line that
/// replaces the row in place is the safety. Detaches nothing — the scenario link
/// rows stay, and the scenario reads simply do not see a deleted subset, so an
/// Undo brings the attachment back with it.
///
/// # Errors
/// 400 for a malformed id; 404 when there is no such subset. Deleting an
/// already-deleted subset is a success that changes nothing and writes no
/// history — see below.
#[tracing::instrument(skip(state, user), fields(by = %user.username, subset_id = %id))]
pub async fn delete_subset(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<SubsetDetailDto>, AppError> {
    let subset_id = parse_subset_id(&id)?;
    let writer = open_write(&user);
    let existing = require_subset(&state, subset_id, "the subset being deleted").await?;
    if existing.deleted_at.is_some() {
        // Already deleted. Answering 200 with the row is right — the caller's
        // intent holds — but sealing would append a second `deleted` history row
        // for a delete that did not happen, and history is a record of acts, not
        // of requests.
        tracing::info!(%subset_id, "timeline: delete of an already-deleted subset; no history written");
        return Ok(Json(subset_response(&state, &existing).await?));
    }

    let written = subset_write::soft_delete(&state.pipeline_pool, subset_id, &writer)
        .await
        .map_err(|e| write_error(e, "deleting a subset"))?;
    let Some(written) = written else {
        // Lost the race against another delete: the check above said live, the
        // UPDATE inside the transaction found it already gone. The other
        // request's history row is the true record of the act, so this one
        // writes none and still answers 200 — the caller's intent holds either
        // way. `existing` is stale now, hence the re-read.
        tracing::info!(%subset_id, "timeline: delete raced another delete; no history written");
        let current = require_subset(&state, subset_id, "the subset being deleted").await?;
        return Ok(Json(subset_response(&state, &current).await?));
    };
    tracing::info!(subset_id = %written.subset.id, by = %writer.by, "timeline: a subset was deleted (soft)");
    Ok(Json(subset_response(&state, &written.subset).await?))
}

/// `POST /api/timeline/subsets/:id/undelete` — the Undo.
///
/// # Errors
/// 400 for a malformed id; 404 when there is no such subset. Undoing a live
/// subset is a success that changes nothing and writes no history, for the same
/// reason a second delete does not.
#[tracing::instrument(skip(state, user), fields(by = %user.username, subset_id = %id))]
pub async fn post_undelete(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<SubsetDetailDto>, AppError> {
    let subset_id = parse_subset_id(&id)?;
    let writer = open_write(&user);
    let existing = require_subset(&state, subset_id, "the subset being restored").await?;
    if existing.deleted_at.is_none() {
        tracing::info!(%subset_id, "timeline: undo of a live subset; no history written");
        return Ok(Json(subset_response(&state, &existing).await?));
    }

    let written = subset_write::restore(&state.pipeline_pool, subset_id, &writer)
        .await
        .map_err(|e| write_error(e, "restoring a subset"))?;
    let Some(written) = written else {
        // Somebody else's Undo landed first. Same reasoning as the delete race
        // above: their `restored` row is the record, ours would be a duplicate
        // of an act that happened once.
        tracing::info!(%subset_id, "timeline: undo raced another undo; no history written");
        let current = require_subset(&state, subset_id, "the subset being restored").await?;
        return Ok(Json(subset_response(&state, &current).await?));
    };
    tracing::info!(subset_id = %written.subset.id, by = %writer.by, "timeline: a subset was restored");
    Ok(Json(subset_response(&state, &written.subset).await?))
}

/// Judge a supplied name: non-blank, and not already a live subset's in this case.
///
/// Extracted because two handlers ask the same question and a second copy would
/// be the one that eventually forgot `exclude` — which is how renaming a subset
/// to its own name becomes a 409 against itself.
async fn judge_name(
    state: &AppState,
    case_slug: &str,
    supplied: &str,
    exclude: Option<uuid::Uuid>,
) -> Result<String, AppError> {
    let trimmed = supplied.trim();
    let clash = if trimmed.is_empty() {
        // Nothing to look up, and looking it up would ask the database about the
        // empty string. The blank refusal below is the honest answer.
        false
    } else {
        live_subset_named(&state.pipeline_pool, case_slug, trimmed, exclude)
            .await
            .map_err(|e| subset_failure(e, "whether this subset name is already taken"))?
            .is_some()
    };
    validate_name(supplied, clash).map_err(refusal)
}

#[cfg(test)]
#[path = "writes_tests.rs"]
mod tests;
