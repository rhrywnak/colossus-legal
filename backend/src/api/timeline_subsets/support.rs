//! What the subset handlers share: status mapping, id parsing, composition.
//!
//! Split out for Rule 17 and because a shared answer written twice is two
//! answers waiting to differ. The mapping from a refusal to an HTTP status is
//! the one that matters most: three handlers refuse a duplicate name, and if
//! each wrote its own `match`, one of them would eventually answer 400 where the
//! others answer 409.
//!
//! ## CRITICAL — the pipeline pool
//!
//! Every query reached from here uses `&state.pipeline_pool`, NOT `state.pg_pool`.

use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::dto::chronology_subset::{SubsetDetailDto, SubsetEventRef};
use crate::error::AppError;
use crate::repositories::pipeline_repository::chronology_links::existing_document_ids;
use crate::repositories::pipeline_repository::chronology_subsets::{
    carriers_for_subsets, events_any_state_by_ids, existing_event_ids_in_case,
    get_subset_any_state, links_for_events, list_subset_event_ids, note_counts_for_events,
    ChronologySubsetRow,
};
use crate::repositories::pipeline_repository::PipelineRepoError;
use crate::services::chronology_read::CHECKABLE_TARGET_TYPE;
use crate::services::chronology_subset_guard::SubsetSealError;
use crate::services::chronology_subset_read::{
    build_subset_detail, carriers_by_subset, SubsetDetailSources,
};
use crate::services::chronology_subset_validate::{
    validate_events, SubmittedSubsetEvent, SubsetWriteRefusal, ValidSubsetEvent,
};
use crate::services::chronology_subset_write::SubsetWriteError;
use crate::state::AppState;

/// Turn a repository failure into a 500 that names the operation and the cause.
///
/// Every `?` in this directory terminates here, at [`refusal`], or at
/// [`write_error`], so a reader of the logs can always tell WHICH read or write
/// failed and why — the alternative, one generic "database error", is the silent
/// failure Standing Rule 1 forbids.
pub(crate) fn subset_failure(error: PipelineRepoError, what: &str) -> AppError {
    tracing::error!(
        operation = %what,
        error = %error,
        "timeline subset operation failed"
    );
    AppError::Internal {
        message: format!("the timeline subset could not be read or written ({what})"),
    }
}

/// Turn a seal failure into a 500. The write is already rolled back.
fn seal_failure(error: &SubsetSealError, what: &str) -> AppError {
    tracing::error!(
        operation = %what,
        error = %error,
        "timeline subset write could not be sealed; the whole transaction rolled back"
    );
    AppError::Internal {
        message: format!("the subset change could not be recorded ({what}), so it was not made"),
    }
}

/// Turn a write failure into the status its KIND deserves.
///
/// `Deleted` is a 409 and not a 404 on purpose: the subset is there, in a state
/// that cannot take an edit, and it is one press of Undo away. Telling somebody
/// their story is gone when it is recoverable is the collapse this table exists
/// to prevent.
pub(crate) fn write_error(error: SubsetWriteError, what: &str) -> AppError {
    match error {
        SubsetWriteError::Repo { source } => subset_failure(source, what),
        SubsetWriteError::Seal { ref source } => seal_failure(source, what),
        SubsetWriteError::Deleted { subset_id } => {
            tracing::info!(%subset_id, "timeline subset write refused: the subset is deleted");
            AppError::Conflict {
                message: error.to_string(),
                details: serde_json::json!({ "subset_id": subset_id }),
            }
        }
    }
}

/// Turn a validation refusal into the status its KIND deserves.
///
/// ## ⚑ The 400/409/422 table, in one place
///
/// `SubsetWriteRefusal` decides (`is_conflict`, `is_unprocessable`); this
/// function only maps. The refused VALUE and the FIELD both reach the body's
/// `details`, so a form can highlight the box and quote what was rejected.
pub(crate) fn refusal(refusal: SubsetWriteRefusal) -> AppError {
    let details = serde_json::json!({
        "field": refusal.field(),
        "value": refusal.value(),
    });
    let message = refusal.to_string();
    tracing::info!(
        field = refusal.field().unwrap_or("(whole request)"),
        conflict = refusal.is_conflict(),
        unprocessable = refusal.is_unprocessable(),
        "timeline subset write refused: {message}"
    );
    if refusal.is_conflict() {
        AppError::Conflict { message, details }
    } else if refusal.is_unprocessable() {
        AppError::UnprocessableEntity { message, details }
    } else {
        AppError::BadRequest { message, details }
    }
}

/// Parse a path id, or a 400 that says what an id looks like.
pub(crate) fn parse_subset_id(id: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(id).map_err(|_| AppError::BadRequest {
        message: format!("'{id}' is not a timeline subset id"),
        details: serde_json::json!({ "expected": "a UUID" }),
    })
}

