//! Notes, routed at last — writing one, and striking one through.
//!
//! CC_TASK_REVIEW_LOOP_v1 §4. Thin handlers over the repository built on
//! 2026-08-19 (`pipeline_repository::practice_notes`):
//!
//! - `POST /practice/answers/:answer_id/notes`     → a note on one attempt
//! - `POST /practice/questions/:question_id/notes` → a note on the question
//! - `PUT  /practice/notes/:note_id/strike`        → withdraw a note
//!
//! Scenario-level notes wait (task §4).
//!
//! ## Domain note: why a note matters to the War Room
//!
//! An UNSTRUCK note on a question, or on its current answer, counts toward
//! Marie's "new or changed" pill until she answers that question again
//! (`war_room_status::changed_counts`). Striking a note withdraws it from that
//! count. So these three routes are how Chuck's side of the loop reaches Marie.
//!
//! ## Why the answer and question routes are not case-scoped
//!
//! The same reason every `/practice/questions/:id/…` and `/practice/answers/:id/…`
//! route gives (`api::practice`): the ids are server-minted and unguessable, and
//! the WHERE of the note is read from the target row itself — never taken from a
//! body that could disagree with the path.
//!
//! The routes are declared in `practice::routes`, beside every other practice path.

use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    dto::practice_review::{NoteTextRequest, PracticeNoteDto},
    error::AppError,
    repositories::pipeline_repository::practice_notes::{
        answer_home, insert_note, note_by_id, strike_note, NewNote, NoteRecord,
    },
    services::{practice_note_view::note_dto, practice_notes::attribution},
    state::AppState,
};

use super::practice::repo_error;
use super::practice_editor::require_question;

/// Write a note on one attempt at a question.
///
/// # Errors
/// 400 for a blank note; 404 when no answer carries that id; 500 (logged) for a
/// failed read or write.
pub async fn post_answer_note(
    user: AuthUser,
    State(state): State<AppState>,
    Path(answer_id): Path<Uuid>,
    Json(body): Json<NoteTextRequest>,
) -> Result<Json<PracticeNoteDto>, AppError> {
    let text = note_text(&body.text)?;
    let (scenario_id, question_id) = answer_home(&state.pipeline_pool, answer_id)
        .await
        .map_err(|e| repo_error("answer_home", format!("answer {answer_id}: {e}")))?
        .ok_or_else(|| AppError::NotFound {
            message: format!("practice answer {answer_id} not found"),
        })?;
    let target = NoteTarget {
        scenario_id,
        question_id,
        answer_id: Some(answer_id),
    };
    write_note(&state, &user, target, text).await
}

/// Write a note on a question, rather than on one attempt at it.
///
/// # Errors
/// 400 for a blank note; 404 when no question carries that id; 500 (logged).
pub async fn post_question_note(
    user: AuthUser,
    State(state): State<AppState>,
    Path(question_id): Path<Uuid>,
    Json(body): Json<NoteTextRequest>,
) -> Result<Json<PracticeNoteDto>, AppError> {
    let text = note_text(&body.text)?;
    let question = require_question(&state, question_id).await?;
    let target = NoteTarget {
        scenario_id: question.scenario_id,
        question_id,
        answer_id: None,
    };
    write_note(&state, &user, target, text).await
}

/// Strike a note through. Idempotent: a second strike keeps the first moment.
///
/// Returns the note as it now stands, so the screen redraws from the store's
/// word rather than from a guess.
///
/// # Errors
/// 404 when no note carries that id; 500 (logged).
pub async fn put_strike_note(
    user: AuthUser,
    State(state): State<AppState>,
    Path(note_id): Path<Uuid>,
) -> Result<Json<PracticeNoteDto>, AppError> {
    let (by_id, _) = attribution(&user);
    let exists = strike_note(&state.pipeline_pool, note_id, &by_id)
        .await
        .map_err(|e| repo_error("strike_note", format!("note {note_id} by {by_id}: {e}")))?;
    if !exists {
        return Err(AppError::NotFound {
            message: format!("practice note {note_id} not found"),
        });
    }
    let stored = read_back(&state, note_id).await?;
    tracing::info!(%note_id, by = %by_id, "practice: a note was struck");
    Ok(Json(note_dto(&state.settings.current(), &stored)))
}

/// Where a note lands, read from the target row — never from the request body.
#[derive(Debug, Clone, Copy)]
struct NoteTarget {
    scenario_id: Uuid,
    question_id: Uuid,
    answer_id: Option<Uuid>,
}

/// Insert the note, attributed to the login, and hand back what was stored.
async fn write_note(
    state: &AppState,
    user: &AuthUser,
    target: NoteTarget,
    text: &str,
) -> Result<Json<PracticeNoteDto>, AppError> {
    let (author_id, author) = attribution(user);
    let note_id = insert_note(
        &state.pipeline_pool,
        &NewNote {
            scenario_id: target.scenario_id,
            question_id: Some(target.question_id),
            answer_id: target.answer_id,
            author: &author,
            author_id: &author_id,
            text,
        },
    )
    .await
    .map_err(|e| {
        repo_error(
            "insert_note",
            format!(
                "scenario {} question {} answer {:?} by {author_id}: {e}",
                target.scenario_id, target.question_id, target.answer_id
            ),
        )
    })?;
    let stored = read_back(state, note_id).await?;
    tracing::info!(
        %note_id,
        scenario_id = %target.scenario_id,
        question_id = %target.question_id,
        on_answer = target.answer_id.is_some(),
        by = %author_id,
        "practice: a note was written"
    );
    Ok(Json(note_dto(&state.settings.current(), &stored)))
}

/// Read a note just written or struck. Missing here is a defect, not a 404.
async fn read_back(state: &AppState, note_id: Uuid) -> Result<NoteRecord, AppError> {
    note_by_id(&state.pipeline_pool, note_id)
        .await
        .map_err(|e| repo_error("note_by_id", format!("note {note_id}: {e}")))?
        .ok_or_else(|| {
            tracing::error!(%note_id, "practice: a note vanished between its write and its read-back");
            AppError::Internal {
                message: "the note was written but could not be read back".to_string(),
            }
        })
}

/// The note's text, trimmed — or a 400 when nothing is left.
///
/// ## Why whitespace is BLANK
///
/// The table's CHECK (`btrim(text) <> ''`) would refuse it anyway, but as a 500
/// from a constraint violation. Refusing here names the problem as the caller's,
/// which is what it is.
pub(super) fn note_text(text: &str) -> Result<&str, AppError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(AppError::BadRequest {
            message: "a note needs some text".to_string(),
            details: serde_json::Value::Null,
        });
    }
    Ok(trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A blank note is refused by the handler, not by the database.
    #[test]
    fn a_blank_note_is_a_400() {
        assert!(matches!(
            note_text("   \n"),
            Err(AppError::BadRequest { .. })
        ));
        assert!(matches!(note_text(""), Err(AppError::BadRequest { .. })));
    }

    /// The stored text is trimmed — the words, not the padding around them.
    #[test]
    fn note_text_is_trimmed() {
        assert_eq!(note_text("  hold the $750  ").ok(), Some("hold the $750"));
    }
}
