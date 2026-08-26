//! The chronology's LINK, NOTE and document-picker endpoints (Phase C, §C1).
//!
//! ```text
//! POST   /api/timeline/events/:id/links            link a target
//! DELETE /api/timeline/events/:id/links?…          unlink, by the natural key
//! POST   /api/timeline/events/:id/notes            add an attributed note
//! DELETE /api/timeline/events/:id/notes/:note_id   the author retires their own
//! GET    /api/timeline/documents?q=…               the picker's search
//! ```
//!
//! Split from [`super::events`] for Rule 17. The seam is honest as well as
//! arithmetical: that module changes the dated FACT, and everything here hangs
//! something off an event that already exists.
//!
//! ## ⚑ Same guard, same seal
//!
//! Every mutating handler here takes `user: AuthUser` (a 401 for anonymous
//! before the body runs), calls `open_write` to stamp the acting user, and ends
//! at `seal_and_commit` — so linking a document and writing a note each land
//! ONE history row on the event, exactly as an edit does. A link or a note IS a
//! change to what the event says; recording only the field edits would leave a
//! history that quietly omitted half of what people did.
//!
//! The document PICKER is a read and stays open, like every other chronology
//! read (Phase A).
//!
//! ## CRITICAL — the pipeline pool
//!
//! Every table here lives in `colossus_legal_v2`, `documents` included — which
//! is the same table the link resolver reads, deliberately. See
//! `document_titles::search_document_titles`.

use axum::{
    extract::{Path, Query, State},
    Json,
};

use crate::auth::AuthUser;
use crate::dto::chronology::TimelineEventDetailDto;
use crate::dto::chronology_write::{
    CreateLinkRequest, CreateNoteRequest, DeleteLinkQuery, DocumentChoiceDto, DocumentSearchQuery,
    DocumentSearchResultDto,
};
use crate::error::AppError;
use crate::repositories::pipeline_repository::chronology_links::existing_document_ids;
use crate::repositories::pipeline_repository::chronology_note_write::{
    get_note_any_state, insert_note, note_is_deletable_by, soft_delete_note,
};
use crate::repositories::pipeline_repository::chronology_write::{
    delete_link, insert_link, NewChronologyLink,
};
use crate::repositories::pipeline_repository::document_titles::search_document_titles;
use crate::services::chronology_guard::{open_write, seal_and_commit, HistoryAction};
use crate::services::chronology_read::CHECKABLE_TARGET_TYPE;
use crate::services::chronology_validate::{validate_link, validate_note, ValidLink};
use crate::state::AppState;

use super::support::{
    event_response, parse_event_id, refusal, require_live_event, seal_failure, write_failure,
};

/// Validate a batch of submitted links and prove every checkable target exists.
///
/// ## ⚑ THE DEFECT THIS FUNCTION IS NAMED AFTER
///
/// Ten of the eleven links in the retired `timeline.json` pointed at document
/// ids that did not exist, and the page rendered every one of them as a live
/// blue link because nothing ever asked. §C1: "for target_type=document,
/// creation VALIDATES the target exists and returns its resolution". So a dead
/// document id is refused at the moment somebody tries to create it, with a 422
/// naming the id — never stored and discovered later by a reader.
///
/// Other target types are stored UNCHECKED, also by §C1, and that is not a
/// loophole: this build has no resolver for a `statement` or a
/// `paperless_document`, so "it exists" would be a claim nobody had checked. The
/// read side reports those as `unchecked`, which is a third answer and not a
/// quiet yes.
///
/// # Errors
/// 400 naming the blank field; 422 naming the document id that is not there.
pub(crate) async fn validated_links(
    state: &AppState,
    submitted: &[CreateLinkRequest],
) -> Result<Vec<ValidLink>, AppError> {
    let mut links = Vec::with_capacity(submitted.len());
    for raw in submitted {
        links.push(
            validate_link(
                &raw.target_type,
                &raw.target_id,
                raw.label.as_deref(),
                raw.pinpoint.as_deref(),
            )
            .map_err(refusal)?,
        );
    }

    let mut wanted: Vec<String> = links
        .iter()
        .filter(|l| l.target_type == CHECKABLE_TARGET_TYPE)
        .map(|l| l.target_id.clone())
        .collect();
    wanted.sort();
    wanted.dedup();
    if wanted.is_empty() {
        return Ok(links);
    }

    let found = existing_document_ids(&state.pipeline_pool, &wanted)
        .await
        .map_err(|e| write_failure(e, "checking that the linked documents exist"))?;
    for id in &wanted {
        if !found.contains(id) {
            return Err(AppError::UnprocessableEntity {
                message: format!(
                    "no document '{id}' is in this system, so an event cannot be linked to it. \
                     If it has not been scanned yet, leave the event unlinked — an unlinked \
                     event is the to-scan to-do list"
                ),
                details: serde_json::json!({ "field": "target_id", "value": id }),
            });
        }
    }
    Ok(links)
}

