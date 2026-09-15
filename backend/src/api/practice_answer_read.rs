//! What happens between an answer landing on disk and a read landing beside it.
//!
//! Split from [`super::practice_answers`] on 2026-09-15, when the Answer-analysis
//! switch took that module past the 300-line limit (Rule 17) — the same reason,
//! and the same kind of seam, as the 2026-08-17 split that created it from
//! [`super::practice`].
//!
//! ## The seam: a ROUTE module and a READ module
//!
//! The sibling serves four addresses. This is the one thing that happens at one
//! of them and at no other: a model is asked what it makes of what Marie typed,
//! and what it says is written onto her answer row. Nothing here is routed,
//! nothing here is fenced, and nothing here can fail a request.
//!
//! ## ⚑ Why the SWITCH is not read in this file
//!
//! `want_read` is consulted by the handler, which then either calls
//! [`read_and_attach`] or does not. Putting the branch here instead would mean
//! the off path still ran the re-read arm's `attach_read` — overwriting a
//! critique Marie already has with a marker saying nobody asked. The decision
//! belongs where the alternative to it (do nothing at all) can be expressed.

use uuid::Uuid;

use crate::{
    dto::practice::{AnswerRequest, ReadSourceDto},
    repositories::pipeline_repository::{
        practice::PracticeQuestionRecord, practice_answers::attach_read,
    },
    services::{
        practice_read::read_answer, practice_read_gather::gather_payload,
        practice_read_outcome::ReadOutcome,
    },
    state::AppState,
};

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
// `pub(crate)` so the proof that the two "no read yet" markers are DIFFERENT
// sentences can name them both — see `practice_read_outcome_tests`. A test that
// re-typed this string would agree with itself and nothing else.
pub(crate) const READ_IN_FLIGHT: &str =
    "no read yet: the answer was recorded and the model is being asked";

/// STEP TWO and STEP THREE: ask the model, then put what it said on the row.
///
/// Extracted from the handler when the switch arrived, so that "what happens when
/// a read IS wanted" is one thing with one name — and so the handler reads as the
/// decision it now is: fence, write, and then either this or nothing.
///
/// ## Why an attach failure does not fail the request
///
/// Her answer is already committed. Telling her it was lost would be the exact
/// lie the two-write shape exists to prevent, so the row keeps its in-flight
/// marker — the honest record of what happened — and she sees the same "no read"
/// surface as every other read failure.
///
/// Never fails: every arm is inside the returned outcome.
pub(super) async fn read_and_attach(
    state: &AppState,
    scenario_id: Uuid,
    question: &PracticeQuestionRecord,
    body: &AnswerRequest,
    answer_id: Uuid,
) -> (ReadOutcome, Vec<ReadSourceDto>) {
    let (outcome, read_sources) = read_for(
        state,
        scenario_id,
        question,
        &body.answer_text,
        body.points_to.as_ref(),
    )
    .await;

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

    (outcome, read_sources)
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
            // ⚑ ONE AUTHORITY. `citable_sources` is the same function the
            // prompt's key line is built from, so the footnote list cannot
            // disagree with what the model was allowed to cite. It used to be
            // assembled here by hand from points and receipts, and it omitted
            // the sworn pair — a read could cite S2 and the screen would show
            // that key with nothing under it, silently, on every sworn-pair
            // question.
            //
            // These are the words that were SENT. Never words that came back:
            // a list built from the reply would let a hallucinated citation
            // render its own supporting evidence.
            let sources = payload
                .citable_sources()
                .into_iter()
                .map(|(key, text)| ReadSourceDto { key, text })
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
pub(crate) fn is_stored_dont_recall(stored: &str, answer_text: &str) -> bool {
    answer_text.trim() == stored.trim()
}
