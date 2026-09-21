//! The question chat's wire shapes (CC_TASK_CHAT_ENGINE_v1).
//!
//! Every value a screen needs to DECIDE something is decided here, server-side
//! (Standing Rule 12): whether a thread is read-only, whose it is, how many are
//! unread. The browser formats; it does not judge.

use serde::{Deserialize, Serialize};

/// `GET /chat/question/:question_id/threads` — the switcher and the header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatThreadsPayload {
    pub question_id: String,
    /// The signed-in login.
    pub viewer: String,
    /// The model chip: its short name and whether its quotes are platform-checked.
    pub model_short_name: String,
    pub grounded: bool,
    /// The names of everyone else on the case, in switcher order — the header's
    /// "{others} can read this thread".
    pub others: Vec<String>,
    /// Every participant's thread, the viewer's first, then the rest.
    pub threads: Vec<ThreadRowDto>,
    /// The old dock's shared thread, when this question has one (ADDENDUM_1).
    pub earlier: Option<EarlierRowDto>,
    /// How long the browser waits with no event before giving up on a reply.
    pub client_idle_timeout_secs: u32,
    /// The per-thread reply cap, for the cap line.
    pub max_turns: u32,
}

/// One row of the switcher.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThreadRowDto {
    pub username: String,
    pub display_name: String,
    /// The avatar's letter.
    pub initial: String,
    pub is_viewer: bool,
    pub read_only: bool,
    pub message_count: i64,
    pub unread: i64,
    /// The last message, one line. `None` = no messages yet.
    pub preview: Option<String>,
    /// The day the thread began, e.g. `Sep 19`, when that was before today.
    pub resumed_from: Option<String>,
}

/// The Earlier team discussion row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EarlierRowDto {
    pub message_count: i64,
    /// The last message's day, e.g. `Sep 17`.
    pub last_on: String,
    pub preview: String,
}

/// `GET …/threads/:username` or `GET …/earlier` — one thread's messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadPayload {
    pub username: Option<String>,
    pub display_name: String,
    pub read_only: bool,
    pub messages: Vec<MessageDto>,
}

/// One message as the thread shows it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageDto {
    pub seq: i32,
    /// `user` or `assistant`.
    pub role: String,
    pub author_name: String,
    /// The prose, split so each cited passage's card sits under the words it backs.
    pub segments: Vec<SegmentDto>,
    /// e.g. `Fri 19 Sep · 8:00 am`.
    pub at: String,
    /// A named failure (`refused`, `truncated`, `stalled`, `failed`) — the screen
    /// shows the matching sentence; `None` = the message is fine.
    pub failure: Option<String>,
}

/// A run of prose and the cards that back it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SegmentDto {
    pub text: String,
    pub cards: Vec<CitationCardDto>,
}

/// A citation card: the document's name and date, and the passage — sliced from
/// the STORED document, never composed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CitationCardDto {
    pub document_title: String,
    pub document_date: Option<String>,
    pub page: Option<i32>,
    pub quoted_text: String,
}

/// `POST …/threads/:username/messages`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SendMessageRequest {
    pub text: String,
}

/// `POST …/threads/:username/read`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarkReadRequest {
    pub seq: i32,
}

/// A stored citation card (the `discussion_messages.citations` JSONB element).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredCard {
    /// Which content block of the message it annotates.
    pub block_index: usize,
    pub document_id: String,
    pub document_title: String,
    pub document_date: Option<String>,
    pub page: Option<i32>,
    pub start_char: usize,
    pub end_char: usize,
    pub quoted_text: String,
    /// The provider's range did not slice to the cited text; found by search.
    pub relocated: bool,
}
