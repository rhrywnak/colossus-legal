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
    /// The tactic's CARD NUMBER, 1–7, or `None` when it carries none.
    ///
    /// ## Why both this and `tactic`, which is the same fact
    ///
    /// They are not the same fact. `tactic` is what a READER sees — a name, with
    /// the braid suffix appended when the question braids — and `tactic_card` is
    /// what an EDITOR sets. The deck editor's dropdown cannot be built from the
    /// name: `compound · braid` is not a card, and matching it back to a number
    /// in the browser would be a second, weaker copy of the resolution
    /// `practice_page::tactic_name` already does. So the name ships for the pill
    /// and the number ships for the form, and neither is derived from the other
    /// anywhere but here.
    ///
    /// ## Rust Learning: `Option<i16>` rather than a sentinel
    ///
    /// The column is `SMALLINT` with `CHECK (tactic IS NULL OR tactic BETWEEN 1
    /// AND 7)`, so "no card" is a NULL and not a zero. `Option<i16>` carries that
    /// across the wire as `null` — a value the browser can test for — where a `0`
    /// would be a magic number both sides had to agree to ignore.
    pub tactic_card: Option<i16>,
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
    /// `cross`, `direct` or `redirect` — what the question DOES. The screen tags
    /// a redirect, the mixed queue pairs one behind the George question it
    /// answers, and the read judges it by a different rule. Three behaviours
    /// that `side` cannot carry.
    pub kind: String,
    /// The stable handle the deck file uses (`g1`, `r2`). `None` on rows seeded
    /// before the key existed.
    pub deck_key: Option<String>,
    /// The `deck_key` of the George question this redirect answers, or `None`.
    /// The browser pairs the queue by it; nothing else reads it.
    pub follows_key: Option<String>,
    /// True when the deck editor has hidden this question. Marie's list and
    /// every queue drop it; the editor still shows it, greyed, so it can be put
    /// back. Never deleted.
    pub hidden: bool,
    /// Who drafted this question when nobody has reviewed it (`architect`), or
    /// `None`. The editor shows a draft badge while it is set.
    pub draft_by: Option<String>,
    /// The notes Marie reads under this row: on the question, or on its CURRENT
    /// answer, oldest first, struck ones included (struck through, with their
    /// date). Empty on the answers page's own question read (CC_TASK_REVIEW_LOOP_v1).
    pub notes: Vec<super::practice_review::PracticeNoteDto>,
    /// The id of this question's CURRENT answer, or `None` when nobody has
    /// answered it.
    ///
    /// Carried so a page that renders the row can name what it showed when the
    /// reviewer presses Done — see `ReviewSweepRequest`. It is an id and not a
    /// count: the sweep marks exactly these rows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answer_id: Option<uuid::Uuid>,
    /// `Chuck left a note` — what is waiting on this question FOR THE PERSON
    /// READING, already composed (CC_TASK_FOR_YOU_v1 L3, mockup board 4), or
    /// `None` when nothing on it is unread by them.
    ///
    /// ## Domain note: two readers of one deck see two different rows
    ///
    /// Read-state is per person (L2), so this field is too. It is absent rather
    /// than blank when nothing waits: an empty mark beside a question reads as
    /// a tag that failed to load.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waiting: Option<String>,
    /// `Answered on 22 Aug`, ALREADY COMPOSED — or `None` when nobody has
    /// answered this question.
    ///
    /// ## Why this is beside `status` rather than instead of it
    ///
    /// `status` is the drill's line — `answered today · repeat · attempt 2` —
    /// and it is scoped to THIS user's sittings, because it reports what SHE
    /// did. This one is scoped to the scenario and carries no mark, because the
    /// one-page deck row is read by two people: Chuck opens it to find her
    /// answers and to print them, and a line scoped to the requester would tell
    /// him every row was unanswered.
    ///
    /// Both ship while the two pages coexist. `status` retires with the sitting
    /// apparatus in L2 — it is not deleted here, because a field removed from
    /// the wire while a screen still reads it is a blank line on that screen.
    pub answered_on: Option<String>,
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
    /// When any question in this deck last changed — `MAX(updated_at)`.
    ///
    /// ## Domain note: the DECK's own date, and what moves it
    ///
    /// Every editor write sets `updated_at = NOW()` (`practice_editor`: the field
    /// edit, the tactic, the reorder and the hide), and a seeded row takes the
    /// column's `DEFAULT NOW()`. So this is the deck's own last change and not
    /// something coarser — but it DOES move on a pure re-order, which changes the
    /// deck without changing any question's words.
    ///
    /// `None` on a deck with no questions, which is a legitimate state: the print
    /// control is disabled there and nothing renders this.
    pub deck_as_of: Option<chrono::DateTime<chrono::Utc>>,
    /// When the SERVER produced this payload.
    ///
    /// ## Domain note: one clock, handed out and handed back
    ///
    /// "Done reviewing" sends back the items the page showed — and this, so the
    /// sweep can also clear the one item no row can name: a note about the whole
    /// scenario. Bounding that by the moment the page was SERVED keeps the
    /// ruling's promise (nothing that arrived after the page loaded is swept)
    /// without trusting a browser's clock, which may be minutes out.
    pub served_at: chrono::DateTime<chrono::Utc>,
    /// The receipts the "I'd point to…" picker offers, de-duplicated and in
    /// deck order: her points' receipts first, then the exhibits her questions
    /// stand on. Composed here so the browser assembles no list of its own.
    pub receipts: Vec<String>,
    /// The sitting she walked out of, if there is one. `None` withdraws the
    /// blue box entirely — an empty box would read as a session that failed to
    /// load.
    pub open_session: Option<OpenSessionDto>,
    /// The notes on this SCENARIO, oldest first. Question- and attempt-level
    /// notes ride the review payload instead.
    /// What changed since her last sitting. `None` withdraws the blue box.
    /// What the editor's add form may attach a new question to — this
    /// scenario's ruled instances and talking points, already labelled.
    pub attach_options: Vec<super::practice_review::PracticeAttachOptionDto>,
    /// The seven TACTIC_DECK_v1 cards, in card order — the dropdown's options.
    ///
    /// ## ⚑ ONE AUTHORITY, and this is it reaching the browser
    ///
    /// These are `settings.practice_read.tactic_names`, the same row
    /// `practice_page::tactic_name` indexes into for the pills. The editor's
    /// dropdown is built from this list rather than from a list of its own, so a
    /// card renamed in the store renames it in the tag and in the form at once.
    /// A second list in the frontend would disagree the first time one was
    /// edited, and the disagreement would be invisible: both would render.
    ///
    /// Shorter than seven when somebody has trimmed the settings row — the same
    /// state `tactic_name` answers with no tag at all. The dropdown then offers
    /// what the vocabulary can name and nothing else.
    pub tactic_cards: Vec<TacticCardDto>,
    /// The review bar: the reviewer's backlog on this deck, whether THIS user may
    /// press Done reviewing, and the reviewer's display name
    /// (CC_TASK_SIMPLE_COUNTS_v1).
    pub review: super::practice_review::DeckReviewDto,
    pub wording: PracticeWordingDto,
}

