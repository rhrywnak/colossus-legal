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
        AnswerRequest, AnswerResponse, CloseAnswerRequest, ReadPartsDto, ReadSourceDto,
        SkipQuestionRequest,
    },
    error::AppError,
    repositories::pipeline_repository::{
        practice::{get_question, list_deck, session_scenario, PracticeQuestionRecord},
        practice_answers::{attach_read, close_answer, insert_answer, mark_help_opened, NewAnswer},
        practice_flow::current_answer_for,
    },
    services::{
        practice_answer_version::is_reread, practice_read::read_answer,
        practice_read_gather::gather_payload, practice_read_outcome::ReadOutcome,
    },
    state::AppState,
};

use super::practice::repo_error;
use super::practice_fences::fence_answer_text;

/// Why a row that has just been opened carries no read.
///
/// ## Domain note: the state the two-write shape creates, named rather than blank
///
/// Before T1 a row with `read_text IS NULL` always had `read_error IS NOT NULL` —
/// every failure arm filled it, so "no read and no reason" was unreachable
/// **[measured: 0 of 12 rows on DEV]**. Writing the answer before the call makes
/// that combination the shape of a read IN FLIGHT — and also the shape of a
/// backend that died mid-read. Two operationally distinct states sharing one
/// observable is what Standing Rule 1 forbids, so the insert says which it is and
/// `attach_read` clears it.
///
/// STRUCTURAL, like the skip marker below it: a DIAGNOSTIC in a log column, not a
/// sentence anybody reads on a screen, and composed by this build from a fact it
/// knows about itself. A settings row here would let an operator edit what a past
/// crash is recorded as having been.
// STRUCTURAL: a diagnostic marker in a log column, never wording on a screen.
// CONST: structural — see the doc comment above for why it is not a settings row.
const READ_IN_FLIGHT: &str = "no read yet: the answer was recorded and the model is being asked";

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

