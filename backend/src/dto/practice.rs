//! The practice drill's wire types (screens S0–S3).
//!
//! ## Everything arrives composed
//!
//! The last-session line, the sheet's heading, the sheet's "From" cell and its
//! Mark and Help cells are all SENTENCES, and every one of them is built on the
//! server from a stored template. The browser concatenates nothing and holds no
//! templates, so it could not recompose one if it tried — the same law the
//! rehearsal payload states, applied to the surface with the least forgiving
//! reader in the product.
//!
//! ## What is deliberately absent
//!
//! No score. No streak. No timer. No confidence, tier or verdict. Adding one
//! would mean ADDING A FIELD to this file — a visible change a reviewer sees —
//! rather than a value slipped through a mapper.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::practice_wording::PracticeWordingDto;

/// One question, with everything its two screens render.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PracticeQuestionDto {
    pub id: Uuid,
    /// `george` or `chuck` — which pill the screen shows.
    pub side: String,
    /// True when this question braids several barrage rows: a third pill, and a
    /// different one, because a braid is answered differently from either side.
    pub braid: bool,
    pub text: String,
    /// The tactic's NAME, resolved server-side from the card number. `None` on a
    /// question that carries none, which withdraws the tag rather than printing
    /// an empty one.
    pub tactic: Option<String>,
    /// The "Built from: …" line, or `None` — which the screen leaves empty
    /// rather than inventing a source.
    pub receipt: Option<String>,
    /// The barrage rows a braid names, appended to the source line in bold.
    pub braid_rows: Option<String>,
    pub watch_for: Option<String>,
    /// The two halves of the pair, present together or not at all.
    pub pair_said: Option<String>,
    pub pair_admitted: Option<String>,
    /// The drawer's example. `None` renders the stored "no receipt for this one"
    /// line — a Chuck question, in the mockup's own words.
    pub stronger: Option<String>,
    pub stronger_lean: Option<String>,
}

/// One of Marie's talking points, with its receipt.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PracticePointDto {
    pub position: i32,
    pub text: String,
    /// The exhibit phrase a human paired with this point. `None` renders the
    /// stored named-absence line — never a blank under the point.
    pub exhibit: Option<String>,
}

/// Everything the practice page needs, in one response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PracticeDeckPayload {
    pub scenario_id: Uuid,
    /// `S-5` — the handle a human reads aloud.
    pub code: String,
    /// The accusation, as the page titles itself.
    pub title: String,
    /// The deck, in seeded order. EMPTY is a legitimate state, not a failure:
    /// the page shows the stored "no practice deck yet — seed it" line.
    pub questions: Vec<PracticeQuestionDto>,
    pub points: Vec<PracticePointDto>,
    /// The composed last-session sentence, or the composed "no session yet" one.
    /// Always present, because the start screen always has that line.
    pub last_session_line: String,
    pub wording: PracticeWordingDto,
}

/// Which deck she chose, and how many.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartSessionRequest {
    /// `george` | `chuck` | `mixed`.
    pub who: String,
}

/// The session she is now in.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartSessionResponse {
    pub session_id: Uuid,
}

/// Her four self-check boxes, as she ticked them.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelfCheckDto {
    pub only_asked: bool,
    pub accepted_premise: bool,
    pub explained_unasked: bool,
    pub guessed: bool,
}

/// One answered question, on its way to the log.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnswerRequest {
    pub session_id: Uuid,
    pub question_id: Uuid,
    pub answer_text: String,
    pub dont_recall: bool,
}

/// How one answer ends: the mark she chose, and the boxes she ticked.
///
/// A separate request from [`AnswerRequest`] because both of these happen AFTER
/// she has read the reveal — see `repositories::…::practice::close_answer` for
/// why the row is created first and settled second.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CloseAnswerRequest {
    /// `fine` or `repeat`. `repeat` is what "Ask me this one again later" sends.
    pub mark: String,
    pub self_check: SelfCheckDto,
}

/// What the reveal screen shows about the read.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnswerResponse {
    /// The answer row's id, which the drawer's help flag addresses.
    pub answer_id: Uuid,
    /// The one sentence, or `None` — in which case the screen shows the stored
    /// "no system read this time" line and every other box stands.
    pub read_text: Option<String>,
    /// `Some(true)` = fine (green), `Some(false)` = it named a tactic (red),
    /// `None` = there was no read. Three states, never two.
    pub read_ok: Option<bool>,
}

/// One row of Chuck's sheet, every cell already a word.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PracticeSheetRowDto {
    pub number: usize,
    /// "George", "George · braid" or "Chuck" — composed from the stored rows.
    pub from: String,
    /// The tactic's name, or the stored dash.
    pub tactic: String,
    pub question: String,
    pub answer: String,
    /// The stored word: "fine" or "repeat".
    pub mark: String,
    /// True when she opened the drawer — the screen picks the stored word, and
    /// the boolean also decides the cell's emphasis.
    pub help_opened: bool,
    /// The stored word for that cell: "opened" or the dash.
    pub help: String,
}

/// Chuck's sheet, composed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PracticeSheetPayload {
    /// "Session done · S-5 · Mon 17 Aug".
    pub kicker: String,
    /// "Six questions. Two to repeat." — one sentence, both clauses stored.
    pub heading: String,
    pub rows: Vec<PracticeSheetRowDto>,
}