/// One TACTIC_DECK_v1 card: the number that is stored, the name that is shown.
///
/// A pair rather than a bare list of names because the POSITION is the stored
/// value — card 3 is `false premise` only for as long as the settings row lists
/// it third — and an option element that carried the name alone would make the
/// browser count the list to work out what to send.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TacticCardDto {
    /// 1–7. What `practice_questions.tactic` stores.
    pub card: i16,
    /// The card's name as the store holds it. Never the braid suffix: that is
    /// composed for the TAG, and a form must not offer it as a choice.
    pub name: String,
}

/// The unfinished sitting the start card offers back.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenSessionDto {
    pub session_id: Uuid,
    /// `· today 09:57 · George's side · 1 of 5 answered.` — one composed
    /// sentence, so the browser holds no template and no date format.
    pub detail: String,
}

/// One sitting, as the page re-enters it at its own address.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SittingPayload {
    pub session_id: Uuid,
    pub scenario_id: Uuid,
    /// `george` | `chuck` | `mixed`.
    pub who: String,
    /// The dealt question ids, in the order they were dealt. EMPTY when the
    /// session predates the stored queue — the page then says it cannot resume
    /// rather than dealing a queue it invented.
    pub queue: Vec<Uuid>,
    /// The questions already dealt, in the order she answered them. A `skipped`
    /// row counts: she was shown that question and set it aside.
    pub answered: Vec<Uuid>,
    /// True when this sitting is already closed. The page then shows the start
    /// card rather than re-entering a session that is over.
    pub ended: bool,
    /// The queued ids that have since been HIDDEN from the deck.
    ///
    /// The sitting walks past these rather than asking them: Chuck took the
    /// question out while this sitting still had it waiting, and asking it
    /// anyway is what .402 did. A subset of `queue`, never anything else, and
    /// normally empty. Ending the sitting writes each one a sheet row marked
    /// `hidden before asked` — see `practice_hidden_queue`.
    pub hidden: Vec<Uuid>,
}

