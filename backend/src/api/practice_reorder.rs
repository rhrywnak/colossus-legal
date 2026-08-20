//! Placing one question where a drag dropped it.
//!
//! The sibling of `practice_editor::post_move_question`, and deliberately a
//! separate handler: the arrows move one step and swap two rows, a drop names a
//! position and re-sequences a side. See `ReorderQuestionRequest`'s note.
//!
//! Everything else is shared with the arrows — the same `sort_order` column, the
//! same `moved` change row, the same attribution from the session. Chuck's
//! "Changed since your last sitting" box cannot tell which control did it, and
//! should not: what changed is that the question moved.

use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    dto::practice_review::{DeckChangeResponse, ReorderQuestionRequest},
    error::AppError,
    repositories::pipeline_repository::practice::{list_deck, PracticeQuestionRecord},
    repositories::pipeline_repository::practice_editor::{log_change, NewChange},
    repositories::pipeline_repository::practice_reorder::{resequenced, write_order},
    services::practice_notes::attribution,
    state::AppState,
};

use super::practice::repo_error;
use super::practice_editor::require_question;

/// `POST /api/practice/questions/:id/reorder`
///
/// ## Why a drop that names no position returns 200 and not 400
///
/// Dropping a row onto itself, or onto something outside its side, is a gesture
/// a person makes by accident several times an hour — starting a drag and
/// changing their mind. The deck is unchanged and the screen re-reads it, which
/// is exactly what the ▲▼ arrows do at the end of a side. A red notice for
/// having changed one's mind mid-drag would be the screen telling somebody off.
///
/// The distinction is still observable: the log line carries `moved = false`, so
/// a drag that silently did nothing when it should have moved something is
/// findable without reproducing it.
///
/// ## Why the handler carries a span
///
/// Every failure here goes through `repo_error`, which logs the STEP that broke
/// (`list_deck`, `begin`, `write_order`, `log_change`, `commit`) and the
/// underlying error — but not which question was being dragged, because it does
/// not know. `#[tracing::instrument]` puts `question_id` on the span, so every
/// event inside inherits it: an operator reading "write_order failed" can tell
/// WHICH drag failed without reproducing it.
///
/// `skip` on `state` and `user`: one holds connection pools and the other a
/// user's groups and email, and neither belongs in a log line.
///
/// # Errors
/// 404 when no question carries that id. 500 when a read or a write fails — the
/// response body is deliberately opaque, as every sibling handler's is, and the
/// STEP that broke (`list_deck`, `begin`, `write_order`, `log_change`, `commit`)
/// is named in the trace log beside the `question_id` the span carries. A 500
/// body that named an internal step would tell a witness something only an
/// operator can act on.
#[tracing::instrument(
    skip(state, user, body),
    fields(question_id = %question_id, before = ?body.before)
)]
pub async fn post_reorder_question(
    user: AuthUser,
    State(state): State<AppState>,
    Path(question_id): Path<Uuid>,
    Json(body): Json<ReorderQuestionRequest>,
) -> Result<Json<DeckChangeResponse>, AppError> {
    // Signed from the session, like every deck write since the 08-19 hotfix
    // removed the "Who is editing?" picker. `attribution` returns the stable
    // username and the display name a screen prints: the id survives a rename,
    // the name is what the change log shows beside the row.
    let (by_id, by) = attribution(&user);
    let question = require_question(&state, question_id).await?;
    let deck = list_deck(&state.pipeline_pool, question.scenario_id)
        .await
        .map_err(|e| repo_error("list_deck", e))?;

    let Some(order) = resequenced(&deck, question_id, body.before) else {
        tracing::info!(moved = false, "practice deck: a drop named no position");
        return Ok(Json(DeckChangeResponse { question_id }));
    };

    // `resequenced` inserts the dragged id before returning `Some`, so it IS in
    // `order`. A `.unwrap_or(0)` here would be the wrong tool for an impossible
    // case: if a future edit to `resequenced` ever broke that invariant, this
    // would write `position = 0` into the change log — a record saying the
    // question moved to the top when it did not — with nothing anywhere saying
    // so. Standing Rule 1: if it can fail, the failure is observable.
    let position = commit_order(&state, &question, &order, (&by_id, &by)).await?;

    tracing::info!(moved = true, position, by = %by, "practice deck: a question was dragged");
    Ok(Json(DeckChangeResponse { question_id }))
}

/// Write the new order and log the change, in ONE transaction.
///
/// Returns the dragged question's new position, for the caller's log line.
///
/// Split from the handler so that function stays the four steps it reads as —
/// sign, read, decide, write — and because the invariant check below deserves to
/// sit beside the write it protects rather than in the middle of a route.
///
/// `attribution` is the `(id, name)` pair from the session — the stable username
/// and the display name the change log prints beside the row.
///
/// ## Rust Learning: `&mut tx` borrowed by each write
///
/// `begin()` returns a `Transaction`, and sqlx implements its executor trait for
/// `&mut Transaction` — so each write BORROWS it rather than consuming it, and
/// the same transaction is still there to `commit()`. Passing it by value to the
/// first write would move it and the second call would not compile: the borrow
/// checker enforcing at build time that a half-finished transaction cannot be
/// used.
///
/// # Errors
/// 500 when the order does not contain the dragged question (an internal
/// invariant), or when any of begin, write, log or commit fails.
async fn commit_order(
    state: &AppState,
    question: &PracticeQuestionRecord,
    order: &[Uuid],
    attribution: (&str, &str),
) -> Result<usize, AppError> {
    let (by_id, by) = attribution;
    let question_id = question.id;
    let Some(position) = order.iter().position(|id| *id == question_id) else {
        tracing::error!(
            "practice deck: resequenced returned an order without the dragged question — \
             an internal invariant in practice_reorder::resequenced is broken"
        );
        return Err(AppError::Internal {
            message: "the question's new position could not be determined; nothing was changed"
                .to_string(),
        });
    };
    let mut tx = state
        .pipeline_pool
        .begin()
        .await
        .map_err(|e| repo_error("begin", e))?;
    write_order(&mut tx, order)
        .await
        .map_err(|e| repo_error("write_order", e))?;
    log_change(
        &mut tx,
        &NewChange {
            scenario_id: question.scenario_id,
            question_id,
            // The same word the arrows write. Chuck's box reads the KIND, and a
            // second word for the same fact would split one question's history
            // into two stories depending on which control was used.
            change_kind: "moved",
            field: None,
            before_value: Some(&question.sort_order.to_string()),
            after_value: Some(&position.to_string()),
            changed_by: by,
            changed_by_id: by_id,
        },
    )
    .await
    .map_err(|e| repo_error("log_change", e))?;
    tx.commit().await.map_err(|e| repo_error("commit", e))?;
    Ok(position)
}
