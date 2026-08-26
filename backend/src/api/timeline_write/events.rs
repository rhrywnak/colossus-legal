//! Create, edit, soft-delete and restore one chronology event (Phase C, §C1).
//!
//! ```text
//! POST   /api/timeline/events              create
//! PUT    /api/timeline/events/:id          edit the same fields
//! DELETE /api/timeline/events/:id          SOFT delete (design R10)
//! POST   /api/timeline/events/:id/undelete the Undo
//! ```
//!
//! Links and notes are [`super::links`]'s; the shared mapping, resolution and
//! response composition are [`super::support`]'s. The directory's own header
//! carries the guard's summary.
//!
//! ## ⚑ EVERY HANDLER HERE TAKES `user: AuthUser`, NOT `Option<AuthUser>`
//!
//! That is the write guard's first half, and it is enforced by axum's extractor:
//! an anonymous request is a 401 before a handler body runs. Reads stay open
//! (Phase A) because looking at the chronology is not privileged; writing to it
//! is. `events_tests` scans this file and its sibling and fails if a handler is
//! ever declared with the optional extractor.
//!
//! The second half is `services::chronology_guard`: `open_write` stamps the
//! acting user, and `seal_and_commit` is the only way any transaction below is
//! committed, so no write can land without its history row.
//!
//! ## Editing rights (design R2)
//!
//! All three named users are equal — enforcement is "authenticated", not
//! role-gated, in v1. Adding `require_edit(&user)?` to `open_write` is the one
//! line that would gate this on `legal_editor` if Roman later rules that way.
//!
//! ## CRITICAL — the pipeline pool
//!
//! Every table here lives in `colossus_legal_v2`.

use axum::{
    extract::{Path, State},
    Json,
};

use crate::auth::AuthUser;
use crate::dto::chronology::TimelineEventDetailDto;
use crate::dto::chronology_write::{CreateEventRequest, UpdateEventRequest};
use crate::error::AppError;
use crate::repositories::pipeline_repository::chronology::{list_phases, list_tags};
use crate::repositories::pipeline_repository::chronology_write::{
    insert_event, insert_link, soft_delete_event, undelete_event, update_event,
    ChronologyEventStateRow, EventEdit, NewChronologyEvent, NewChronologyLink,
};
use crate::services::chronology_guard::{open_write, seal_and_commit, HistoryAction};
use crate::services::chronology_validate::{
    merged_attributes, validate_event, SubmittedEvent, ValidEvent, Vocabularies,
};
use crate::state::AppState;

use super::links::validated_links;
use super::support::{
    event_response, parse_event_id, refusal, require_event, require_live_event, seal_failure,
    write_failure,
};
use crate::api::timeline::case_slug;

/// The phase and tag vocabularies an event is judged against.
///
/// Read fresh on every write rather than held in the settings snapshot, because
/// both are EDITABLE data: a fifth phase or a sixth tag is a row (design R7,
/// R15), and a cached vocabulary would refuse a tag that had existed for
/// minutes. Two small queries against tables of four and five rows.
pub(super) struct LoadedVocabularies {
    phases: Vec<String>,
    tags: Vec<String>,
}

impl LoadedVocabularies {
    fn as_ref(&self) -> Vocabularies<'_> {
        Vocabularies {
            phases: &self.phases,
            tags: &self.tags,
        }
    }
}

/// Read both vocabularies, or a 500 naming which read failed.
async fn load_vocabularies(state: &AppState) -> Result<LoadedVocabularies, AppError> {
    let phases = list_phases(&state.pipeline_pool)
        .await
        .map_err(|e| write_failure(e, "the phase list, to judge a submitted event"))?;
    let tags = list_tags(&state.pipeline_pool)
        .await
        .map_err(|e| write_failure(e, "the tag vocabulary, to judge a submitted event"))?;
    Ok(LoadedVocabularies {
        phases: phases.into_iter().map(|p| p.id).collect(),
        tags: tags.into_iter().map(|t| t.id).collect(),
    })
}

/// `POST /api/timeline/events` — create one event, and any links it arrived with.
///
/// # Errors
/// 400 for a blank title or an unreadable date; 422 naming the value for an
/// unknown phase or tag, or for a document link whose target is not in the
/// store; 503 when `CASE_SLUG` is unset; 500 for a database failure.
#[tracing::instrument(skip(state, user, body), fields(by = %user.username))]
pub async fn post_event(
    user: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<CreateEventRequest>,
) -> Result<Json<TimelineEventDetailDto>, AppError> {
    let case = case_slug(&state)?;
    let writer = open_write(&user);
    let vocab = load_vocabularies(&state).await?;
    let valid = validate_event(
        SubmittedEvent {
            event_date: &body.event_date,
            title: &body.title,
            phase: &body.phase,
            fact: body.fact.as_deref(),
            date_precision: body.date_precision.as_deref(),
            approximate: body.approximate,
            tags: body.tags.as_deref(),
        },
        vocab.as_ref(),
    )
    .map_err(refusal)?;

    // Links are validated and their targets resolved BEFORE the transaction
    // opens, so a bad link id refuses the whole create rather than leaving an
    // event behind with a dead pointer — the exact state design R12 was written
    // after finding ten of.
    let links = validated_links(&state, body.links.as_deref().unwrap_or(&[])).await?;

    let event = create_event(&state, &case, &valid, &links, &writer.by_id).await?;
    let event = seal_and_commit(event.1, event.0, HistoryAction::Created, &writer)
        .await
        .map_err(|e| seal_failure(e, "creating an event"))?;

    tracing::info!(event_id = %event.id, by = %writer.by, "chronology: an event was created");
    Ok(Json(event_response(&state, &event).await?))
}