/// What closing the stale open sittings did.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenSessionsClosed {
    /// How many OTHER open sittings this scenario carried and now does not.
    pub also_closed: u64,
}

/// Which deck she chose, and how many.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartSessionRequest {
    /// `george` | `chuck` | `mixed`.
    pub who: String,
    /// The question ids this sitting will be dealt, IN ORDER.
    ///
    /// ## Why the browser sends the queue rather than the server drawing it
    ///
    /// The ORDER is the drill (George · Chuck · George — the shape of a real
    /// day), and it is composed on the screen that also knows what she kept out
    /// today. Sending it means the server stores the sitting she actually
    /// started, which is what a reload has to resume and what `Ended early.`
    /// is measured against. The server still FENCES it: every id must belong to
    /// this scenario's deck.
    #[serde(default)]
    pub queue: Vec<Uuid>,
    /// What she chose off the count pills. Kept because the queue can grow past
    /// it — "ask me this one again later" appends — and the choice is still
    /// worth knowing afterwards.
    #[serde(default)]
    pub count: Option<i32>,
    /// The ids she kept out of this sitting on the start screen. For the record:
    /// they were never dealt, so they are not in `queue`.
    #[serde(default)]
    pub skipped_today: Vec<Uuid>,
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
    /// Whether to ask a model to read this answer. `false` means NO CALL.
    ///
    /// ## Why the browser decides this and the server obeys it
    ///
    /// Saving an answer and reading it are one request — see
    /// `api::practice_answers::post_practice_answer`, whose three steps write the
    /// row, call the model and attach the result. A browser that does not want
    /// the read therefore cannot decline it by staying silent: the call is on the
    /// far side of the request it has to make to save at all. So the wish travels
    /// WITH the answer, and "Answer analysis · off" on the practice bar means no
    /// model is asked — not a model asked and its answer discarded.
    ///
    /// ## Rust Learning: `#[serde(default = "…")]` and why the default is TRUE
    ///
    /// `#[serde(default)]` would give `bool`'s own default, `false`, and silently
    /// turn the read off for any caller that predates this field. A named default
    /// function makes ABSENT mean what it meant yesterday — a read was wanted —
    /// so the new field changes behaviour only for a caller that asked it to.
    #[serde(default = "want_read_by_default")]
    pub want_read: bool,
    /// The receipts she said she would point to. ABSENT when she never opened
    /// the control, an empty array when she opened it and picked nothing —
    /// two different facts, and the column keeps them different.
    #[serde(default)]
    pub points_to: Option<Vec<String>>,
}

/// What an [`AnswerRequest`] with no `want_read` field means: ask for the read.
///
/// The behaviour every caller had before the switch existed. See the field's own
/// doc for why this is a function and not `#[serde(default)]`.
fn want_read_by_default() -> bool {
    true
}

