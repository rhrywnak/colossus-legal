//! What the "Discuss with AI" model is told — pure, no I/O.
//!
//! CC_TASK_QUESTION_CHAT_v1 §2. The model is sent the SAME package the answer read
//! builds (`practice_read_payload::build_user_message`, with the addendum's attack,
//! parent and notes), then the latest analysis, the thread so far, and the new
//! message. One package, two consumers: a change to what a question's context IS
//! reaches the read and the dock together.
//!
//! ## The draft (GO ruling 11)
//!
//! When Marie's unsaved draft rides the message, the package's HER ANSWER is the
//! draft and a line says so — the model must not treat unsaved words as what she
//! has committed to. The draft is never stored anywhere.

use crate::dto::practice_discussion::DiscussionTurnDto;
use crate::repositories::pipeline_repository::practice_discussions::{
    DiscussionTurnRecord, StandingAnswer,
};
use crate::services::practice_clock::local_clock;

// STRUCTURAL: model-wire vocabulary — these words go to the model to name an
// absent or special field, and are never shown on a screen. They are literals by
// the rule `practice_read_payload` states (an absent value is SAID, never
// omitted); a different phrasing is a coordinated prompt change, not a setting.
const NO_ANALYSIS: &str = "(no analysis has been stored for her answer)";
const DRAFT_NOTE: &str = "NOTE: HER ANSWER ABOVE IS AN UNSAVED DRAFT she is still writing — \
     discuss it as work in progress, not as what she has committed to.";
const NO_CONVERSATION: &str = "(this is the first message on this question)";
/// What stands in for the analysis when it FAILED (ADDENDUM_1, Roman): the
/// model is told there is no analysis and why, never handed Marie's failure
/// sentence as though it were an analysis. Also used by `chat_question_context`.
// STRUCTURAL: model-wire vocabulary, like the block above — the model-facing
// label for a failed analysis; rephrasing it is a coordinated prompt change.
pub const FAILED_ANALYSIS: &str = "no answer analysis (it failed)";
/// What stands in for HER ANSWER when nobody has answered and no draft was sent.
pub const NO_ANSWER: &str = "(she has not answered this question yet)";

/// The analysis line the model is given for her saved answer.
///
/// A FAILED analysis is said as such ([`FAILED_ANALYSIS`]) — never Marie's
/// failure sentence passed along as though it were an analysis (ADDENDUM_1).
/// `None` (no answer, or no analysis stored) becomes `NO_ANALYSIS` downstream.
pub fn analysis_for(saved: Option<&StandingAnswer>) -> Option<&str> {
    let answer = saved?;
    if answer.read_failed() {
        Some(FAILED_ANALYSIS)
    } else {
        answer.read_text.as_deref()
    }
}

/// The whole user message for one discussion call.
///
/// - `package` — `build_user_message` of the question's read payload
/// - `analysis` — the latest stored analysis of her SAVED answer, if any
/// - `draft` — whether HER ANSWER in the package is an unsaved draft
/// - `thread` — the turns BEFORE the new message, oldest first
pub fn compose_discussion_input(
    package: &str,
    analysis: Option<&str>,
    draft: bool,
    thread: &[DiscussionTurnRecord],
    new_author: &str,
    new_text: &str,
) -> String {
    let draft_note = if draft {
        format!("{DRAFT_NOTE}\n\n")
    } else {
        String::new()
    };
    let conversation = if thread.is_empty() {
        NO_CONVERSATION.to_string()
    } else {
        thread
            .iter()
            .map(|t| format!("{}: {}", t.author_name, t.text))
            .collect::<Vec<_>>()
            .join("\n\n")
    };
    format!(
        "{package}\n\
         {draft_note}\
         THE LATEST ANALYSIS OF HER SAVED ANSWER:\n{analysis}\n\n\
         THE CONVERSATION SO FAR, oldest first:\n{conversation}\n\n\
         THE NEW MESSAGE, from {new_author}:\n{new_text}\n",
        analysis = analysis.unwrap_or(NO_ANALYSIS),
    )
}

/// The thread as the dock renders it — authors and clock times composed.
pub fn thread_dtos(turns: &[DiscussionTurnRecord], timezone: &str) -> Vec<DiscussionTurnDto> {
    turns
        .iter()
        .map(|t| DiscussionTurnDto {
            id: t.id,
            role: t.role.clone(),
            author: t.author_name.clone(),
            model_id: t.model_id.clone(),
            text: t.text.clone(),
            when: local_clock(t.created_at, timezone),
            input_tokens: t.input_tokens,
            output_tokens: t.output_tokens,
            ms: t.ms,
        })
        .collect()
}

/// Whether the cap refuses one more model reply.
///
/// `used` is the model replies already stored. At `used >= max` the message is
/// refused BEFORE anything is stored or sent.
pub fn cap_reached(used: i64, max: u32) -> bool {
    used >= i64::from(max)
}

/// The reply to store, or why there is none: a vendor that answered HTTP 200
/// with nothing in it is a failed call, not an empty turn (the table refuses a
/// blank text anyway — this names the cause first).
pub fn usable_reply(text: &str, model_id: &str) -> Result<String, String> {
    let reply = text.trim();
    if reply.is_empty() {
        Err(format!("{model_id} returned an empty reply"))
    } else {
        Ok(reply.to_string())
    }
}

/// Which model answers: the one asked for, else the settings row's default — and
/// only if the chat provider map holds it.
///
/// `Err(id)` names the unknown model, which the route turns into a 400 exactly as
/// `/ask` does. `is_available` is the provider map's membership test, passed in so
/// this stays pure.
pub fn choose_model(
    requested: Option<&str>,
    default_row: &str,
    is_available: impl Fn(&str) -> bool,
) -> Result<String, String> {
    let chosen = requested.map_or_else(|| default_row.to_string(), str::to_string);
    if is_available(&chosen) {
        Ok(chosen)
    } else {
        Err(chosen)
    }
}

/// What HER ANSWER is for this message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscussedAnswer {
    /// The words the package carries as her answer.
    pub text: String,
    /// True when those words are an unsaved draft (GO ruling 11).
    pub is_draft: bool,
    /// The exhibits she ticked — only for a SAVED answer; a draft has none.
    pub points_to: Option<Vec<String>>,
}

/// Her unsaved draft when one was sent, else her saved answer, else the named
/// absence. A blank draft is no draft.
pub fn discussed_answer(draft: Option<&str>, saved: Option<&StandingAnswer>) -> DiscussedAnswer {
    if let Some(words) = draft.map(str::trim).filter(|d| !d.is_empty()) {
        return DiscussedAnswer {
            text: words.to_string(),
            is_draft: true,
            points_to: None,
        };
    }
    match saved {
        Some(answer) => DiscussedAnswer {
            text: answer.answer_text.clone(),
            is_draft: false,
            // best-effort: a points_to that is not a list of strings is sent as
            // "never opened" — the stored answer and its analysis still reach
            // the model, and the column's shape is owned by the answer writer.
            points_to: answer
                .points_to
                .as_ref()
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
        },
        None => DiscussedAnswer {
            text: NO_ANSWER.to_string(),
            is_draft: false,
            points_to: None,
        },
    }
}

#[cfg(test)]
#[path = "practice_discuss_tests.rs"]
mod tests;
