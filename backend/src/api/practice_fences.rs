//! The two refusals that stand between a keypress and an answer row.
//!
//! Split from [`super::practice_answers`] on 2026-08-19 when the hotfix carried
//! that module past Rule 17's limit. The seam is honest as well as arithmetical:
//! everything there RECORDS something, and both of these exist to stop a row
//! being recorded — one because there is nothing in it, one because it is
//! already there.
//!
//! ## CRITICAL — the pipeline pool
//!
//! Every table here lives in `colossus_legal_v2`.

use std::collections::HashSet;

use uuid::Uuid;

use serde_json::json;

use crate::{
    dto::practice::{AnswerRequest, StartSessionRequest},
    error::AppError,
    repositories::pipeline_repository::practice::list_deck,
    repositories::pipeline_repository::practice_flow::answer_count_for_scenario,
    state::AppState,
};

use super::practice::repo_error;

/// Refuse an answer with nothing in it.
///
/// ## Why this is a refusal and not a stored empty string
///
/// v0 stored `""` deliberately, so a blank answer and an unanswered question
/// stayed different rows. That was right when nothing stopped her pressing
/// Answer on an empty box — the row was the record that she had. It is wrong now
/// that the button is disabled until she types: the only way to reach here empty
/// is a client that ignored the disable, and writing the row would put a blank
/// line on Chuck's sheet under her name.
///
/// `dont_recall` is exempt: "I don't recall." is a COMPLETE answer, it stays one
/// click, and it arrives carrying the stored sentence rather than her typing.
///
/// # Errors
/// 400 naming the field.
pub(super) fn fence_answer_text(body: &AnswerRequest) -> Result<(), AppError> {
    if !body.dont_recall && body.answer_text.trim().is_empty() {
        // WARN and not silence: the button is disabled until she types, so every
        // firing of this fence is anomalous — a stale tab, a probing client, or
        // the disable having regressed. Without a line here an operator cannot
        // tell "nobody hit this today" from "somebody hit it forty times", and
        // the second of those is the frontend breaking.
        tracing::warn!(
            session_id = %body.session_id,
            question_id = %body.question_id,
            "practice: refused an empty answer — the disabled Answer button was bypassed"
        );
        return Err(AppError::BadRequest {
            message: "an answer needs words in it, or press \"I don't recall.\"".to_string(),
            details: serde_json::json!({ "field": "answer_text" }),
        });
    }
    Ok(())
}

/// Refuse to delete a scenario Marie has practised.
///
/// ## What happened before this, and why it was worse than either option
///
/// `practice_questions.scenario_id` cascades from `scenarios`, and
/// `practice_answers.question_id` is `ON DELETE RESTRICT`. So deleting a
/// practised scenario hit the RESTRICT and 500'd with "failed to delete
/// scenario" — UNLESS Postgres happened to run the other cascade path first
/// (`scenarios` → `practice_sessions` → `practice_answers`), in which case the
/// answers were gone before the RESTRICT was checked and the delete SUCCEEDED,
/// silently destroying every Chuck's sheet for that scenario.
///
/// Which of the two happened was not something the code decided. That is the
/// whole reason this exists: a refusal that names the count is a decision, and
/// what was there before was a coin toss between an unexplained 500 and losing
/// a witness's recorded testimony.
///
/// # Errors
/// 409 naming the number of answers, so the person who pressed Delete knows what
/// is in the way and roughly how much of it.
pub async fn refuse_if_practised(state: &AppState, scenario_id: Uuid) -> Result<(), AppError> {
    let answers = answer_count_for_scenario(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, %scenario_id, "failed to count practice answers");
            AppError::Internal {
                message: "could not check whether this scenario has been practised".to_string(),
            }
        })?;
    if answers > 0 {
        tracing::warn!(
            %scenario_id, answers,
            "practice: refused to delete a scenario that has been practised"
        );
        return Err(AppError::Conflict {
            message: format!(
                "this scenario has {answers} recorded practice answers — delete would take \
                 Chuck's sheets with it. Hide the questions instead, or clear the practice \
                 log first"
            ),
            details: json!({ "practice_answers": answers }),
        });
    }
    Ok(())
}

