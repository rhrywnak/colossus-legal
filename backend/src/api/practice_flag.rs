//! Marie's flag on one QUESTION: storing it, and clearing it.
//!
//! Split from [`super::practice_answers`] in T1 (2026-08-20), under Rule 17.
//!
//! ## Why this seam and not another
//!
//! The sibling module's own header says what it is: *"the whole of what happens
//! between 'Answer' and 'Got it'"* — the acts that record one answer. A flag is
//! not one of them. It addresses a **question**, it survives every sitting, it is
//! written from the start screen as readily as from a reveal, and it is read by
//! Chuck at the foot of his sheet rather than by anything on the answer path. It
//! has been filed with the answers since the August split; T1 needed the lines
//! back and this is the cut that was already there to be made.
//!
//! The routes are still declared in one place (`practice::routes`), because a
//! route table split across three files is how a path stops being served by
//! anything.

use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    dto::practice::{FlagRequest, FlagResponse},
    error::AppError,
    repositories::pipeline_repository::practice_flow::set_flag,
    state::AppState,
};

use super::practice::repo_error;

/// Store — or clear — Marie's flag on one question.
///
/// ## Why a blank note CLEARS rather than 400s
///
/// The screen has ONE control for both acts: she opens the note, empties it, and
/// saves. A refusal there would leave her looking at a flag she has just decided
/// is wrong with no way to remove it, and a second "unflag" endpoint would be a
/// second way to say the same thing — two routes to keep in step for one act.
///
/// The note is trimmed, and a note that is nothing but whitespace IS blank: a
/// flag reading `" "` prints as an empty complaint on Chuck's sheet.
///
/// # Errors
/// 404 when no question carries that id — never a silent success for a write
/// that touched no row.
pub async fn put_question_flag(
    user: AuthUser,
    State(state): State<AppState>,
    Path(question_id): Path<Uuid>,
    Json(body): Json<FlagRequest>,
) -> Result<Json<FlagResponse>, AppError> {
    let stored = normalize_flag_note(body.note);

    let touched = set_flag(
        &state.pipeline_pool,
        question_id,
        stored.as_deref(),
        &user.username,
    )
    .await
    .map_err(|e| repo_error("set_flag", e))?;

    if !touched {
        return Err(AppError::NotFound {
            message: format!("practice question {question_id} not found"),
        });
    }

    tracing::info!(
        %question_id,
        user = %user.username,
        cleared = stored.is_none(),
        "practice: flag written"
    );
    Ok(Json(FlagResponse { flag_note: stored }))
}

/// What a submitted note becomes: `None` to clear, or the trimmed line.
///
/// ## Why whitespace is BLANK and not a note
///
/// A flag reading `" "` prints as an empty complaint at the foot of Chuck's
/// sheet — a row saying Marie objected to a question, with nothing where the
/// objection should be. Trimming to nothing and clearing is the honest reading
/// of an empty box.
pub(super) fn normalize_flag_note(note: Option<String>) -> Option<String> {
    let note = note?;
    let trimmed = note.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}
