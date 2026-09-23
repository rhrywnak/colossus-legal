//! The "For you" page, as the browser receives it (CC_TASK_FOR_YOU_v1 L1).
//!
//! ## Domain note: every sentence arrives finished
//!
//! A row carries its deck line, its body and its byline as COMPOSED strings,
//! and the two tab labels arrive with their counts already in them. The browser
//! holds no templates and fills no placeholder — the law `wording_practice_row`
//! states for the deck row, applied to a page whose whole content is sentences.
//! What that buys: retuning how a row reads is a Settings edit and a restart.
//!
//! The wording block rides along anyway, because the page has its own furniture
//! to render — three day headings, an empty state, a title — and it is the same
//! one-request shape the practice deck uses.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::for_you_wording::ForYouWordingDto;

/// Whose list this viewer is being served.
///
/// ## Rust Learning: `#[serde(rename_all = "snake_case")]` on a unit enum
///
/// The variants cross the wire as `"reviewers"`, `"witness"` and `"none"`
/// rather than as Rust's `Reviewers`. The browser compares strings, and a
/// capital letter in one of them would be a bug nothing in either language
/// could catch — so the casing is declared once, here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForYouSide {
    /// A listed reviewer, or an administrator (`may_review`).
    Reviewers,
    /// The witness.
    Witness,
    /// Neither. An honest empty page, never an error.
    None,
}

/// Which day heading a row belongs under.
///
/// Decided on the SERVER, in the case's own timezone. A browser in another
/// timezone would group the same rows differently from the deck they came out
/// of, and the person reading would have no way to tell which was right.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForYouDay {
    Today,
    Yesterday,
    Earlier,
}

/// One row on the page.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForYouRowDto {
    /// `answer`, `note` or `change` — what the row is about. Carried so the
    /// page can mark the three apart visually; the SENTENCE that names the kind
    /// is already in `byline`.
    pub kind: String,
    /// The answer, note or change this row stands for.
    pub item_id: Uuid,
    pub scenario_id: Uuid,
    /// `None` only for a note about a whole scenario, which opens the deck
    /// rather than a question.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question_id: Option<Uuid>,
    /// `S-11 · The $50,000 — “Marie, whose money was the $50,000 check…”`
    pub deck_line: String,
    /// The note's text, or the answer / new wording under its stored template.
    pub body: String,
    /// `Chuck · on your answer of Mon 21 Sep`
    pub byline: String,
    /// The time, in the case's timezone: the clock alone for today, the full
    /// stamp for anything older.
    pub when: String,
    pub day: ForYouDay,
    /// True on the Everything tab for a row this person has already read.
    pub read: bool,
}

/// The whole page, in one request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForYouPayload {
    pub side: ForYouSide,
    /// The line under the title, with the other side already named.
    pub subtitle: String,
    /// `Unread · 3` — composed, so the browser fills no template.
    pub tab_unread_label: String,
    /// `Everything · 14`
    pub tab_everything_label: String,
    /// The badge's number. Serialized even at zero: the BROWSER decides not to
    /// draw a badge, and a missing field would be indistinguishable from a
    /// payload that forgot to carry it.
    pub unread_count: u32,
    pub unread: Vec<ForYouRowDto>,
    pub everything: Vec<ForYouRowDto>,
    /// The empty state's second line, with the other side named.
    pub empty_hint: String,
    /// Its third line. `None` when this side has never had an item — the line
    /// is then withheld entirely rather than rendered with an empty date.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub empty_last: Option<String>,
    pub wording: ForYouWordingDto,
}

/// The menu badge's one number, on its own address.
///
/// Its own endpoint rather than the whole page, because it is fetched on EVERY
/// page: the badge is the thing that carries the page to the rest of the app,
/// and making every screen pay for a list nobody is looking at would be the
/// wrong trade.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForYouSummaryDto {
    pub side: ForYouSide,
    /// Unread items across every deck of the case. Zero is a number, not an
    /// absent field — the badge's absence is the browser's decision.
    pub unread_count: u32,
}

/// What the read-clear write reports back.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestionSeenResponse {
    pub question_id: Uuid,
    /// How many seen rows this call actually WROTE — zero when the person had
    /// already read everything on the question, which is a legitimate state and
    /// is reported as a number rather than swallowed.
    ///
    /// The badge is NOT recomputed here. This route is addressed by a question
    /// id alone (like every other question write), so it does not know which
    /// case the caller is looking at, and guessing would be worse than the one
    /// extra request the browser already makes to refresh the count.
    pub marked: u32,
}