/// Record one answer, then ask the model to read it.
///
/// ## Why the read cannot fail this request
///
/// Her answer is worth recording whatever the model does — and since T1 it is
/// recorded FIRST. The row is committed before a single token is sent, so a
/// vendor that is slow, down, or returns something unusable costs her a read and
/// never an answer. A failed read is attached as an abstain with the reason in
/// `read_abstain_reason`; every other box on the reveal stands.
///
/// # Errors
/// 404/400 from the fences, 409 for a second answer to a settled question, 500
/// only if the answer itself cannot be written.
pub async fn post_practice_answer(
    _user: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<AnswerRequest>,
) -> Result<Json<AnswerResponse>, AppError> {
    let (scenario_id, question) = fence_answer(&state, body.session_id, body.question_id).await?;
    fence_answer_text(&body)?;

    // ## ⚑ WHY THERE IS NO "already answered" REFUSAL HERE ANY MORE
    //
    // `fence_not_already_answered` used to 409 a second answer to the same
    // question in the same sitting, and was right to: the sitting model dealt
    // each question once, so a second arrival meant a stale tab — which is what
    // its message said.
    //
    // CC_TASK_PRACTICE_ONE_PAGE makes that loop THE DESIGN. §4: "She edits the
    // box and presses Answer again, or goes back and picks another question.
    // That loop is the whole design." And the sitting is now invisible plumbing
    // reused across an afternoon, so the fence would fire on her second edit of
    // the day and tell her to reload a page that is perfectly current.
    //
    // What replaces it is Roman's ruling of 2026-08-23, below: a VERSION IS A
    // CHANGE SHE MADE, NOT A BUTTON SHE PRESSED TWICE.

    // `None` when the control was never opened; `Some([])` when she opened it
    // and picked nothing. The column keeps the two apart, so the payload does
    // too — collapsing them would tell the model she considered the exhibits and
    // reached for none, on an answer where she never saw the list.
    let points_to_json = body
        .points_to
        .as_ref()
        .map(|picked| serde_json::json!(picked));

    // Byte-identical to what already stands? Then she pressed Answer twice —
    // after Stop waiting, or out of habit having re-read a critique — and this
    // is a RE-REQUEST OF THE READ, not a new version. Her "2 earlier versions"
    // line must count things that are versions of something.
    //
    // Byte-identical and not trimmed-equal: a trailing space she added and meant
    // is a change, and this code cannot tell which spaces she meant.
    let standing = current_answer_for(&state.pipeline_pool, question.id)
        .await
        .map_err(|e| repo_error("current_answer_for", e))?;
    let unchanged = is_reread(
        standing.as_ref().map(|(_, text)| text.as_str()),
        &body.answer_text,
    );

    // STEP ONE: her answer, on disk, before anything is asked of anybody.
    let answer_id = if let (true, Some((existing, _))) = (unchanged, standing.as_ref()) {
        tracing::info!(
            question_id = %question.id,
            answer_id = %existing,
            "practice: the text is unchanged — re-reading, not versioning"
        );
        *existing
    } else {
        // ⚑ THE OTHER ARM LOGS TOO, and that is Rule 1 rather than symmetry.
        // Two operationally distinct states must produce two observables. With
        // only the re-read logged, an operator would have to infer "a version
        // was written" from the ABSENCE of a line — which is indistinguishable
        // from the request never arriving.
        tracing::info!(
            question_id = %question.id,
            had_previous = standing.is_some(),
            "practice: the text changed — writing a new version"
        );
        insert_answer(
            &state.pipeline_pool,
            &NewAnswer {
                session_id: body.session_id,
                question_id: question.id,
                answer_text: body.answer_text.clone(),
                dont_recall: body.dont_recall,
                // The row opens PROVISIONAL: no boxes ticked, and marked fine. Both
                // are settled by `post_close_answer` when she leaves the reveal,
                // which is the first moment either is known.
                self_check: unticked_self_check(),
                points_to: points_to_json,
                // The question AS ASKED, copied onto the answer now. Chuck's sheet
                // and the review page print this rather than joining the deck's
                // current text — Chuck edits the deck on Thursday, and a sheet that
                // silently re-worded itself would put her Tuesday answer under a
                // question she was never asked.
                question_text: question.text.clone(),
                mark: "fine".to_string(),
                read_error: Some(READ_IN_FLIGHT.to_string()),
            },
        )
        .await
        .map_err(|e| repo_error("insert_answer", e))?
    };

    // STEP TWO: the read.
    let (outcome, read_sources) = read_for(
        &state,
        scenario_id,
        &question,
        &body.answer_text,
        body.points_to.as_ref(),
    )
    .await;

    // STEP THREE: attach it. A failure HERE must not 500 — her answer is already
    // committed, and telling her it was lost would be the exact lie the two-write
    // shape exists to prevent. The row keeps its in-flight marker, which is the
    // honest record of what happened, and she sees the same "no read" surface as
    // every other read failure.
    match attach_read(&state.pipeline_pool, answer_id, &outcome.to_row()).await {
        Ok(true) => {}
        Ok(false) => tracing::error!(
            %answer_id,
            "practice: the read named an answer row that vanished between two writes"
        ),
        Err(e) => tracing::error!(
            %answer_id, error = %e,
            "practice: her answer is recorded and its read could not be attached"
        ),
    }

    Ok(Json(AnswerResponse {
        answer_id,
        // The composed line still ships for anything that wants one sentence;
        // the parts ship beside it for the screen that draws three.
        read_parts: outcome.parts.as_ref().map(|parts| ReadPartsDto {
            call: parts.call.clone(),
            why: parts.why.clone(),
            pointers: parts.pointers.clone(),
            keys: parts.keys.clone(),
        }),
        read_sources,
        read_text: outcome.text,
        read_ok: outcome.ok,
    }))
}

