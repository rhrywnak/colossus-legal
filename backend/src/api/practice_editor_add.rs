//! Adding a question by hand, and proving one before it is written.
//!
//! Split from [`super::practice_editor`] on 2026-08-19 when Part B carried that
//! module past Rule 17's limit. The seam is honest as well as arithmetical: the
//! sibling CHANGES questions that already exist and every one of its three
//! writes is a two-line edit plus its change row, while adding one is mostly
//! REFUSAL — nine ways a typed question can be wrong, all of them proved before
//! a transaction opens.
//!
//! ## The REFUSALS moved out (2026-09-10)
//!
//! This module's own header has always said that adding a question is mostly
//! refusal — nine ways a typed one can be wrong, all proved before a transaction
//! opens — and task DECK_DRAG_AND_ADD added a tenth and an eleventh (an `after`
//! naming another side's row, and `after` and `at_start` together), which carried
//! the file past Rule 17's limit. So the refusals are now the sibling
//! [`super::practice_editor_add_fences`], along with the `AddPlan` they produce,
//! and what is left here is the WRITE: insert, place, log, commit.
//!
//! The seam is the one the header already drew. Nothing changed but where the
//! functions live.
//!
//! ## CRITICAL — the pipeline pool
//!
//! Every table here lives in `colossus_legal_v2`.

use axum::{
    extract::{Path, State},
    Json,
};

use crate::{
    auth::AuthUser,
    dto::practice_review::{AddQuestionRequest, DeckChangeResponse},
    error::AppError,
    repositories::pipeline_repository::{
        practice::{list_deck, PracticeQuestionRecord},
        practice_editor::{insert_question, log_change, next_sort_order, NewChange, NewQuestion},
        practice_reorder::{placed_after, position_within_side, write_order, NewPosition},
    },
    services::practice_notes::attribution,
    state::AppState,
};

use super::practice::repo_error;
use super::practice_editor_add_fences::{fence_after, plan_question, AddPlan};
use super::scenario_facts::{ensure_scenario_in_case, parse_scenario_id};

/// Add a question somebody typed on the page.
///
/// ## Where the new question lands (task DECK_DRAG_AND_ADD part 3)
///
/// Without `after` it appends, as it always has. With `after` it is written at
/// the end exactly as before and then MOVED, in the same transaction, to sit
/// immediately below that row on its own side — see [`write_question`] for why
/// the two are one act rather than an insert followed by a reorder call.
///
/// # Errors
/// 400 for an unsigned change, an unknown kind, a blank text, a redirect with
/// no `follows`, a `follows` naming no cross question in this deck, or an
/// `after` naming a question on the OTHER side; 404 when the scenario does not
/// exist, is reached through the wrong case, or when `after` names a question
/// this scenario does not hold.
pub async fn post_add_question(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
    Json(body): Json<AddQuestionRequest>,
) -> Result<Json<DeckChangeResponse>, AppError> {
    let scenario_id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, scenario_id, &slug).await?;
    let (by_id, by) = attribution(&user);

    let deck = list_deck(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| repo_error("list_deck", e))?;
    let plan = plan_question(&state, &body, &deck)?;

    // Fenced BEFORE the transaction opens, like every other refusal in this
    // module: a request that names an impossible position is answered without a
    // row having been written and then moved back.
    let at = fence_after(&body, &plan, &deck, scenario_id)?;

    let question_id =
        write_question(&state, scenario_id, &body, &plan, &deck, at, (&by, &by_id)).await?;

    tracing::info!(%scenario_id, %question_id, kind = %plan.kind, ?at, by = %by, "practice deck: a question was added");
    Ok(Json(DeckChangeResponse { question_id }))
}

