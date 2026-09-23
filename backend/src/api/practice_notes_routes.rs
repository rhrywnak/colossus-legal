//! Notes, routed at last — writing one, and striking one through.
//!
//! CC_TASK_REVIEW_LOOP_v1 §4. Thin handlers over the repository built on
//! 2026-08-19 (`pipeline_repository::practice_notes`):
//!
//! - `POST /practice/answers/:answer_id/notes`     → a note on one attempt
//! - `POST /practice/questions/:question_id/notes` → a note on the question
//! - `POST /practice/notes/:note_id/reply`         → answer a note (L3)
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
        answer_home, insert_note, note_by_id, note_scenario, strike_note, NewNote, NoteRecord,
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
    write_note(&state, &user, target, text, None).await
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
    write_note(&state, &user, target, text, None).await
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

/// Answer one note, in a note of your own (CC_TASK_FOR_YOU_v1 L3).
///
/// ## Domain note: a reply IS a note
///
/// One table, one kind of thing. The reply lands on the same question and the
/// same attempt as the note it answers, it waits for the other side like any
/// other item, and it clears by being read. What makes it a reply is one
/// column, `answers_note_id`, which the exchange is drawn from.
///
/// ## Why the WHERE is read from the parent
///
/// The same rule the other two note routes follow: the scenario, the question
/// and the attempt are taken from the row being answered, never from a body
/// that could disagree with the path. A caller cannot file a reply onto some
/// other question by asking for it.
///
/// ## Why a reply to a reply attaches to the ROOT
///
/// The mockup's exchange is two lines deep — a note, and answers under it —
/// and the panel draws exactly that. Left to nest, a third message would be
/// drawn under a reply and the fourth under that, until the panel is a thread
/// nobody designed. Flattening to the root keeps every answer under the line
/// that started it, which is what a person means by "this conversation".
///
/// # Errors
/// 400 for a blank reply, or for a reply to a note that was withdrawn; 404 when
/// no note carries that id; 500 (logged) for a failed read or write.
pub async fn post_note_reply(
    user: AuthUser,
    State(state): State<AppState>,
    Path(note_id): Path<Uuid>,
    Json(body): Json<NoteTextRequest>,
) -> Result<Json<PracticeNoteDto>, AppError> {
    let text = note_text(&body.text)?;
    let parent = note_by_id(&state.pipeline_pool, note_id)
        .await
        .map_err(|e| repo_error("note_by_id", format!("note {note_id}: {e}")))?
        .ok_or_else(|| AppError::NotFound {
            message: format!("practice note {note_id} not found"),
        })?;
    let reply = reply_target(&parent)?;
    // The scenario comes from its own read: `NoteRecord` does not carry one
    // (no panel prints it), and inferring it from the question would be a
    // second source of truth for where a note lives.
    let scenario_id = note_scenario(&state.pipeline_pool, note_id)
        .await
        .map_err(|e| repo_error("note_scenario", format!("note {note_id}: {e}")))?
        .ok_or_else(|| {
            // NOT the same fact as the 404 above, and it must not read as one.
            // The note was READ one statement ago, so it existed; `scenario_id`
            // is `NOT NULL`, so the only way here is that the row was deleted
            // between the two statements. That is a race, and an operator
            // reading "not found" would go looking for a bad client id instead.
            tracing::warn!(
                %note_id,
                "practice: a note vanished between its read and its scenario lookup — \
                 the reply was not written"
            );
            AppError::NotFound {
                message: format!("practice note {note_id} was withdrawn while you were replying"),
            }
        })?;
    let target = NoteTarget {
        scenario_id,
        question_id: reply.question_id,
        answer_id: parent.answer_id,
    };
    write_note(&state, &user, target, text, Some(reply.root)).await
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
    answers_note_id: Option<Uuid>,
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
            answers_note_id,
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
        // `?` rather than `%`: it is an `Option`, and `None` is the fact that
        // this was a note of its own rather than a reply — which is the one
        // thing this line could otherwise not tell an operator.
        replying_to = ?answers_note_id,
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

/// Where a reply attaches, once the note it answers has been read back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ReplyTarget {
    /// The head of the exchange — the note a reply is filed under, which is the
    /// parent itself unless the parent was already a reply.
    pub(super) root: Uuid,
    /// The question both halves belong to.
    pub(super) question_id: Uuid,
}