/// `POST /api/timeline/events/:id/links` — link one target to one event.
///
/// # Errors
/// 400 for a blank target field; 404 for no such event; 409 when the event is
/// deleted; 422 naming a document id that is not in the store.
#[tracing::instrument(skip(state, user, body), fields(by = %user.username, event_id = %id))]
pub async fn post_link(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<CreateLinkRequest>,
) -> Result<Json<TimelineEventDetailDto>, AppError> {
    let event_id = parse_event_id(&id)?;
    let writer = open_write(&user);
    require_live_event(&state, event_id, "the event being linked").await?;
    let links = validated_links(&state, std::slice::from_ref(&body)).await?;

    let mut tx = state
        .pipeline_pool
        .begin()
        .await
        .map_err(|e| write_failure(e.into(), "opening the link transaction"))?;
    for link in &links {
        insert_link(
            &mut *tx,
            &NewChronologyLink {
                event_id,
                target_type: &link.target_type,
                target_id: &link.target_id,
                label: link.label.as_deref(),
                pinpoint: link.pinpoint.as_deref(),
                created_by: &writer.by_id,
            },
        )
        .await
        .map_err(|e| write_failure(e, "inserting the link"))?;
    }
    let event = seal_and_commit(tx, event_id, HistoryAction::Updated, &writer)
        .await
        .map_err(|e| seal_failure(e, "linking a document"))?;

    tracing::info!(%event_id, by = %writer.by, "chronology: a document was linked");
    Ok(Json(event_response(&state, &event).await?))
}

/// `DELETE /api/timeline/events/:id/links?target_type=…&target_id=…`
///
/// The natural key is the address, because that is what a human picked off a
/// screen — no surrogate id had to be invented for a row somebody can point at.
///
/// # Errors
/// 400 for a blank key field; 404 for no such event OR no such link on it; 409
/// when the event is deleted.
#[tracing::instrument(skip(state, user, query), fields(by = %user.username, event_id = %id))]
pub async fn delete_event_link(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<DeleteLinkQuery>,
) -> Result<Json<TimelineEventDetailDto>, AppError> {
    let event_id = parse_event_id(&id)?;
    let writer = open_write(&user);
    require_live_event(&state, event_id, "the event being unlinked").await?;
    // Validated for the same reason a create is: the natural key is three
    // values, and a blank one would silently match nothing and report success.
    let key = validate_link(&query.target_type, &query.target_id, None, None).map_err(refusal)?;

    let mut tx = state
        .pipeline_pool
        .begin()
        .await
        .map_err(|e| write_failure(e.into(), "opening the unlink transaction"))?;
    let removed = delete_link(&mut *tx, event_id, &key.target_type, &key.target_id)
        .await
        .map_err(|e| write_failure(e, "removing the link"))?;
    if removed == 0 {
        // Reported, not sealed. A history row saying the event changed when no
        // link was removed would be a false record, and answering 200 would tell
        // somebody a stale screen's Remove had worked.
        return Err(AppError::NotFound {
            message: format!(
                "chronology event {event_id} has no {} link to '{}'",
                key.target_type, key.target_id
            ),
        });
    }

    let event = seal_and_commit(tx, event_id, HistoryAction::Updated, &writer)
        .await
        .map_err(|e| seal_failure(e, "removing a link"))?;
    tracing::info!(%event_id, by = %writer.by, "chronology: a link was removed");
    Ok(Json(event_response(&state, &event).await?))
}

/// `POST /api/timeline/events/:id/notes` — one attributed note (design R8).
///
/// # Errors
/// 400 for a blank note; 404 for no such event; 409 when the event is deleted.
#[tracing::instrument(skip(state, user, body), fields(by = %user.username, event_id = %id))]
pub async fn post_note(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<CreateNoteRequest>,
) -> Result<Json<TimelineEventDetailDto>, AppError> {
    let event_id = parse_event_id(&id)?;
    let writer = open_write(&user);
    require_live_event(&state, event_id, "the event being annotated").await?;
    let note = validate_note(&body.note).map_err(refusal)?;

    let mut tx = state
        .pipeline_pool
        .begin()
        .await
        .map_err(|e| write_failure(e.into(), "opening the note transaction"))?;
    insert_note(&mut *tx, event_id, &note, &writer.by_id)
        .await
        .map_err(|e| write_failure(e, "inserting the note"))?;
    let event = seal_and_commit(tx, event_id, HistoryAction::Updated, &writer)
        .await
        .map_err(|e| seal_failure(e, "adding a note"))?;

    tracing::info!(%event_id, by = %writer.by, "chronology: a note was added");
    Ok(Json(event_response(&state, &event).await?))
}

