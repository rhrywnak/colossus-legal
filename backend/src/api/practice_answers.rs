//! The answer family: recording one, settling it, and the drawer she opened.
//!
//! Split from [`super::practice`] on 2026-08-17 when that module passed the
//! 300-line limit (Rule 17). The seam is the one the routes already draw: the
//! sibling module serves the DECK and the SESSION — addresses fenced by a case
//! and a scenario — while these three serve server-minted handles and are the
//! whole of what happens between "Answer" and "Got it".
//!
//! The routes are still declared in one place (`practice::routes`), because a
//! route table split across two files is how a path stops being served by
//! anything.
//!
//! ## Why one answer is written TWICE
//!
//! See [`crate::repositories::pipeline_repository::practice::close_answer`]: the
//! read has to exist before Marie can react to it, and her four boxes and her
//! mark are both decided afterwards. The row is created when she answers — the
//! moment worth surviving a closed laptop — and settled when she moves on.

use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    dto::practice::{
        AnswerRequest, AnswerResponse, CloseAnswerRequest, FlagRequest, FlagResponse,
        SkipQuestionRequest,
    },
    error::AppError,
    repositories::pipeline_repository::{
        practice::{
            close_answer, get_question, insert_answer, list_deck, list_points, mark_help_opened,
            session_scenario, NewAnswer, PracticeQuestionRecord,
        },
        practice_flow::set_flag,
    },
    services::{
        practice_page::tactic_name,
        practice_read::{read_answer, ReadOutcome},
        practice_read_parse::ReadInputs,
    },
    state::AppState,
};

use super::practice::repo_error;

/// The four boxes as an answer row OPENS: none ticked.
///
/// A helper rather than a literal at each of the two call sites, so a fifth box
/// added to the reveal cannot be added to one of them and forgotten in the
/// other. Domain note: this is PROVISIONAL. `post_close_answer` settles it when
/// she leaves the reveal, which is the first moment any of the four is known —
/// and a skipped question keeps these, because she was never shown a reveal to
/// tick them on.
fn unticked_self_check() -> serde_json::Value {
    serde_json::json!({
        "only_asked": false,
        "accepted_premise": false,
        "explained_unasked": false,
        "guessed": false
    })
}

pub async fn post_practice_answer(
    _user: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<AnswerRequest>,
) -> Result<Json<AnswerResponse>, AppError> {
    let (scenario_id, question) = fence_answer(&state, body.session_id, body.question_id).await?;
    let outcome = read_for(&state, scenario_id, &question, &body.answer_text).await;
    // `None` when the control was never opened; `Some([])` when she opened it
    // and picked nothing. The column keeps the two apart, so the mapping does
    // too — collapsing them would tell Chuck she considered the exhibits and
    // reached for none, on an answer where she never saw the list.
    let points_to = body.points_to.map(|picked| serde_json::json!(picked));

    let answer_id = insert_answer(
        &state.pipeline_pool,
        &NewAnswer {
            session_id: body.session_id,
            question_id: question.id,
            answer_text: body.answer_text,
            dont_recall: body.dont_recall,
            read_text: outcome.text.clone(),
            read_ok: outcome.ok,
            read_error: outcome.error,
            read_input_tokens: outcome.input_tokens,
            read_output_tokens: outcome.output_tokens,
            read_ms: outcome.ms,
            read_model: outcome.model,
            read_raw_reply: outcome.raw_reply,
            // The row opens PROVISIONAL: no boxes ticked, and marked fine. Both
            // are settled by `post_close_answer` when she leaves the reveal,
            // which is the first moment either is known. Recording her typed
            // answer now is what survives a closed laptop.
            self_check: unticked_self_check(),
            points_to,
            // The question AS ASKED, copied onto the answer now. Chuck's sheet
            // and the review page print this rather than joining the deck's
            // current text — Chuck edits the deck on Thursday, and a sheet that
            // silently re-worded itself would put her Tuesday answer under a
            // question she was never asked.
            question_text: question.text.clone(),
            mark: "fine".to_string(),
        },
    )
    .await
    .map_err(|e| repo_error("insert_answer", e))?;

    Ok(Json(AnswerResponse {
        answer_id,
        read_text: outcome.text,
        read_ok: outcome.ok,
    }))
}