/// Refuse a sitting whose side is unknown or whose questions are not this
/// scenario's.
///
/// Split from the handler so that function stays the four steps it reads as
/// (fence the case, check the sitting, store it, say so) — and because these are
/// the two refusals that are about what a CLIENT sent, which is a different
/// subject from recording a sitting.
///
/// # Errors
/// 400 with the offending field and value, both times.
pub async fn check_sitting(
    state: &AppState,
    scenario_id: Uuid,
    body: &StartSessionRequest,
) -> Result<(), AppError> {
    // The column has a CHECK, but a CHECK violation is a 500 with a constraint
    // name in it. Refusing here makes a bad `who` a 400 that says which values
    // exist — the difference between a client bug found in review and one found
    // in a log at midnight.
    fence_who(&body.who)?;

    // FENCE: every id the browser sent must belong to THIS scenario's deck.
    //
    // The queue is composed on screen, so without this a client could open a
    // sitting whose queue named another scenario's questions — and Chuck's
    // sheet would then carry a question Marie was never asked, with nothing on
    // the page looking wrong. Same reasoning as `fence_answer`, applied at the
    // moment the sitting is recorded rather than one answer at a time.
    let deck = list_deck(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| repo_error("list_deck", e))?;
    let known: HashSet<Uuid> = deck.iter().map(|q| q.id).collect();
    if let Some(stray) = fence_queue(&body.queue, &body.skipped_today, &known) {
        return Err(AppError::BadRequest {
            message: "every question in the sitting must be in this scenario's deck".to_string(),
            details: serde_json::json!({ "field": "queue", "value": stray.to_string() }),
        });
    }
    Ok(())
}

/// Refuse a sitting whose side is not one of the three the column permits.
///
/// Split out of [`check_sitting`] so it can be tested without a pool: the check
/// is a `matches!` over three literals and the reason it exists is worth
/// pinning, but the function around it needs a database to reach it.
///
/// ## Rust Learning: `matches!`
///
/// `matches!(value, PATTERN)` is a one-expression `match` returning a bool. It
/// takes PATTERNS, not values, which is why the alternatives are written with
/// `|` rather than compared with `==` — and why a fourth permitted side must be
/// added here and to the CHECK constraint together.
///
/// # Errors
/// 400 naming the field and the offending value.
pub(super) fn fence_who(who: &str) -> Result<(), AppError> {
    // The column has a CHECK, but a CHECK violation is a 500 with a constraint
    // name in it. Refusing here makes a bad `who` a 400 that says which values
    // exist — the difference between a client bug found in review and one found
    // in a log at midnight.
    if !matches!(who, "george" | "chuck" | "mixed") {
        return Err(AppError::BadRequest {
            message: "who must be george, chuck or mixed".to_string(),
            details: serde_json::json!({ "field": "who", "value": who }),
        });
    }
    Ok(())
}

/// The first id in the sitting that this scenario's deck does not contain.
///
/// ## Why this fence exists
///
/// The queue and today's skips are both composed in the BROWSER — the order is
/// the drill, and the screen is what knows it. That makes them client input.
/// Without this check a sitting could be opened whose queue named another
/// scenario's questions, and Chuck's sheet would then carry a question Marie was
/// never asked, with nothing on the page looking wrong. Same reasoning as
/// [`super::practice_answers`]'s per-answer fence, applied once at the moment
/// the sitting is recorded.
///
/// `skipped_today` is fenced too, and deliberately: it is written to the row as
/// the record of what she was offered, so a foreign id there is a lie in the
/// record even though it deals no question.
///
/// Returns `None` when everything belongs — which is also the answer for an
/// empty sitting, because a sitting that deals nothing names nothing foreign.
pub(super) fn fence_queue<'a>(
    queue: &'a [Uuid],
    skipped_today: &'a [Uuid],
    known: &HashSet<Uuid>,
) -> Option<&'a Uuid> {
    queue
        .iter()
        .chain(skipped_today.iter())
        .find(|id| !known.contains(id))
}

#[cfg(test)]
#[path = "practice_fences_tests.rs"]
mod tests;
