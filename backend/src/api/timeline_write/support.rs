//! What the chronology's two write-handler modules share.
//!
//! Split out for Rule 17 and because a shared answer written twice is two
//! answers waiting to differ: the mapping from a refusal to an HTTP status, the
//! composition of a write's response, and the target resolution the READ handler
//! also uses all live here, once.
//!
//! ## CRITICAL — the pipeline pool
//!
//! Every chronology table lives in `colossus_legal_v2`, so every query reached
//! from here uses `&state.pipeline_pool`, NOT `state.pg_pool`.

use std::collections::HashSet;

use uuid::Uuid;

use crate::dto::chronology::TimelineEventDetailDto;
use crate::error::AppError;
use crate::repositories::pipeline_repository::chronology::ChronologyLinkRow;
use crate::repositories::pipeline_repository::chronology_links::{
    existing_document_ids, list_history_for_event, list_links_for_event, list_notes_for_event,
};
use crate::repositories::pipeline_repository::chronology_write::{
    get_event_any_state, ChronologyEventStateRow,
};
use crate::repositories::pipeline_repository::PipelineRepoError;
use crate::services::chronology_guard::SealError;
use crate::services::chronology_read::CHECKABLE_TARGET_TYPE;
use crate::services::chronology_validate::ChronologyWriteRefusal;
use crate::services::chronology_write_response::{build_write_response, WriteResponseSources};
use crate::state::AppState;

/// Turn a repository failure into a 500 that names the operation and the cause.
///
/// Every `?` on a write terminates here or at [`refusal`], so a reader of the
/// logs can always tell WHICH write failed and why — the alternative, one
/// generic "database error", is the silent failure Standing Rule 1 forbids.
pub(crate) fn write_failure(error: PipelineRepoError, what: &str) -> AppError {
    tracing::error!(
        operation = %what,
        error = %error,
        "chronology write failed"
    );
    AppError::Internal {
        message: format!("the chronology could not be written ({what})"),
    }
}

/// Turn a seal failure into a 500. The write is already rolled back.
///
/// A separate function from [`write_failure`] because a seal failure is a
/// different event to an operator: the mutation itself succeeded and the RECORD
/// of it did not, so the log says so rather than reporting a failed write that
/// might have half-happened.
pub(crate) fn seal_failure(error: SealError, what: &str) -> AppError {
    tracing::error!(
        operation = %what,
        error = %error,
        "chronology write could not be sealed; the whole transaction rolled back"
    );
    AppError::Internal {
        message: format!(
            "the chronology change could not be recorded ({what}), so it was not made"
        ),
    }
}

/// Turn a validation refusal into the status its KIND deserves.
///
/// ## ⚑ The 400/422 table, in one place
///
/// `ChronologyWriteRefusal::is_unprocessable` decides; this function only maps.
/// Two handlers each writing their own `match` is how one of them would
/// eventually answer 400 to an unknown phase — which is the exact defect Phase C
/// names ("an unknown phase is a 422 naming the value, never a 500").
///
/// The refused VALUE and the FIELD both reach the body's `details`, so a form
/// can highlight the box and quote what was rejected.
pub(crate) fn refusal(refusal: ChronologyWriteRefusal) -> AppError {
    let details = serde_json::json!({
        "field": refusal.field(),
        "value": refusal.value(),
    });
    let message = refusal.to_string();
    tracing::info!(
        field = refusal.field().unwrap_or("(whole request)"),
        unprocessable = refusal.is_unprocessable(),
        "chronology write refused: {message}"
    );
    if refusal.is_unprocessable() {
        AppError::UnprocessableEntity { message, details }
    } else {
        AppError::BadRequest { message, details }
    }
}