/// The two decisions a reply makes about its parent, without a database.
///
/// Pure so that both REFUSALS can be tested — an error path nothing exercises
/// is an error path nobody has read (Standing Rule 1).
///
/// # Errors
/// 400 when the parent was withdrawn, or when it is a note about the whole
/// scenario rather than about a question.
pub(super) fn reply_target(parent: &NoteRecord) -> Result<ReplyTarget, AppError> {
    if parent.struck_at.is_some() {
        // A struck note has been withdrawn. Answering it would put a live reply
        // under a line nobody stands behind any more — and the reply would wait
        // for somebody, on a page, about a note that says it was taken back.
        return Err(AppError::BadRequest {
            message: "that note was withdrawn — there is nothing to reply to".to_string(),
            details: serde_json::json!({ "field": "note_id" }),
        });
    }
    // Scenario-level notes are not routed anywhere yet (this module's header),
    // so no screen can offer Reply on one. Refused rather than filed with a
    // NULL question, which the table's CHECK would reject as a 500 and which no
    // panel could draw.
    let Some(question_id) = parent.question_id else {
        return Err(AppError::BadRequest {
            message: "a note about the whole scenario cannot be replied to".to_string(),
            details: serde_json::json!({ "field": "note_id" }),
        });
    };
    Ok(ReplyTarget {
        // The ROOT of the exchange — see `post_note_reply`.
        root: parent.answers_note_id.unwrap_or(parent.id),
        question_id,
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

    /// One stored note, as `note_by_id` hands it back.
    fn parent(struck: bool, question: Option<Uuid>, answers: Option<Uuid>) -> NoteRecord {
        NoteRecord {
            id: Uuid::from_u128(1),
            question_id: question,
            answer_id: None,
            author: "Chuck".to_string(),
            text: "whose account was it in?".to_string(),
            created_at: chrono::Utc::now(),
            struck_at: struck.then(chrono::Utc::now),
            struck_by: struck.then(|| "chuck".to_string()),
            answers_note_id: answers,
        }
    }

    /// A reply to a plain note is filed under that note, on its question.
    #[test]
    fn a_reply_is_filed_under_the_note_it_answers() {
        let target = reply_target(&parent(false, Some(Uuid::from_u128(9)), None))
            .expect("a standing note can be answered");
        assert_eq!(target.root, Uuid::from_u128(1));
        assert_eq!(target.question_id, Uuid::from_u128(9));
    }

    /// (M) A reply to a REPLY is filed under the ROOT, not under the reply.
    ///
    /// Left to nest, a third message would be drawn under a reply and the
    /// fourth under that, until the panel is a thread nobody designed. The
    /// exchange is two lines deep, and this is what keeps it that way.
    #[test]
    fn a_reply_to_a_reply_is_filed_under_the_root() {
        let root = Uuid::from_u128(42);
        let target = reply_target(&parent(false, Some(Uuid::from_u128(9)), Some(root)))
            .expect("a standing reply can be answered");
        assert_eq!(target.root, root);
    }

    /// A withdrawn note cannot be answered — a 400, not a live reply under a
    /// line nobody stands behind any more.
    #[test]
    fn a_struck_note_cannot_be_replied_to() {
        let Err(AppError::BadRequest { message, details }) =
            reply_target(&parent(true, Some(Uuid::from_u128(9)), None))
        else {
            panic!("a reply to a struck note is a 400");
        };
        assert!(message.contains("withdrawn"), "message: {message}");
        assert_eq!(details["field"], "note_id");
    }

    /// A note about the whole scenario has no question to file a reply on.
    #[test]
    fn a_scenario_note_cannot_be_replied_to() {
        assert!(matches!(
            reply_target(&parent(false, None, None)),
            Err(AppError::BadRequest { .. })
        ));
    }
}