/// One question she was dealt and set aside mid-sitting.
///
/// A request of its own rather than a flag on [`AnswerRequest`], because it is a
/// different act with a different cost: it makes NO model call, stores the
/// stored "doesn't fit" phrase rather than anything she typed, and lands on the
/// row already marked `skipped`. Folding it into the answer path would put a
/// `if skipped { … }` around the read, the mark and the text at once — three
/// branches whose only shared code is the insert.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkipQuestionRequest {
    pub session_id: Uuid,
    pub question_id: Uuid,
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
pub struct ReadPartsDto {
    /// The one line naming what happened.
    pub call: String,
    /// The reasoning. Empty is legitimate.
    pub why: String,
    /// What to do instead, in the order the model gave them. 0–3.
    pub pointers: Vec<String>,
    /// The citation keys used — every one proven to be a key that was sent.
    pub keys: Vec<String>,
}

/// One citable source, as the critique's footnote list shows it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadSourceDto {
    /// `S2`, `R1`, `P3` — the key as the critique cites it.
    pub key: String,
    /// The words behind that key.
    pub text: String,
}

/// What one answer produced.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnswerResponse {
    /// The answer row's id, which the drawer's help flag addresses.
    pub answer_id: Uuid,
    /// The one sentence, or `None` — in which case the screen shows the stored
    /// `practice_read_unavailable` line ("No answer analysis for this answer.") and every other box stands.
    pub read_text: Option<String>,
    /// `Some(true)` = fine (green), `Some(false)` = it named a tactic (red),
    /// `None` = there was no read. Three states, never two.
    pub read_ok: Option<bool>,
    /// The critique's three parts, when the read produced three parts.
    ///
    /// ## ⚑ ADDITIVE. T1's read is unchanged.
    ///
    /// The parts have existed since T1 — `ReadParts`, stored in `read_call`,
    /// `read_why`, `read_pointers` and `read_keys` — and were simply never put
    /// on the wire, because the screen that consumed this response rendered one
    /// composed sentence. Mockup v7 view 4 draws them separately, so they ship.
    /// Nothing about what the model is SENT changes; `read_text` still carries
    /// the composed line for anything that wants it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_parts: Option<ReadPartsDto>,
    /// What the keys in `read_parts.keys` refer to — the critique's footnotes.
    ///
    /// Domain note: a critique that cites `S2` with no way to see what S2 SAYS
    /// is worse than the single sentence it replaced. Citing receipts by key is
    /// the whole of what T1 bought, and the key is only half of it.
    pub read_sources: Vec<ReadSourceDto>,
    /// The answer box's new label — `Your answer — saved {when}` for the row
    /// this press left standing — so the label moves the moment the press
    /// returns, with no refetch.
    ///
    /// `None` only on a SKIP, which puts no answer in the box. On an unchanged
    /// re-press it carries the ORIGINAL time: no version was written, and a
    /// label claiming a fresh save would be untrue.
    pub saved_label: Option<String>,
    /// `Saved. Answer analysis is off, so no analysis was requested.` — present
    /// ONLY when the press asked for no read.
    ///
    /// Domain note: decided HERE because the server is what knows no model was
    /// asked (CLAUDE.md rule 12). On v2.2.0 an analysis-off press returned
    /// nothing visible at all, and "nothing happened" was a fair reading of it.
    pub saved_line: Option<String>,
    /// `true` when the answer analysis FAILED for a system reason
    /// (`practice_read_outcome::is_failed_read`) — `read_text` is then the one
    /// failure line, and the screen draws it on a neutral rail with no hint
    /// (ADDENDUM_1). `false` for a judgement, a model's decline, or no read.
    pub read_failed: bool,
    /// `true` when the MODEL declined to judge the answer
    /// (`practice_read_outcome::is_declined_read`) — `read_text` is the abstain
    /// line plus the model's sentence, drawn on a neutral rail with no hint
    /// (ADDENDUM_3). Never true together with `read_failed`.
    pub read_declined: bool,
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
    /// What she said she would point to. EMPTY withdraws the line — a "would
    /// point to:" with nothing after it reads as data that went missing.
    pub points_to: Vec<String>,
}

/// One change the sheet's footer prints. See `services::practice_changes`.
pub type SheetChangeLine = String;