/// Read one subset whatever state it is in, or a 404 naming the id.
pub(crate) async fn require_subset(
    state: &AppState,
    subset_id: Uuid,
    what: &str,
) -> Result<ChronologySubsetRow, AppError> {
    get_subset_any_state(&state.pipeline_pool, subset_id)
        .await
        .map_err(|e| subset_failure(e, what))?
        .ok_or_else(|| AppError::NotFound {
            message: format!("no timeline subset {subset_id}"),
        })
}

/// Judge a submitted ordered event set against what this case actually holds.
///
/// One query for every id at once, then the pure rules. Doing it BEFORE the
/// transaction opens is deliberate: a bad event id refuses the whole write
/// rather than leaving a subset behind with a dead pointer — the same discipline
/// the event create applies to its links.
pub(crate) async fn validated_events(
    state: &AppState,
    case_slug: &str,
    submitted: &[SubsetEventRef],
) -> Result<Vec<ValidSubsetEvent>, AppError> {
    let ids: Vec<Uuid> = submitted.iter().map(|e| e.event_id).collect();
    let known: HashSet<Uuid> = existing_event_ids_in_case(&state.pipeline_pool, case_slug, &ids)
        .await
        .map_err(|e| subset_failure(e, "which of these events exist in this case"))?
        .into_iter()
        .collect();
    let borrowed: Vec<SubmittedSubsetEvent<'_>> = submitted
        .iter()
        .map(|e| SubmittedSubsetEvent {
            event_id: e.event_id,
            position: e.position,
            note: e.note.as_deref(),
        })
        .collect();
    validate_events(&borrowed, &known).map_err(refusal)
}

/// The composed subset every read and every write answers with.
///
/// Five reads after the write has committed. At the design's volume (twelve to
/// twenty events) that is cheap, and it is the only way the answer is the
/// SERVER's account of the story rather than the handler's memory of what it
/// asked for — which is what keeps a surface from drifting after a normalising
/// validator trims a name or a note.
pub(crate) async fn subset_response(
    state: &AppState,
    subset: &ChronologySubsetRow,
) -> Result<SubsetDetailDto, AppError> {
    let refs = list_subset_event_ids(&state.pipeline_pool, subset.id)
        .await
        .map_err(|e| subset_failure(e, "this subset's event references"))?;
    let event_ids: Vec<Uuid> = refs.iter().map(|r| r.event_id).collect();

    let events = events_any_state_by_ids(&state.pipeline_pool, &event_ids)
        .await
        .map_err(|e| subset_failure(e, "the events this subset references"))?;
    let links = links_for_events(&state.pipeline_pool, &event_ids)
        .await
        .map_err(|e| subset_failure(e, "those events' links"))?;
    let counts = note_counts_for_events(&state.pipeline_pool, &event_ids)
        .await
        .map_err(|e| subset_failure(e, "those events' note counts"))?;
    let carriers = carriers_for_subsets(&state.pipeline_pool, &[subset.id])
        .await
        .map_err(|e| subset_failure(e, "the scenarios carrying this subset"))?;

    let resolved = resolve_documents(state, &links).await?;
    let note_counts: HashMap<Uuid, i64> = counts.into_iter().collect();
    let carried_by = carriers_by_subset(&carriers)
        .remove(&subset.id)
        .unwrap_or_default();

    let composed = build_subset_detail(SubsetDetailSources {
        subset,
        refs: &refs,
        events: &events,
        links: &links,
        note_counts: &note_counts,
        resolved_documents: &resolved,
        carried_by: &carried_by,
    });
    for warning in &composed.warnings {
        tracing::warn!(surface = "timeline subset", "{warning}");
    }
    Ok(composed.payload)
}

/// Which of these links' document targets actually exist.
///
/// The same three-state resolution the timeline uses, reached through the same
/// query, so a document link reads identically on both surfaces. Only checkable
/// types are asked about, because the answer for any other type would be a claim
/// nobody checked.
async fn resolve_documents(
    state: &AppState,
    links: &[crate::repositories::pipeline_repository::chronology::ChronologyLinkRow],
) -> Result<HashSet<String>, AppError> {
    let mut ids: Vec<String> = links
        .iter()
        .filter(|l| l.target_type == CHECKABLE_TARGET_TYPE)
        .map(|l| l.target_id.clone())
        .collect();
    ids.sort();
    ids.dedup();
    existing_document_ids(&state.pipeline_pool, &ids)
        .await
        .map_err(|e| subset_failure(e, "which linked documents exist"))
}

/// The status-mapping proofs: what each failure actually says.
#[cfg(test)]
#[path = "support_tests.rs"]
mod support_tests;