/// Prove the answer belongs where it says it does, and return what it is about.
///
/// ## Why this fence exists
///
/// `session_id` and `question_id` both arrive from the browser. Without the deck
/// check, an answer could be logged against a question from ANOTHER scenario's
/// deck — and Chuck's sheet would then hold a question Marie was never asked,
/// with nothing on the page looking wrong.
///
/// # Errors
/// 404 when the session or the question does not exist; 400 when the question is
/// not in this session's deck.
async fn fence_answer(
    state: &AppState,
    session_id: Uuid,
    question_id: Uuid,
) -> Result<(Uuid, PracticeQuestionRecord), AppError> {
    let scenario_id = session_scenario(&state.pipeline_pool, session_id)
        .await
        .map_err(|e| repo_error("session_scenario", e))?
        .ok_or_else(|| AppError::NotFound {
            message: format!("practice session {session_id} not found"),
        })?;

    let question = get_question(&state.pipeline_pool, question_id)
        .await
        .map_err(|e| repo_error("get_question", e))?
        .ok_or_else(|| AppError::NotFound {
            message: format!("practice question {question_id} not found"),
        })?;

    let deck = list_deck(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| repo_error("list_deck", e))?;
    if !deck.iter().any(|q| q.id == question.id) {
        tracing::warn!(
            %session_id,
            question = %question.id,
            "practice: an answer named a question outside this session's deck"
        );
        return Err(AppError::BadRequest {
            message: "that question is not in this session's deck".to_string(),
            details: serde_json::json!({ "field": "question_id" }),
        });
    }
    Ok((scenario_id, question))
}

/// Ask the model for its one sentence about this answer.
///
/// Gathers the four things the read is judged against — the question, its
/// tactic, her three points and the watch-for — plus the ALWAYS card, which is
/// read from the same store the screen renders it from so the floor the model
/// judges by and the floor she sees are one row.
///
/// Never fails: every failure arm is inside the returned outcome. See
/// `services::practice_read` for why a slow vendor must not discard her answer.
async fn read_for(
    state: &AppState,
    scenario_id: Uuid,
    question: &PracticeQuestionRecord,
    answer_text: &str,
) -> ReadOutcome {
    let settings = state.settings.current();
    // A failure here costs the read its points, not the answer its row: the
    // outcome the caller stores says so, and the screen shows the stored
    // "no system read this time" line.
    let points: Vec<String> = match list_points(&state.pipeline_pool, scenario_id).await {
        Ok(rows) => rows.into_iter().map(|p| p.text).collect(),
        Err(e) => {
            tracing::error!(error = %e, %scenario_id, "practice: the read ran without her points");
            Vec::new()
        }
    };

    let side = if question.side == "george" {
        settings.practice_wording.pill_george.clone()
    } else {
        settings.practice_wording.pill_chuck.clone()
    };
    let tactic = tactic_name(&settings, question.tactic);

    read_answer(
        state,
        &ReadInputs {
            question: &question.text,
            tactic: tactic.as_deref(),
            side: &side,
            // Prompt v2 judges a CROSS answer, a DIRECT answer and a REDIRECT
            // answer by three different rules — a paragraph on cross is
            // "that's redirect", a paragraph on redirect is no fault at all —
            // so the kind is sent as itself rather than inferred from `side`,
            // which cannot tell Chuck's two apart.
            kind: &question.kind,
            answer: answer_text,
            points: &points,
            watch_for: question.watch_for.as_deref(),
            always: &settings.practice_wording.always_line,
        },
    )
    .await
}