/// Insert the event and its links in ONE transaction, and hand the transaction
/// back UNCOMMITTED for the guard to seal.
///
/// ## Rust Learning: returning the transaction to the caller
///
/// This looks unusual and is the point: the only way to commit is
/// `seal_and_commit`, which consumes the transaction and writes the history row
/// first. A helper that committed here would be a write with no history, and
/// nothing but a reviewer's attention would have caught it.
async fn create_event(
    state: &AppState,
    case: &str,
    valid: &ValidEvent,
    links: &[crate::services::chronology_validate::ValidLink],
    by_id: &str,
) -> Result<(uuid::Uuid, sqlx::Transaction<'static, sqlx::Postgres>), AppError> {
    let attributes = merged_attributes(&serde_json::json!({}), Some(&valid.tags));
    let mut tx = state
        .pipeline_pool
        .begin()
        .await
        .map_err(|e| write_failure(e.into(), "opening the create transaction"))?;

    let id = insert_event(
        &mut *tx,
        &NewChronologyEvent {
            case_slug: case,
            event_date: valid.event_date,
            date_precision: valid.date_precision.as_str(),
            approximate: valid.approximate,
            phase: &valid.phase,
            title: &valid.title,
            fact: valid.fact.as_deref(),
            attributes: &attributes,
            created_by: by_id,
        },
    )
    .await
    .map_err(|e| write_failure(e, "inserting the event"))?;

    for link in links {
        insert_link(
            &mut *tx,
            &NewChronologyLink {
                event_id: id,
                target_type: &link.target_type,
                target_id: &link.target_id,
                label: link.label.as_deref(),
                pinpoint: link.pinpoint.as_deref(),
                created_by: by_id,
            },
        )
        .await
        .map_err(|e| write_failure(e, "linking a document to the new event"))?;
    }
    Ok((id, tx))
}

/// `PUT /api/timeline/events/:id` — edit the same fields.
///
/// # Errors
/// 400 / 422 as the create; 404 when there is no such event; 409 when it is
/// deleted (restore it with Undo first — a different answer from "gone", because
/// it is one press away).
#[tracing::instrument(skip(state, user, body), fields(by = %user.username, event_id = %id))]
pub async fn put_event(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateEventRequest>,
) -> Result<Json<TimelineEventDetailDto>, AppError> {
    let event_id = parse_event_id(&id)?;
    let writer = open_write(&user);
    let existing = require_live_event(&state, event_id, "the event being edited").await?;
    let vocab = load_vocabularies(&state).await?;
    let valid = validate_event(
        SubmittedEvent {
            event_date: &body.event_date,
            title: &body.title,
            phase: &body.phase,
            fact: body.fact.as_deref(),
            date_precision: body.date_precision.as_deref(),
            approximate: body.approximate,
            tags: body.tags.as_deref(),
        },
        vocab.as_ref(),
    )
    .map_err(refusal)?;

    let tx = apply_edit(&state, event_id, &existing, &valid, &body, &writer.by_id).await?;
    let event = seal_and_commit(tx, event_id, HistoryAction::Updated, &writer)
        .await
        .map_err(|e| seal_failure(e, "editing an event"))?;
    tracing::info!(event_id = %event.id, by = %writer.by, "chronology: an event was edited");
    Ok(Json(event_response(&state, &event).await?))
}