/// Write the question and its `added` change row, in one transaction.
///
/// Split from the handler so that function is the four steps it reads as —
/// fence the case, fence the editor, plan, write — and because the two writes
/// are ONE act: a question nobody can see the arrival of is a question Marie is
/// asked with no explanation of where it came from.
async fn write_question(
    state: &AppState,
    scenario_id: uuid::Uuid,
    body: &AddQuestionRequest,
    plan: &AddPlan,
    deck: &[PracticeQuestionRecord],
    at: NewPosition,
    attribution: (&str, &str),
) -> Result<uuid::Uuid, AppError> {
    let (by, by_id) = attribution;
    let mut tx = state
        .pipeline_pool
        .begin()
        .await
        .map_err(|e| repo_error("begin", e))?;
    let sort_order = next_sort_order(&mut tx, scenario_id)
        .await
        .map_err(|e| repo_error("next_sort_order", e))?;
    let question_id = insert_question(
        &mut tx,
        &NewQuestion {
            scenario_id,
            side: plan.side,
            kind: plan.kind,
            text: body.text.trim(),
            tactic: plan.tactic,
            follows_key: plan.follows.as_deref(),
            watch_for: body
                .watch_for
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty()),
            source_kind: plan.source_kind,
            source_ref: plan.source_ref.as_deref(),
            receipt: None,
            sort_order,
            created_by: by,
        },
    )
    .await
    .map_err(|e| repo_error("insert_question", e))?;

    // The move, inside the SAME transaction as the insert. A question that
    // arrived at the bottom and then hopped to the middle a moment later would be
    // two facts on Chuck's screen for one thing he did, and a failure between
    // them would leave the row somewhere he did not ask for with nothing saying
    // so. `placed_after` returns `None` only for positions `fence_after` has
    // already refused, so reaching it here would be a broken invariant rather
    // than a request problem — and it is reported as one.
    // `End` is where the INSERT already put it, so there is nothing to write and
    // nothing to say — which keeps every add that predates the gap control on
    // exactly the one-statement path it has always taken.
    let position = match at {
        NewPosition::End => None,
        at => Some(place_new_question(&mut tx, deck, question_id, plan, at).await?),
    };
    let placement = match position {
        // The stored value counts from ONE, because it is read by a person: the
        // deck's third question is "3" on screen and in Chuck's box, never "2".
        Some(at) => format!("{} {}", plan.side, at + 1),
        None => plan.side.to_string(),
    };

    log_change(
        &mut tx,
        &NewChange {
            scenario_id,
            question_id,
            change_kind: "added",
            field: None,
            before_value: None,
            // The SIDE, which is the one fact about a new question the change
            // list needs — and it is stored rather than joined because the
            // question's row may have moved by the time the list is read.
            //
            // Since task DECK_DRAG_AND_ADD it carries the POSITION too, when the
            // add named one: "chuck 3" rather than "chuck". A question added into
            // the middle of a deck is a different event from one appended to the
            // end, and the box that tells Chuck what changed since his last
            // sitting could not tell them apart. Appended adds are unchanged —
            // they say the side alone, exactly as every stored row already does.
            after_value: Some(&placement),
            changed_by: by,
            changed_by_id: by_id,
        },
    )
    .await
    .map_err(|e| repo_error("log_change", e))?;
    tx.commit().await.map_err(|e| repo_error("commit", e))?;
    Ok(question_id)
}

/// Move the row that was just inserted to the position that was asked for.
///
/// Returns its new position WITHIN ITS SIDE, counting from zero — the number the
/// change row prints (incremented, because a person counts from one).
///
/// ## Why the whole deck is rewritten for one new row
///
/// `write_order` renumbers every id it is handed to `0..N-1`, and
/// `practice_questions_order_unique` spans the scenario — so a partial list
/// collides with the rows left out of it. That is the same 500 that made the
/// drag never save; see `practice_reorder`'s module doc. `placed_after` therefore
/// returns a permutation of the deck plus the new row, and all of it is written.
///
/// ## Rust Learning: `ok_or_else` turning a broken invariant into an error
///
/// `placed_after` answers `None` only for the two positions [`fence_after`] has
/// already refused, so reaching that arm means the two functions have drifted
/// apart. It is reported as a 500 with a log line naming the drift rather than
/// silently leaving the question at the bottom of the deck — which would be a row
/// sitting somewhere nobody asked for, with nothing anywhere saying so
/// (Standing Rule 1).
async fn place_new_question(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    deck: &[PracticeQuestionRecord],
    question_id: uuid::Uuid,
    plan: &AddPlan,
    at: NewPosition,
) -> Result<usize, AppError> {
    let order = placed_after(deck, question_id, plan.side, at).ok_or_else(|| {
        tracing::error!(
            %question_id, ?at, side = plan.side,
            "practice deck: fence_after passed a position placed_after refuses — \
             an internal invariant between the two is broken"
        );
        AppError::Internal {
            message: "the new question's position could not be determined; nothing was changed"
                .to_string(),
        }
    })?;

    write_order(tx, &order)
        .await
        .map_err(|e| repo_error("write_order", e))?;

    position_within_side(deck, &order, question_id, plan.side).ok_or_else(|| {
        tracing::error!(
            %question_id, side = plan.side,
            "practice deck: the new question is missing from the order that was just written"
        );
        AppError::Internal {
            message: "the new question's position could not be determined; nothing was changed"
                .to_string(),
        }
    })
}

#[cfg(test)]
#[path = "practice_editor_add_tests.rs"]
mod tests;