/// She was dealt this question and set it aside: "Skip this one — doesn't fit".
///
/// ## Why this writes a ROW at all
///
/// Because it happened. A question she was shown and declined is a different
/// fact from one she was never dealt, and Chuck's sheet is the record of the
/// sitting — a skip that left no row would make the sheet claim a shorter
/// evening than she had. The stored phrase goes in `answer_text` so a skipped
/// row and a blank answer stay different rows.
///
/// ## Why there is no model call
///
/// There is nothing to read. She typed nothing, and asking a model to judge the
/// stored phrase would spend tokens to produce a sentence about a sentence this
/// service wrote itself.
///
/// # Errors
/// 404 when the session or the question does not exist; 400 when the question is
/// not in this session's deck. Same fence as an answer, for the same reason.
pub async fn post_skip_question(
    _user: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<SkipQuestionRequest>,
) -> Result<Json<AnswerResponse>, AppError> {
    let (_, question) = fence_answer(&state, body.session_id, body.question_id).await?;
    let settings = state.settings.current();

    let answer_id = insert_answer(
        &state.pipeline_pool,
        &NewAnswer {
            session_id: body.session_id,
            question_id: question.id,
            answer_text: settings.practice_wording.flow.skipped_answer_text.clone(),
            dont_recall: false,
            read_text: None,
            read_ok: None,
            // NOT an error: `read_error` says why a read is ABSENT, and the
            // honest reason here is that none was asked for. Leaving it NULL
            // would make a skip indistinguishable from a call that vanished.
            //
            // CONST (structural): a DIAGNOSTIC marker in a log column, not a
            // sentence anybody reads on a screen — which is exactly what
            // distinguishes it from `skipped_answer_text` two lines above, a
            // stored row because Marie's answer cell prints it. Every other
            // value this column holds is composed by this build from a failure
            // it observed (`the call failed: …`, `the read was 41 words; …`);
            // one of them arriving from the settings store would mean an
            // operator could edit what a past failure is recorded as having
            // been. It changes only when this code path changes.
            read_error: Some("no read: the question was skipped mid-sitting".to_string()),
            read_input_tokens: None,
            read_output_tokens: None,
            read_ms: None,
            read_model: None,
            read_raw_reply: None,
            self_check: unticked_self_check(),
            points_to: None,
            question_text: question.text.clone(),
            mark: "skipped".to_string(),
        },
    )
    .await
    .map_err(|e| repo_error("insert_answer", e))?;

    tracing::info!(
        session = %body.session_id,
        question = %question.id,
        %answer_id,
        "practice: a question was skipped mid-sitting"
    );
    Ok(Json(AnswerResponse {
        answer_id,
        read_text: None,
        read_ok: None,
    }))
}

/// She opened the stronger-answer drawer. Chuck's sheet says so.
pub async fn post_help_opened(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(answer_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let touched = mark_help_opened(&state.pipeline_pool, answer_id)
        .await
        .map_err(|e| repo_error("mark_help_opened", e))?;

    // A write that hit nothing is NOT a success. Reporting 200 here would let the
    // sheet print "—" against a question where she did open the help, which is
    // the one column Chuck reads to decide where to spend his mock cross.
    //
    // The log line names the id because the 404 alone cannot: an operator seeing
    // one in the access log has no way to tell which answer it was about, and
    // this is a request the browser makes on its own (a drawer opening), so
    // there is nobody at the keyboard to ask.
    if !touched {
        tracing::warn!(
            %answer_id,
            "practice: the drawer was opened on an answer that does not exist"
        );
        return Err(AppError::NotFound {
            message: format!("no practice answer {answer_id}"),
        });
    }
    Ok(Json(serde_json::json!({ "help_opened": true })))
}

/// Settle one answer: the mark she chose, and the four boxes she ticked.
///
/// Separate from the answer write because both are decided AFTER she has read
/// Which marks Marie may settle a row with from the reveal screen.
///
/// ## Domain note: `skipped` is stored but NOT settleable here
///
/// The `mark` CHECK permits three values, and this endpoint accepts two of
/// them. That is deliberate rather than an oversight: `skipped` is written by
/// the mid-sitting "Skip this one — doesn't fit" control, on a question she was
/// never shown a reveal for. Accepting it here would let a row that HAS an
/// answer and a read be relabelled as one she set aside — which would put a
/// sentence on Chuck's sheet under a mark saying she never gave it.
pub(super) fn is_settleable_mark(mark: &str) -> bool {
    matches!(mark, "fine" | "repeat")
}

/// the reveal. See `close_answer`'s note for why the row is created first.
pub async fn post_close_answer(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(answer_id): Path<Uuid>,
    Json(body): Json<CloseAnswerRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !is_settleable_mark(&body.mark) {
        return Err(AppError::BadRequest {
            message: "mark must be fine or repeat".to_string(),
            details: serde_json::json!({ "field": "mark", "value": body.mark }),
        });
    }

    let self_check = serde_json::to_value(&body.self_check).map_err(|e| {
        tracing::error!(error = %e, "practice: the self-check boxes would not serialize");
        AppError::Internal {
            message: "the self-check boxes could not be recorded".to_string(),
        }
    })?;

    let touched = close_answer(&state.pipeline_pool, answer_id, &body.mark, &self_check)
        .await
        .map_err(|e| repo_error("close_answer", e))?;

    // A write that hit nothing is NOT a success. Reporting 200 here would print
    // "fine" on Chuck's sheet against a question she asked to be repeated — the
    // one column he reads to decide where to spend his mock cross.
    if !touched {
        tracing::warn!(%answer_id, mark = %body.mark, "practice: a mark named an answer that does not exist");
        return Err(AppError::NotFound {
            message: format!("no practice answer {answer_id}"),
        });
    }
    Ok(Json(serde_json::json!({ "mark": body.mark })))
}

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