/// Chuck's sheet, composed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PracticeSheetPayload {
    /// "Session done · S-5 · Mon 17 Aug".
    pub kicker: String,
    /// "Six questions. Two to repeat." — one sentence, both clauses stored.
    pub heading: String,
    pub rows: Vec<PracticeSheetRowDto>,
    /// The deck's flagged questions, already composed into the sentence the
    /// sheet prints. EMPTY withdraws the whole block, heading included — a
    /// heading over nothing reads as a list that failed to load.
    pub flagged: Vec<String>,
    /// The block's heading and its one sentence, so the browser composes none of
    /// it. Both empty when `flagged` is.
    pub flagged_heading: String,
    pub flagged_hint: String,
    /// The deck changes made on the day of this sitting, already composed
    /// (task B2: "Chuck's sheet footer lists the changes made that day"). EMPTY
    /// withdraws the block, heading included.
    pub changes: Vec<SheetChangeLine>,
    /// That block's heading. Empty when `changes` is.
    pub changes_heading: String,
}

/// One question's current answer, for the printed answers sheet and the Review
/// answers page.
///
/// ## Domain note: the CURRENT answer only
///
/// Not the earlier versions. Chuck is reading what Marie would say today; a
/// sheet carrying three versions of one answer asks him to work out which is
/// live, which is the one job the screen already does for him.
///
/// ## Why two more fields arrived on 2026-09-19
///
/// The Review answers page needs to WRITE on what it reads — a note lands on
/// the answer that stands now, which needs its id — and it prints who wrote it.
/// Both are additive: the print sheet reads neither and is unchanged.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PracticeAnswerDto {
    pub question_id: Uuid,
    /// The standing answer's own id — what a note written on the review page
    /// attaches to, and what tells a note on THIS answer from one on a
    /// superseded attempt.
    pub answer_id: Uuid,
    /// Her words, exactly as typed.
    pub text: String,
    /// `Answered on 22 Aug`, already composed — the same line the deck row shows.
    pub answered_on: String,
    /// `Answered 14 Sep · Marie`, already composed — the review page's own line.
    ///
    /// A second composed line rather than a name the client would join: the
    /// browser holds no templates and no date format, so how this reads is a
    /// Settings edit (see `services::practice_page::answered_meta_line`).
    pub answered_meta: String,
}

/// Every current answer in one scenario, for the print-answers view.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PracticeAnswersPayload {
    pub answers: Vec<PracticeAnswerDto>,
}

/// One version of a question's answer, as the question page receives it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnswerVersionDto {
    pub answer_id: Uuid,
    /// Her words, exactly as typed.
    pub text: String,
    /// `Answered on 22 Aug`, already composed — the same line the row shows.
    pub answered_on: String,
    /// `Your answer — saved Wed 21 Sep · 10:54 pm`, already composed — the
    /// label over the answer box when this is the CURRENT version
    /// (`practice_page::answer_saved_label`). Sent on every version so the
    /// field has one meaning; the page reads it only from `current`.
    pub saved_label: String,
    /// The notes on THIS attempt, oldest first, struck ones included
    /// (CC_TASK_REVIEW_LOOP_v1 §4).
    pub notes: Vec<super::practice_review::PracticeNoteDto>,
}

/// One question's answer history: what stands now, and what came before.
///
/// ## Domain note: `earlier` is NOT editable, and the split says so
///
/// Two fields rather than one list with the current flagged, because the page
/// treats them as different things: `current` is pre-filled into a box she can
/// change, and `earlier` is a quiet line she never has to open. A single list
/// would leave the client deciding which is which, and a client that got it
/// wrong would offer her an edit box over an answer Chuck has already read.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestionAnswersPayload {
    /// What she would say today, or `None` if she has never answered.
    pub current: Option<AnswerVersionDto>,
    /// Everything before it, newest first. Empty when there is one answer or none.
    pub earlier: Vec<AnswerVersionDto>,
    /// Notes on the question itself rather than on one attempt, oldest first.
    pub question_notes: Vec<super::practice_review::PracticeNoteDto>,
}
