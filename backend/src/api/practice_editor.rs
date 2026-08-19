//! The deck editor's four writes, each of them signed and recorded.
//!
//! Task B1. Chuck re-orders, re-words, hides and adds; every one of those leaves
//! a `practice_deck_changes` row naming who did it and what it was before.
//!
//! ## Why "Editing as" is enforced HERE and not only on the screen
//!
//! There is one login. The whole honesty of the record rests on the editor
//! having said who they are, and a control that could be bypassed by a curl
//! would make the record a suggestion. So `editing_as` is a required field on
//! every request, checked against the stored vocabulary, and a change signed by
//! somebody the store does not list is a 400 that names the list.
//!
//! ## Why every write is one transaction
//!
//! The edit and its change row are one act. A hidden question with no record of
//! who hid it is worse than either — Marie is told a question vanished and
//! nobody can say why.
//!
//! ## CRITICAL — the pipeline pool
//!
//! Every table here lives in `colossus_legal_v2`.

use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    domain::practice_params::TACTIC_CARD_MAX,
    dto::practice_review::{
        DeckChangeResponse, EditQuestionRequest, HideQuestionRequest, MoveQuestionRequest,
    },
    error::AppError,
    repositories::pipeline_repository::{
        practice::{get_question, list_deck, PracticeQuestionRecord},
        practice_editor::{
            log_change, set_field, set_hidden, set_tactic, swap_sort_order, NewChange,
        },
    },
    services::practice_notes::attribution,
    state::AppState,
};

use super::practice::repo_error;

/// Read one question, or 404 naming it.
async fn require_question(
    state: &AppState,
    question_id: Uuid,
) -> Result<PracticeQuestionRecord, AppError> {
    get_question(&state.pipeline_pool, question_id)
        .await
        .map_err(|e| repo_error("get_question", e))?
        .ok_or_else(|| AppError::NotFound {
            message: format!("practice question {question_id} not found"),
        })
}

/// The column one editable field name maps to, and nothing else.
///
/// ## Why this exists as an exhaustive match
///
/// `set_field` interpolates the column into SQL, which is injection unless the
/// value can only be one of a few words THIS CRATE wrote. This is where a
/// client's string becomes a `&'static str` — and the `_` arm refuses rather
/// than guessing, so a field name nobody implemented is a 400 and never a
/// query.
fn column_for(field: &str) -> Option<&'static str> {
    match field {
        "text" => Some("text"),
        "watch_for" => Some("watch_for"),
        "stronger" => Some("stronger"),
        "follows" => Some("follows_key"),
        _ => None,
    }
}

/// The change kind an edit to one field records.
///
/// A re-wording of the question is its own kind because it is the one edit
/// Marie must re-read; everything else is `edited` with the field named.
fn kind_for(field: &str) -> &'static str {
    if field == "text" {
        "reworded"
    } else {
        "edited"
    }
}

/// The value a field currently holds, for the change row's `before`.
fn current(question: &PracticeQuestionRecord, field: &str) -> Option<String> {
    match field {
        "text" => Some(question.text.clone()),
        "watch_for" => question.watch_for.clone(),
        "stronger" => question.stronger.clone(),
        "follows" => question.follows_key.clone(),
        "tactic" => question.tactic.map(|t| t.to_string()),
        _ => None,
    }
}

/// Write one edited field, choosing the column safely.
///
/// Split from the handler so that function reads as the five steps it is —
/// fence, read, prove, write, record — and because the tactic arm is the only
/// place in this module where a client's string becomes a NUMBER, which is worth
/// having somewhere a reader can see all at once.
///
/// # Errors
/// 400 for a field this build cannot edit, or a tactic that is not a number.
async fn write_field(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    question_id: Uuid,
    field: &str,
    value: Option<&str>,
) -> Result<(), AppError> {
    if field == "tactic" {
        // The only editable column that is not TEXT. A value that will not parse
        // is a 400 rather than a silent clear: "tactic: banana" is a client bug,
        // and clearing the card it names would hide it.
        let tactic = match value {
            None => None,
            Some(raw) => Some(raw.parse::<i16>().map_err(|_| AppError::BadRequest {
                message: format!("tactic must be a card number from 1 to {TACTIC_CARD_MAX}"),
                details: serde_json::json!({ "field": "value", "value": raw }),
            })?),
        };
        return set_tactic(tx, question_id, tactic)
            .await
            .map_err(|e| repo_error("set_tactic", e));
    }
    let column = column_for(field).ok_or_else(|| AppError::BadRequest {
        message: "that field cannot be edited here".to_string(),
        details: serde_json::json!({ "field": "field", "value": field }),
    })?;
    set_field(tx, question_id, column, value)
        .await
        .map_err(|e| repo_error("set_field", e))
}