/// `DELETE /api/timeline/events/:id/notes/:note_id` — retire your own note.
///
/// # Domain note — the one place the three authors are NOT equal
///
/// R2 makes them equal on events: anyone may add, edit or delete any event, and
/// history makes every act attributable. A note is a signed remark rather than a
/// shared field (R8), so only its author may withdraw it. The refusal is a 403
/// naming the author, not a 404 — pretending somebody else's note does not exist
/// would be a lie a reader can see through by scrolling.
///
/// # Errors
/// 404 for no such note or a note on a different event; 403 when it is not
/// yours; 409 when the event is deleted.
#[tracing::instrument(skip(state, user), fields(by = %user.username, event_id = %id, note_id = %note_id))]
pub async fn delete_note(
    user: AuthUser,
    State(state): State<AppState>,
    Path((id, note_id)): Path<(String, String)>,
) -> Result<Json<TimelineEventDetailDto>, AppError> {
    let event_id = parse_event_id(&id)?;
    let note_uuid = parse_event_id(&note_id)?;
    let writer = open_write(&user);
    require_live_event(&state, event_id, "the event whose note is being deleted").await?;

    let note = get_note_any_state(&state.pipeline_pool, note_uuid)
        .await
        .map_err(|e| write_failure(e, "reading the note being deleted"))?
        .filter(|n| n.event_id == event_id && n.deleted_at.is_none())
        .ok_or_else(|| AppError::NotFound {
            message: format!("no live note {note_uuid} on chronology event {event_id}"),
        })?;
    if !note_is_deletable_by(&note, &user.username) {
        return Err(AppError::Forbidden {
            message: format!(
                "that note was written by {} and only its author may delete it",
                note.created_by
                    .as_deref()
                    .unwrap_or("nobody this build can name")
            ),
        });
    }

    let mut tx = state
        .pipeline_pool
        .begin()
        .await
        .map_err(|e| write_failure(e.into(), "opening the note-delete transaction"))?;
    soft_delete_note(&mut *tx, note_uuid)
        .await
        .map_err(|e| write_failure(e, "soft-deleting the note"))?;
    let event = seal_and_commit(tx, event_id, HistoryAction::Updated, &writer)
        .await
        .map_err(|e| seal_failure(e, "deleting a note"))?;

    tracing::info!(%event_id, %note_uuid, by = %writer.by, "chronology: a note was deleted (soft)");
    Ok(Json(event_response(&state, &event).await?))
}

/// `GET /api/timeline/documents?q=…` — the picker's search.
///
/// An open READ, like every other chronology read: seeing which documents exist
/// is not privileged, and the write that USES a choice is guarded.
///
/// # Errors
/// 400 when `q` is blank — a picker that dumps the whole store is not a picker;
/// 500 for a database failure.
#[tracing::instrument(skip(state, user, query), fields(matches = tracing::field::Empty))]
pub async fn get_document_choices(
    user: Option<AuthUser>,
    State(state): State<AppState>,
    Query(query): Query<DocumentSearchQuery>,
) -> Result<Json<DocumentSearchResultDto>, AppError> {
    if let Some(ref u) = user {
        tracing::info!("{} GET /timeline/documents", u.username);
    }
    let needle = query.q.trim();
    if needle.is_empty() {
        return Err(AppError::BadRequest {
            message: "type something to search for — the document picker offers matches, \
                      not the whole store"
                .to_string(),
            details: serde_json::json!({ "field": "q" }),
        });
    }

    let limit = state.settings.current().chronology_document_picker_max;
    let page = search_document_titles(&state.pipeline_pool, needle, limit as i64)
        .await
        .map_err(|e| write_failure(e, "searching the documents for the picker"))?;

    tracing::Span::current().record("matches", page.total);
    if page.total > page.matches.len() as i64 {
        // ⚑ NOT A SILENT CAP. The response says so too — this line is so the
        // same fact is in the log, where somebody tuning the stored number can
        // see how often it bites.
        tracing::info!(
            total = page.total,
            shown = page.matches.len(),
            "chronology document picker: the short list was capped"
        );
    }
    Ok(Json(DocumentSearchResultDto {
        matches: page
            .matches
            .into_iter()
            .map(|(id, title)| DocumentChoiceDto { id, title })
            .collect(),
        total: page.total,
        shown_limit: limit,
    }))
}