/// Ask the model to read one typed answer — or decline, honestly, without one.
///
/// Three arms, and the first two never reach a model:
///
/// 1. **The stored "I don't recall." line.** No call. See [`ReadOutcome::stored`].
/// 2. **An input that failed to load.** No call, and an ABSTAIN rather than a
///    read composed against material that silently went missing.
/// 3. Everything else, including a one-word answer like `test`, which goes to the
///    model and comes back on the abstain arm. A length rule here would be wrong:
///    `"Yes."` is a complete answer on direct.
///
/// Never fails: every arm is inside the returned outcome.
async fn read_for(
    state: &AppState,
    scenario_id: Uuid,
    question: &PracticeQuestionRecord,
    answer_text: &str,
    points_to: Option<&Vec<String>>,
) -> (ReadOutcome, Vec<ReadSourceDto>) {
    let settings = state.settings.current();

    if is_stored_dont_recall(&settings.practice_wording.dont_recall_text, answer_text) {
        tracing::info!(
            question = %question.id,
            "practice read: the stored don't-recall line — no model call"
        );
        // No model call, so nothing was cited and there is nothing to footnote.
        return (
            ReadOutcome::stored(
                settings
                    .practice_report_wording
                    .read_dont_recall_line
                    .clone(),
            ),
            Vec::new(),
        );
    }

    match gather_payload(state, scenario_id, question, answer_text, points_to).await {
        Ok(payload) => {
            // The sources are taken from the payload that was SENT, not from the
            // reply: a key the model invented is already refused upstream, and a
            // footnote list built from the reply could only ever agree with
            // itself. These are the words Marie was judged against.
            // ⚑ THE SWORN PAIR IS PART OF THIS LIST. `citable_keys()` — the one
            // authority on what the model may cite — adds S1 when `said` is
            // present and S2 when `admitted` is. Iterating only points and
            // receipts left a critique free to cite S2 with NOTHING behind it,
            // which is the exact failure `practice_read_source_missing` exists
            // to expose, reachable on every sworn-pair question. Caught by the
            // architecture gate before it shipped.
            //
            // The keys are literals here because they are literals in
            // `citable_keys` — the model is told those two names, and a
            // constant that drifted from the payload builder would produce a
            // footnote list that agreed with itself and not with the prompt.
            let sworn = [
                payload.said.as_ref().map(|text| ReadSourceDto {
                    key: "S1".to_string(),
                    text: text.clone(),
                }),
                payload.admitted.as_ref().map(|text| ReadSourceDto {
                    key: "S2".to_string(),
                    text: text.clone(),
                }),
            ];
            let sources = payload
                .points
                .iter()
                .chain(payload.receipts.iter())
                .filter_map(|item| {
                    item.text.as_ref().map(|text| ReadSourceDto {
                        key: item.key.clone(),
                        text: text.clone(),
                    })
                })
                .chain(sworn.into_iter().flatten())
                .collect();
            (read_answer(state, &payload).await, sources)
        }
        Err(failure) => {
            tracing::error!(
                question = %question.id, %scenario_id, reason = %failure,
                "practice read: abstaining — an input the read is judged against did not load"
            );
            // An abstain cites nothing, so it footnotes nothing.
            (
                ReadOutcome::from_payload_failure(
                    &settings.practice_report_wording.read_abstain_line,
                    &failure,
                ),
                Vec::new(),
            )
        }
    }
}

/// Is this answer the sentence the "I don't recall." button sends?
///
/// ## Domain note: TRIMMED equality, and nothing looser
///
/// The comparison is against the stored line and no other rule. Not a prefix, not
/// a case-insensitive match, not "contains" — because an answer that BEGINS with
/// "I don't recall" and goes on to say something is a real answer and must be
/// read. Only the exact stored sentence, which this system wrote and this system
/// therefore has nothing to learn from.
///
/// Trimmed because a browser may send a trailing newline, and a short-circuit
/// defeated by one whitespace character would be a silent per-click cost nobody
/// would ever notice.
pub(super) fn is_stored_dont_recall(stored: &str, answer_text: &str) -> bool {
    answer_text.trim() == stored.trim()
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
            // NOT an error: `read_error` says why a read is ABSENT, and the
            // honest reason here is that none was asked for. Leaving it NULL
            // would make a skip indistinguishable from a call that vanished.
            //
            // STRUCTURAL: a DIAGNOSTIC marker in a log column, not a
            // sentence anybody reads on a screen — which is exactly what
            // distinguishes it from `skipped_answer_text` two lines above, a
            // stored row because Marie's answer cell prints it. Every other
            // value this column holds is composed by this build from a failure
            // it observed (`the call failed: …`, `the read was 41 words; …`);
            // one of them arriving from the settings store would mean an
            // operator could edit what a past failure is recorded as having
            // been. It changes only when this code path changes.
            //
            // A skip writes its marker at INSERT and never calls `attach_read`,
            // which is what keeps it distinct from the in-flight marker three
            // screens up: one says a read is coming, the other says none was ever
            // asked for.
            read_error: Some("no read: the question was skipped mid-sitting".to_string()),
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
        // A skip makes no model call, so there is no critique and nothing to
        // footnote. Both absent rather than empty-but-present: "no read" and
        // "a read that said nothing" are different facts.
        read_parts: None,
        read_sources: Vec::new(),
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

#[cfg(test)]
#[path = "practice_answers_tests.rs"]
mod tests;
