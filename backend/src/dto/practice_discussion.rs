//! The wire shape of "Discuss with AI" (CC_TASK_QUESTION_CHAT_v1).
//!
//! Every turn arrives COMPOSED — its author name and its clock time — so the
//! browser formats nothing. The dock's WORDS ride the deck payload the question
//! page already has (`PracticeWordingDto`, `discuss_*`); this carries the thread,
//! the model list and the cap.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One message the dock renders.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscussionTurnDto {
    pub id: Uuid,
    /// `user` or `model`.
    pub role: String,
    /// The person's name, or the model's display name.
    pub author: String,
    /// The model that wrote a model turn; `null` on a user turn.
    pub model_id: Option<String>,
    pub text: String,
    /// `5:36 pm`, in the case's timezone.
    pub when: String,
    /// What a model reply cost; `null` on user turns and when not reported.
    /// Surfaced so spend is observable in the dock, not only in SQL.
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    pub ms: Option<i32>,
}

/// One model the picker offers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscussModelDto {
    pub model_id: String,
    pub display_name: String,
    /// `billed` or `local` — which `{cost}` word the footer prints.
    pub billing_class: String,
}

/// Everything the dock needs, in one response (GET and POST alike).
///
/// ## Domain note: the model list rides HERE (GO ruling 2)
///
/// Not `/api/chat/models`, which requires the AI group. The dock requires a login
/// only, so Marie can never be refused a picker she was offered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscussionPayload {
    pub question_id: Uuid,
    /// `S-5` — the dock's subtitle.
    pub scenario_code: String,
    /// The question's place in its scenario's deck, 1-based — `{n}` in the title.
    pub position: u32,
    /// Oldest first.
    pub turns: Vec<DiscussionTurnDto>,
    pub models: Vec<DiscussModelDto>,
    /// The `practice_discuss_default_model` settings row.
    pub default_model: String,
    /// The per-question cap on model replies, and how many are used.
    pub max_turns: u32,
    pub model_turns: u32,
}

/// One message sent to the dock.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscussionRequest {
    pub text: String,
    /// A model id from the payload's list; absent = the default row.
    #[serde(default)]
    pub model: Option<String>,
    /// Marie's UNSAVED draft, sent only when it differs from her saved answer.
    /// Given to the model as her current words for this one message; never stored.
    #[serde(default)]
    pub draft_answer: Option<String>,
}
