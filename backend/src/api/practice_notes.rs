//! Notes, and the review page — the two surfaces about work already done.
//!
//! Tasks B3 and B4. Neither changes an answer: a note is written beside one, and
//! the review page only reads. Roman's ruling, and the reason is worth keeping —
//! an answer is a moment, and she answers again instead of correcting it.
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
    domain::scenario_code::scenario_code,
    dto::practice_review::{
        NewNoteRequest, PracticeNoteDto, PracticeReviewPayload, StrikeNoteRequest,
    },
    error::AppError,
    repositories::pipeline_repository::{
        get_scenario,
        practice::{get_question, list_deck, list_point_receipts, list_points},
        practice_notes::{
            attempts_for_question, insert_note, list_notes, note_scenario, strike_note, NewNote,
        },
    },
    services::{
        practice_notes::{author_list, is_note_author, note_dto},
        practice_page::{point_dto, question_dto_for},
        practice_review::{attempts, progress, question_notes},
    },
    state::AppState,
};

use super::practice::repo_error;
use super::scenario_facts::{ensure_scenario_in_case, parse_scenario_id};

/// Refuse a note nobody has signed.
///
/// # Errors
/// 400 naming the stored list of authors.
fn fence_author(state: &AppState, author: &str) -> Result<String, AppError> {
    let settings = state.settings.current();
    if !is_note_author(&settings, author.trim()) {
        return Err(AppError::BadRequest {
            message: format!(
                "\"{}\" is not one of the people who may write a note here ({})",
                author.trim(),
                author_list(&settings)
            ),
            details: serde_json::json!({ "field": "author", "value": author }),
        });
    }
    Ok(author.trim().to_string())
}

/// Prove a note before it is written, and hand back its trimmed text.
///
/// ## Why the fence checks the QUESTION belongs to the scenario
///
/// `scenario_id` comes from the path and `question_id` from the body. Without
/// this a note could be filed against another scenario's question and appear on
/// a panel nobody expected — the same class as the answer path's fence, and the
/// same fix.
///
/// # Errors
/// 400 for a blank note, a question outside this scenario's deck, or an attempt
/// note that names no question.
async fn fence_note<'a>(
    state: &AppState,
    scenario_id: Uuid,
    body: &'a NewNoteRequest,
) -> Result<&'a str, AppError> {
    let text = body.text.trim();
    if text.is_empty() {
        return Err(AppError::BadRequest {
            message: "a note has to say something".to_string(),
            details: serde_json::json!({ "field": "text" }),
        });
    }
    if let Some(question_id) = body.question_id {
        let deck = list_deck(&state.pipeline_pool, scenario_id)
            .await
            .map_err(|e| repo_error("list_deck", e))?;
        if !deck.iter().any(|q| q.id == question_id) {
            return Err(AppError::BadRequest {
                message: "that question is not in this scenario's deck".to_string(),
                details: serde_json::json!({ "field": "question_id" }),
            });
        }
    } else if body.answer_id.is_some() {
        // The table's CHECK says the same thing; saying it here makes it a 400
        // with a sentence rather than a 500 naming a constraint.
        return Err(AppError::BadRequest {
            message: "a note on an attempt must also name its question".to_string(),
            details: serde_json::json!({ "field": "question_id" }),
        });
    }
    Ok(text)
}

/// Write one note — on the scenario, on a question, or on one attempt.
///
/// # Errors
/// 400 for an unknown author or anything [`fence_note`] refuses; 404 when the
/// scenario does not exist.
pub async fn post_note(
    _user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
    Json(body): Json<NewNoteRequest>,
) -> Result<Json<PracticeNoteDto>, AppError> {
    let scenario_id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, scenario_id, &slug).await?;
    let author = fence_author(&state, &body.author)?;

    let text = fence_note(&state, scenario_id, &body).await?;

    let id = insert_note(
        &state.pipeline_pool,
        &NewNote {
            scenario_id,
            question_id: body.question_id,
            answer_id: body.answer_id,
            author: &author,
            text,
        },
    )
    .await
    .map_err(|e| repo_error("insert_note", e))?;

    // Re-read rather than composing the reply from the request: the stored
    // `created_at` is the server's, and a screen showing a note dated by the
    // browser's clock would disagree with the panel the moment it reloads.
    let stored = list_notes(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| repo_error("list_notes", e))?;
    let record = stored.into_iter().find(|n| n.id == id).ok_or_else(|| {
        // Unreachable through the database — the insert returned this id from
        // the same table this read walks. It is logged with BOTH identifiers
        // anyway: the `info!` below never runs on this path, so without this an
        // operator seeing the 500 has no way to tell which note or which
        // scenario it was about except by correlating timestamps by hand.
        tracing::error!(
            %scenario_id,
            note = %id,
            "practice: a note was written and could not be read back"
        );
        AppError::Internal {
            message: format!(
                "note {id} was written to scenario {scenario_id} but could not be read back"
            ),
        }
    })?;

    tracing::info!(
        %scenario_id,
        note = %id,
        author = %author,
        on_question = body.question_id.is_some(),
        on_attempt = body.answer_id.is_some(),
        "practice: a note was written"
    );
    Ok(Json(note_dto(&state.settings.current(), &record)))
}