/// Edit one field on one question.
///
/// # Errors
/// 400 for an unsigned change, an unknown field, or a blank question text;
/// 404 when no question carries that id.
pub async fn post_edit_question(
    user: AuthUser,
    State(state): State<AppState>,
    Path(question_id): Path<Uuid>,
    Json(body): Json<EditQuestionRequest>,
) -> Result<Json<DeckChangeResponse>, AppError> {
    let (by_id, by) = attribution(&user);
    let question = require_question(&state, question_id).await?;

    // A blank optional field CLEARS it; a blank question text is refused. The
    // asymmetry is the point: a question with no words is not a question, while
    // a watch-for somebody decided was wrong should be removable.
    let value = body
        .value
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty());
    if body.field == "text" && value.is_none() {
        return Err(AppError::BadRequest {
            message: "a question must have words in it".to_string(),
            details: serde_json::json!({ "field": "value" }),
        });
    }

    commit_edit(&state, &question, &body, value, (&by_id, &by)).await?;

    tracing::info!(%question_id, field = %body.field, by = %by, "practice deck: a question was edited");
    Ok(Json(DeckChangeResponse { question_id }))
}

/// Write one field and log the change, in ONE transaction.
///
/// The sibling of [`commit_move`], and for the same reason: an edit that
/// changed the question and failed to log it would leave Marie reading new words
/// with nothing telling her they were new — which is the entire promise of the
/// `changed` badge. Both writes land or neither does.
///
/// `before` is read from the record the caller already fetched rather than
/// re-queried inside the transaction: it is the value the change row must
/// record as the OLD one, and re-reading it here would race with a second
/// editor's write and log a `before` that was never on screen.
///
/// # Errors
/// 500 when any of begin, write, log or commit fails; each names its own step.
async fn commit_edit(
    state: &AppState,
    question: &PracticeQuestionRecord,
    body: &EditQuestionRequest,
    value: Option<&str>,
    attribution: (&str, &str),
) -> Result<(), AppError> {
    let (by_id, by) = attribution;
    let before = current(question, &body.field);
    let mut tx = state
        .pipeline_pool
        .begin()
        .await
        .map_err(|e| repo_error("begin", e))?;

    write_field(&mut tx, question.id, &body.field, value).await?;

    log_change(
        &mut tx,
        &NewChange {
            scenario_id: question.scenario_id,
            question_id: question.id,
            change_kind: kind_for(&body.field),
            field: Some(&body.field),
            before_value: before.as_deref(),
            after_value: value,
            changed_by: by,
            changed_by_id: by_id,
        },
    )
    .await
    .map_err(|e| repo_error("log_change", e))?;

    tx.commit().await.map_err(|e| repo_error("commit", e))
}

/// The question one arrow would swap with, or `None` at the end of a side.
///
/// # Errors
/// 400 for a direction that is neither up nor down.
fn neighbour_of<'a>(
    deck: &'a [PracticeQuestionRecord],
    question: &PracticeQuestionRecord,
    direction: &str,
) -> Result<Option<&'a PracticeQuestionRecord>, AppError> {
    let side: Vec<&PracticeQuestionRecord> =
        deck.iter().filter(|q| q.side == question.side).collect();
    let Some(at) = side.iter().position(|q| q.id == question.id) else {
        return Ok(None);
    };
    let index = match direction {
        "up" => at.checked_sub(1),
        "down" => at.checked_add(1).filter(|n| *n < side.len()),
        other => {
            return Err(AppError::BadRequest {
                message: "direction must be up or down".to_string(),
                details: serde_json::json!({ "field": "direction", "value": other }),
            })
        }
    };
    Ok(index.and_then(|n| side.get(n)).copied())
}

