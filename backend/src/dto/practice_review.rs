//! Part B's wire types: the deck editor's requests, notes, what changed, and the
//! review page.
//!
//! A file of its own rather than more of [`super::practice`], which was already
//! near Rule 17's limit. The seam is the audience: everything there is what
//! MARIE is served during a sitting; everything here is either addressed to
//! Chuck (the editor and its record) or about a sitting that is over (the notes
//! and the review page).
//!
//! ## Everything arrives composed, still
//!
//! The same law the v0 payload states. The heading over a stack of attempts, the
//! "changed since your last sitting" sentence, a struck note's "struck Tue 18
//! Aug" — every one is built on the server from a stored template. The browser
//! holds no templates and no date format, so how any of them reads is a Settings
//! edit.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::practice::{PracticePointDto, PracticeQuestionDto};
use super::practice_wording::PracticeWordingDto;

/// One note, as every panel renders it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PracticeNoteDto {
    pub id: Uuid,
    /// `None` on a scenario-level note.
    pub question_id: Option<Uuid>,
    /// `None` unless the note is about one attempt.
    pub answer_id: Option<Uuid>,
    pub author: String,
    pub text: String,
    /// `Tue 18 Aug` — composed, so the browser holds no date format.
    pub when: String,
    /// `struck Tue 19 Aug`, or `None` while the note stands. Its presence is
    /// also what tells the screen to strike the text through: one field, so a
    /// note cannot render struck without saying when.
    pub struck: Option<String>,
}

/// What changed since her last sitting, composed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PracticeChangedDto {
    /// `Changed since your last sitting: 3 questions — Chuck, Wed 19 Aug`,
    /// with the new-notes clause already appended when there is one.
    pub heading: String,
    /// The plain-words list behind the fold: `Q3 re-worded`, `Q6 moved`.
    pub items: Vec<String>,
}

/// One thing a new question can be attached to, in the add form's picker.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PracticeAttachOptionDto {
    /// `instance` or `point` — what the question's `source_kind` becomes.
    pub source_kind: String,
    /// 1-based, in the order the seed counts them.
    pub source_index: i32,
    /// `instance 2 — Hearing…, p. 33`, composed from the stored template.
    pub label: String,
}

/// One attempt at one question, as the review page stacks them.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PracticeAttemptDto {
    pub answer_id: Uuid,
    /// `attempt 2 · Wed 19 Aug 08:40`. The number counts from her FIRST
    /// attempt, so it does not change when she answers again.
    pub heading: String,
    /// The stored mark word: `fine`, `repeat` or `skipped`.
    pub mark: String,
    /// The raw stored value, so the screen can colour the word without matching
    /// on a sentence a Settings edit could change.
    pub mark_key: String,
    /// Her words, exactly as she typed them.
    pub answer: String,
    /// The one sentence, or `None` — the screen then shows the stored
    /// "no system read this time" line, as the reveal does.
    pub read_text: Option<String>,
    /// `true` green, `false` red, `None` neutral. Three states, as everywhere.
    pub read_ok: Option<bool>,
    /// What she said she would point to. EMPTY withdraws the clause.
    pub points_to: Vec<String>,
    /// `help: opened · boxes: I accepted a word or premise I shouldn't have`,
    /// composed from the stored template and the four stored box labels.
    pub detail: String,
    /// The notes written on THIS attempt, oldest first.
    pub notes: Vec<PracticeNoteDto>,
}

/// The review page, in one response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PracticeReviewPayload {
    pub scenario_id: Uuid,
    pub code: String,
    pub title: String,
    /// The question, with everything the study block renders.
    pub question: PracticeQuestionDto,
    /// `Question 3 · review` — composed, because the number is the question's
    /// printed position in the deck and only the server knows it.
    pub progress: String,
    /// Newest first, which is the order the page states out loud.
    pub attempts: Vec<PracticeAttemptDto>,
    /// Her three points, for the study block.
    pub points: Vec<PracticePointDto>,
    /// The notes on the QUESTION (Roman's amendment 2). Per-attempt notes ride
    /// on their attempt, not here.
    pub notes: Vec<PracticeNoteDto>,
    pub wording: PracticeWordingDto,
}

/// One field the editor may change, and its new value.
///
/// ## Why the field is a NAME and not a whole question object
///
/// The change log records one row per field with a before and an after, and a
/// request carrying the whole question would make "what changed" a diff this
/// service computed rather than a fact the editor stated. It also keeps the
/// audit honest: a request that touches one field cannot silently rewrite five.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditQuestionRequest {
    /// `text` | `tactic` | `follows` | `watch_for` | `stronger`.
    pub field: String,
    /// The new value. Absent or blank CLEARS the optional fields; `text` refuses
    /// a blank, because a question with no words is not a question.
    #[serde(default)]
    pub value: Option<String>,
    /// Chuck or Roman, from "Editing as". Required on every write.
    pub editing_as: String,
}

/// Move one question up or down within its own side.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoveQuestionRequest {
    /// `up` or `down`.
    pub direction: String,
    pub editing_as: String,
}

/// Hide one question, or put it back.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HideQuestionRequest {
    pub hidden: bool,
    pub editing_as: String,
}

/// A question somebody typed on the page.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddQuestionRequest {
    /// `cross` | `direct` | `redirect`. The side follows from it: a cross is
    /// George's and the other two are Chuck's, which is why the form asks one
    /// question and not two.
    pub kind: String,
    pub text: String,
    /// TACTIC_DECK_v1 card 1–7, on a cross question only.
    #[serde(default)]
    pub tactic: Option<i16>,
    /// The `deck_key` of the George question a redirect follows.
    #[serde(default)]
    pub follows: Option<String>,
    #[serde(default)]
    pub watch_for: Option<String>,
    /// What it attaches to. Absent = `no receipt`, and the screen says so in
    /// words rather than showing a blank source line.
    #[serde(default)]
    pub source_kind: Option<String>,
    #[serde(default)]
    pub source_index: Option<i32>,
    pub editing_as: String,
}

/// What a write to the deck did, so the browser can re-read rather than guess.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeckChangeResponse {
    /// The question the change landed on — a new one's id, for an add.
    pub question_id: Uuid,
}

/// One note on its way to the table.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewNoteRequest {
    /// `None` for a note about the scenario.
    #[serde(default)]
    pub question_id: Option<Uuid>,
    /// `None` unless the note is about one attempt.
    #[serde(default)]
    pub answer_id: Option<Uuid>,
    pub author: String,
    pub text: String,
}

/// Who is striking a note through.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrikeNoteRequest {
    pub author: String,
}

/// Place one question at an arbitrary position in its side (nav cleanup Part 2).
///
/// ## Why this is not a field on `MoveQuestionRequest`
///
/// The ▲▼ arrows and a drag are different operations, not two spellings of one.
/// "Move one step" cannot fail to name a position; "put this here" can, and it
/// re-sequences a whole side where the arrows swap two rows. Folding them into
/// one request would mean a `direction` that is sometimes required and a
/// `before` that is sometimes required, with nothing in the type saying which —
/// the shape where a client sends both and the server picks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReorderQuestionRequest {
    /// The question this one lands immediately ABOVE.
    ///
    /// `None` means the end of the side, which is what dropping past the last
    /// row means. Both are legitimate; neither is an error.
    #[serde(default)]
    pub before: Option<Uuid>,
}