/// Strike one note through. Never a delete.
///
/// # Errors
/// 400 for an unknown author; 404 when no note carries that id.
pub async fn post_strike_note(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(note_id): Path<Uuid>,
    Json(body): Json<StrikeNoteRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let author = fence_author(&state, &body.author)?;

    let touched = strike_note(&state.pipeline_pool, note_id, &author)
        .await
        .map_err(|e| repo_error("strike_note", e))?;
    // `strike_note` returns TRUE for a note that was already struck — striking
    // twice keeps the first striking, because the moment somebody withdrew a
    // note is not something a second press should move. So `false` means one
    // thing only: no note carries that id.
    if !touched {
        return Err(AppError::NotFound {
            message: format!(
                "practice note {note_id} does not exist — reload the panel to see the                  notes as they stand"
            ),
        });
    }
    // Read for the log only: an operator seeing a strike wants to know which
    // scenario it was on, and the id alone cannot say.
    let scenario = note_scenario(&state.pipeline_pool, note_id)
        .await
        .map_err(|e| repo_error("note_scenario", e))?;
    tracing::info!(note = %note_id, by = %author, ?scenario, "practice: a note was struck");
    Ok(Json(serde_json::json!({ "struck": true })))
}

/// Everything one review page is read from.
struct ReviewRead {
    position: usize,
    rows: Vec<crate::repositories::pipeline_repository::practice_notes::AttemptRecord>,
    notes: Vec<crate::repositories::pipeline_repository::practice_notes::NoteRecord>,
    points: Vec<crate::repositories::pipeline_repository::practice::PracticePointRecord>,
    receipts: Vec<crate::repositories::pipeline_repository::practice::PracticePointReceipt>,
}

/// The five reads, or fail naming the one that did.
///
/// Gathered so the handler stays the four steps it reads as — fence the case,
/// find the question, read, compose — rather than a run of five near-identical
/// `map_err` blocks.
async fn read_review_sources(
    state: &AppState,
    scenario_id: Uuid,
    question_id: Uuid,
) -> Result<ReviewRead, AppError> {
    let deck = list_deck(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| repo_error("list_deck", e))?;
    Ok(ReviewRead {
        // The question's PRINTED position, which is the number beside it on the
        // start card. `unwrap_or(0) + 1` cannot fire — the caller has already
        // proved the question is in this scenario — and it renders 1 rather
        // than panicking if it somehow does.
        position: deck.iter().position(|q| q.id == question_id).unwrap_or(0) + 1,
        rows: attempts_for_question(&state.pipeline_pool, scenario_id, question_id)
            .await
            .map_err(|e| repo_error("attempts_for_question", e))?,
        notes: list_notes(&state.pipeline_pool, scenario_id)
            .await
            .map_err(|e| repo_error("list_notes", e))?,
        points: list_points(&state.pipeline_pool, scenario_id)
            .await
            .map_err(|e| repo_error("list_points", e))?,
        receipts: list_point_receipts(&state.pipeline_pool, scenario_id)
            .await
            .map_err(|e| repo_error("list_point_receipts", e))?,
    })
}

/// The review page for one question: every attempt, newest first.
///
/// # Errors
/// 404 when the scenario or the question does not exist, or when the question
/// belongs to another scenario's deck.
pub async fn get_question_review(
    _user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id, question_id)): Path<(String, String, Uuid)>,
) -> Result<Json<PracticeReviewPayload>, AppError> {
    let scenario_id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, scenario_id, &slug).await?;

    let record = get_scenario(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| repo_error("get_scenario", e))?
        .ok_or_else(|| AppError::NotFound {
            message: format!("scenario {scenario_id} not found"),
        })?;
    let question = get_question(&state.pipeline_pool, question_id)
        .await
        .map_err(|e| repo_error("get_question", e))?
        .filter(|q| q.scenario_id == scenario_id)
        .ok_or_else(|| AppError::NotFound {
            message: format!("practice question {question_id} is not in this scenario's deck"),
        })?;

    let read = read_review_sources(&state, scenario_id, question_id).await?;
    let settings = state.settings.current();
    let payload = PracticeReviewPayload {
        scenario_id,
        code: scenario_code(record.code_ordinal),
        title: record.name,
        question: question_dto_for(&settings, question),
        progress: progress(&settings, read.position),
        attempts: attempts(&settings, &read.rows, &read.notes),
        points: read
            .points
            .into_iter()
            .map(|p| point_dto(p, &read.receipts))
            .collect(),
        notes: question_notes(&settings, &read.notes, question_id),
        wording: crate::dto::practice_wording::PracticeWordingDto::from_blocks(
            &settings.practice_wording,
            &settings.practice_report_wording,
        ),
    };

    tracing::info!(
        %scenario_id,
        %question_id,
        attempts = payload.attempts.len(),
        notes = payload.notes.len(),
        "served one practice question's review"
    );
    Ok(Json(payload))
}