/// Every document id the given links point at, so resolution is ONE query.
///
/// Moved here from `api::timeline` in Phase C, so the read handler and the write
/// handlers resolve targets by the same code. Only checkable types are asked
/// about, because the answer for any other type would be meaningless.
pub(crate) fn checkable_target_ids(links: &[ChronologyLinkRow]) -> Vec<String> {
    let mut ids: Vec<String> = links
        .iter()
        .filter(|l| l.target_type == CHECKABLE_TARGET_TYPE)
        .map(|l| l.target_id.clone())
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

/// Which of these links' document targets actually exist.
pub(crate) async fn resolve_targets(
    state: &AppState,
    links: &[ChronologyLinkRow],
    what: &str,
) -> Result<HashSet<String>, AppError> {
    let ids = checkable_target_ids(links);
    existing_document_ids(&state.pipeline_pool, &ids)
        .await
        .map_err(|e| write_failure(e, what))
}

/// Parse a path id, or a 400 that says what an id looks like.
pub(crate) fn parse_event_id(id: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(id).map_err(|_| AppError::BadRequest {
        message: format!("'{id}' is not a chronology event id"),
        details: serde_json::json!({ "expected": "a UUID" }),
    })
}

/// The composed response every write answers with: the event, whatever state it
/// is now in, with its links, notes and history.
///
/// Four reads after the write has committed. At the design's volume that is
/// cheap, and it is the only way the answer is the SERVER's account of the row
/// rather than the handler's memory of what it asked for — which is what §C3
/// requires and what a normalising validator (a trimmed title, a cleared fact)
/// would otherwise make untrue.
pub(crate) async fn event_response(
    state: &AppState,
    event: &ChronologyEventStateRow,
) -> Result<TimelineEventDetailDto, AppError> {
    let links = list_links_for_event(&state.pipeline_pool, event.id)
        .await
        .map_err(|e| write_failure(e, "this event's links after the write"))?;
    let notes = list_notes_for_event(&state.pipeline_pool, event.id)
        .await
        .map_err(|e| write_failure(e, "this event's notes after the write"))?;
    let history = list_history_for_event(&state.pipeline_pool, event.id)
        .await
        .map_err(|e| write_failure(e, "this event's history after the write"))?;
    let resolved = resolve_targets(state, &links, "which linked documents exist").await?;

    let composed = build_write_response(WriteResponseSources {
        event,
        links: &links,
        notes: &notes,
        history: &history,
        resolved_documents: &resolved,
    });
    for warning in &composed.warnings {
        tracing::warn!(surface = "chronology write", "{warning}");
    }
    Ok(composed.payload)
}

/// Read one event whatever state it is in, or a 404 naming the id.
///
/// Used by the handlers that need the row BEFORE they change it — the edit's
/// attribute merge, and the note and link paths, which must refuse to hang
/// anything off an event that does not exist.
pub(crate) async fn require_event(
    state: &AppState,
    event_id: Uuid,
    what: &str,
) -> Result<ChronologyEventStateRow, AppError> {
    get_event_any_state(&state.pipeline_pool, event_id)
        .await
        .map_err(|e| write_failure(e, what))?
        .ok_or_else(|| AppError::NotFound {
            message: format!("no chronology event {event_id}"),
        })
}

/// The same, but refusing a DELETED event.
///
/// ## Why two functions and not a boolean
///
/// A boolean parameter at a call site reads as `require_event(state, id, true)`
/// and tells a reader nothing. More importantly the two REFUSALS differ: adding
/// a note to a deleted event is a 409 (the event is there, in a state that
/// cannot take a note) while adding one to an id that never existed is a 404.
/// Collapsing them would tell somebody their event is gone when it is one Undo
/// away.
pub(crate) async fn require_live_event(
    state: &AppState,
    event_id: Uuid,
    what: &str,
) -> Result<ChronologyEventStateRow, AppError> {
    let event = require_event(state, event_id, what).await?;
    if event.deleted_at.is_some() {
        return Err(AppError::Conflict {
            message: format!(
                "chronology event {event_id} is deleted; restore it with Undo before changing it"
            ),
            details: serde_json::json!({ "deleted_at": event.deleted_at }),
        });
    }
    Ok(event)
}

#[cfg(test)]
#[path = "support_tests.rs"]
mod tests;