/// Open the transaction and apply one edit, handing it back UNCOMMITTED.
///
/// Split from the handler for Rule 18 and because the handler then reads as the
/// four steps it is — fence the id, fence the row, judge the submission, write —
/// with the transaction discipline in one place. Like [`create_event`], it
/// returns the transaction rather than committing: the only way to commit is
/// `seal_and_commit`, which writes the history row first.
async fn apply_edit(
    state: &AppState,
    event_id: uuid::Uuid,
    existing: &ChronologyEventStateRow,
    valid: &ValidEvent,
    body: &UpdateEventRequest,
    by_id: &str,
) -> Result<sqlx::Transaction<'static, sqlx::Postgres>, AppError> {
    // ⚑ The stored bag is MERGED, never rebuilt: `people`, `spine` and every
    // seeded event's `source: legacy_json` survive an edit by today's form,
    // which knows about none of them. That is the change rule (design R4) at the
    // moment it matters most. An ABSENT `tags` leaves the stored tags alone; an
    // empty one clears them, which is why the request's field decides and not
    // the validated list.
    let attributes = merged_attributes(
        &existing.attributes,
        body.tags.as_ref().map(|_| valid.tags.as_slice()),
    );

    let mut tx = state
        .pipeline_pool
        .begin()
        .await
        .map_err(|e| write_failure(e.into(), "opening the edit transaction"))?;
    let changed = update_event(
        &mut *tx,
        event_id,
        &EventEdit {
            event_date: valid.event_date,
            date_precision: valid.date_precision.as_str(),
            approximate: valid.approximate,
            phase: &valid.phase,
            title: &valid.title,
            fact: valid.fact.as_deref(),
            attributes: &attributes,
        },
        by_id,
    )
    .await
    .map_err(|e| write_failure(e, "applying the edit"))?;
    if changed == 0 {
        // The row was live when it was read and is not now — somebody deleted it
        // between the two statements. Reported rather than sealed: a history row
        // saying "edited" over an event nobody edited would be a false record.
        // Dropping `tx` here rolls it back, which is what "nothing was changed"
        // is asserting.
        return Err(AppError::Conflict {
            message: format!(
                "chronology event {event_id} was deleted while this edit was being made; \
                 nothing was changed"
            ),
            details: serde_json::json!({ "event_id": event_id }),
        });
    }
    Ok(tx)
}

/// `DELETE /api/timeline/events/:id` — SOFT delete (design R10).
///
/// Nothing is removed and there is NO confirm dialog anywhere: the undo line
/// that replaces the card in place is the safety, which is the pattern already
/// ruled on the practice page. The response is the event it just deleted, so the
/// surface draws that line from the server's answer rather than from a guess.
///
/// # Errors
/// 404 when there is no such event. Deleting an already-deleted event is a
/// success that changes nothing and writes no history — see below.
#[tracing::instrument(skip(state, user), fields(by = %user.username, event_id = %id))]
pub async fn delete_event(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<TimelineEventDetailDto>, AppError> {
    let event_id = parse_event_id(&id)?;
    let writer = open_write(&user);
    let existing = require_event(&state, event_id, "the event being deleted").await?;
    if existing.deleted_at.is_some() {
        // Already deleted. Answering 200 with the row is right — the caller's
        // intent holds — but sealing would append a second `deleted` history row
        // for a delete that did not happen, and history is a record of acts, not
        // of requests.
        tracing::info!(%event_id, "chronology: delete of an already-deleted event; no history written");
        return Ok(Json(event_response(&state, &existing).await?));
    }

    let mut tx = state
        .pipeline_pool
        .begin()
        .await
        .map_err(|e| write_failure(e.into(), "opening the delete transaction"))?;
    soft_delete_event(&mut *tx, event_id, &writer.by_id)
        .await
        .map_err(|e| write_failure(e, "soft-deleting the event"))?;
    let event = seal_and_commit(tx, event_id, HistoryAction::Deleted, &writer)
        .await
        .map_err(|e| seal_failure(e, "deleting an event"))?;

    tracing::info!(event_id = %event.id, by = %writer.by, "chronology: an event was deleted (soft)");
    Ok(Json(event_response(&state, &event).await?))
}

/// `POST /api/timeline/events/:id/undelete` — the Undo (design R10).
///
/// # Errors
/// 404 when there is no such event. Undoing a live event is a success that
/// changes nothing and writes no history, for the same reason a second delete
/// does not.
#[tracing::instrument(skip(state, user), fields(by = %user.username, event_id = %id))]
pub async fn post_undelete(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<TimelineEventDetailDto>, AppError> {
    let event_id = parse_event_id(&id)?;
    let writer = open_write(&user);
    let existing = require_event(&state, event_id, "the event being restored").await?;
    if existing.deleted_at.is_none() {
        tracing::info!(%event_id, "chronology: undo of a live event; no history written");
        return Ok(Json(event_response(&state, &existing).await?));
    }

    let mut tx = state
        .pipeline_pool
        .begin()
        .await
        .map_err(|e| write_failure(e.into(), "opening the undo transaction"))?;
    undelete_event(&mut *tx, event_id, &writer.by_id)
        .await
        .map_err(|e| write_failure(e, "restoring the event"))?;
    let event = seal_and_commit(tx, event_id, HistoryAction::Restored, &writer)
        .await
        .map_err(|e| seal_failure(e, "restoring an event"))?;

    tracing::info!(event_id = %event.id, by = %writer.by, "chronology: an event was restored");
    Ok(Json(event_response(&state, &event).await?))
}

#[cfg(test)]
#[path = "events_tests.rs"]
mod tests;