/// Move one question up or down WITHIN ITS OWN SIDE.
///
/// ## Domain note: within its side, and why
///
/// The list Marie reads is filtered by who is asking, and the arrows are on that
/// list. An arrow that swapped a George question with the Chuck question above
/// it in `sort_order` would move a row she cannot see, and appear to do nothing.
/// So the neighbour is the next question on the SAME side — which is also what
/// keeps Mixed's pairs intact, because a redirect follows its trap by key rather
/// than by position.
pub async fn post_move_question(
    user: AuthUser,
    State(state): State<AppState>,
    Path(question_id): Path<Uuid>,
    Json(body): Json<MoveQuestionRequest>,
) -> Result<Json<DeckChangeResponse>, AppError> {
    let (by_id, by) = attribution(&user);
    let question = require_question(&state, question_id).await?;
    let scenario_id = question.scenario_id;

    let deck = list_deck(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| repo_error("list_deck", e))?;
    let Some(neighbour) = neighbour_of(&deck, &question, &body.direction)? else {
        // Already at its end of its side. A NO-OP that reports success: the
        // button did what it could, nothing moved, and no change row is written
        // because nothing changed. A 400 would put a failure notice in front of
        // somebody who pressed a control that was simply at its limit.
        return Ok(Json(DeckChangeResponse { question_id }));
    };

    commit_move(&state, &question, neighbour, (&by_id, &by)).await?;

    tracing::info!(%question_id, direction = %body.direction, by = %by, "practice deck: a question moved");
    Ok(Json(DeckChangeResponse { question_id }))
}

/// Swap two questions' places and log the change, in ONE transaction.
///
/// ## Why the swap and the log cannot be two calls
///
/// A move that swapped the rows and failed to log would leave the deck in an
/// order nobody can account for — and "Changed since your last sitting" would
/// tell Marie nothing had moved while the questions sat in a different order in
/// front of her. Both writes land or neither does.
///
/// ## Rust Learning: `&mut tx` passed to each write
///
/// `begin()` hands back a `Transaction`, and sqlx's executor trait is
/// implemented for `&mut Transaction` — so each write BORROWS the transaction
/// rather than consuming it, and the same one is still available afterwards to
/// `commit()`. Passing `tx` by value to the first write would move it, and the
/// second call would not compile: the borrow checker enforcing, at build time,
/// that a half-finished transaction cannot be used.
///
/// `attribution` arrives as a `(&str, &str)` pair — the id and the display name
/// — rather than as two positional parameters, so the two cannot be swapped at a
/// call site without the types complaining.
///
/// # Errors
/// 500 when any of begin, swap, log or commit fails; each names its own step.
async fn commit_move(
    state: &AppState,
    question: &PracticeQuestionRecord,
    neighbour: &PracticeQuestionRecord,
    attribution: (&str, &str),
) -> Result<(), AppError> {
    let (by_id, by) = attribution;
    let mut tx = state
        .pipeline_pool
        .begin()
        .await
        .map_err(|e| repo_error("begin", e))?;
    swap_sort_order(&mut tx, question.id, neighbour.id)
        .await
        .map_err(|e| repo_error("swap_sort_order", e))?;
    log_change(
        &mut tx,
        &NewChange {
            scenario_id: question.scenario_id,
            question_id: question.id,
            change_kind: "moved",
            field: None,
            before_value: Some(&question.sort_order.to_string()),
            after_value: Some(&neighbour.sort_order.to_string()),
            changed_by: by,
            changed_by_id: by_id,
        },
    )
    .await
    .map_err(|e| repo_error("log_change", e))?;
    tx.commit().await.map_err(|e| repo_error("commit", e))
}

/// Hide one question, or put it back. Never a delete.
pub async fn post_hide_question(
    user: AuthUser,
    State(state): State<AppState>,
    Path(question_id): Path<Uuid>,
    Json(body): Json<HideQuestionRequest>,
) -> Result<Json<DeckChangeResponse>, AppError> {
    let (by_id, by) = attribution(&user);
    let question = require_question(&state, question_id).await?;

    let mut tx = state
        .pipeline_pool
        .begin()
        .await
        .map_err(|e| repo_error("begin", e))?;
    set_hidden(&mut tx, question_id, body.hidden.then_some(by.as_str()))
        .await
        .map_err(|e| repo_error("set_hidden", e))?;
    log_change(
        &mut tx,
        &NewChange {
            scenario_id: question.scenario_id,
            question_id,
            change_kind: if body.hidden { "hidden" } else { "unhidden" },
            field: None,
            before_value: None,
            after_value: None,
            changed_by: &by,
            changed_by_id: &by_id,
        },
    )
    .await
    .map_err(|e| repo_error("log_change", e))?;
    tx.commit().await.map_err(|e| repo_error("commit", e))?;

    tracing::info!(%question_id, hidden = body.hidden, by = %by, "practice deck: a question was hidden or restored");
    Ok(Json(DeckChangeResponse { question_id }))
}
