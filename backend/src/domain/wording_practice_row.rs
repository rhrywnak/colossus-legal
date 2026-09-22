// =============================================================================
// backend/src/domain/wording_practice_row.rs — the words about ONE question
// =============================================================================
//
// What CC_TASK_PRACTICE_V1_CHUCK_REVIEW_v1 Part A adds (items A2, A3, A5, A7):
// the way into a single question, what happened to it last time, what KIND of
// question it is, and what Marie said she would point to when she answered it.
//
// ## Why a fourth practice block and not more fields on the other three
//
// Rule 17, first: `wording_practice` (the drill), `wording_practice_flow`
// (navigation) and `wording_practice_report` (the reveal and the sheet) are all
// near the 300-line limit, and this task has no business splitting one of them
// on a deadline afternoon.
//
// But the seam is real. Every other practice block speaks about a SITTING —
// choosing a side, moving through a queue, reading the sheet at the end. These
// speak about ONE QUESTION, independently of any sitting: `answered today ·
// repeat` is true of a row whether or not she is in a session, `Practice this
// one ▸` opens a sitting that exists only for that question, `redirect` is a
// fact about the question itself, and `You'd point to:` names what she reached
// for on one answer. They are also the strings Chuck's Thursday review will move
// first, which is the practical reason to keep them together.
//
// ## Where each of these is composed
//
// The three status templates are filled SERVER-side (`services::practice_page`)
// and arrive on the deck row as a finished sentence, which is the same law the
// last-session line follows: the browser holds no templates, so a change to how
// a status reads is a Settings edit. The other six are labels the components
// render directly.

/// The words one deck row, and one answer, speak.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PracticeRowWording {
    // ── The way into one question ────────────────────────────────────────
    /// The control on a row that opens a one-question sitting. The question
    /// text on the row is the same link; this is its visible half, for a reader
    /// who does not know the text is clickable.
    pub practice_this_label: String,

    // ── What happened to this question ───────────────────────────────────
    /// `answered today · {mark}` — the status under a row answered TODAY.
    /// `{mark}` is the stored mark word, so the row and Chuck's sheet speak one
    /// vocabulary.
    pub answered_today_template: String,
    /// The status under a row whose newest attempt today was a mid-sitting
    /// skip. Domain note: distinct from `Skip today` on the start card, which
    /// writes no answer row at all — one is a question she was dealt and set
    /// aside, the other is one she was never dealt.
    pub skipped_today: String,
    /// `last: {when} · {mark}` — the status when the newest attempt was on an
    /// earlier day. Named as a date because the TENSE is what she needs.
    pub earlier_template: String,
    /// `· attempt {n}`, appended only above one attempt. Withdrawn at one: an
    /// "attempt 1" on every row is noise, and the number only means something
    /// once it is above one.
    pub attempt_suffix_template: String,

    // ── The redirect ─────────────────────────────────────────────────────
    /// The small tag beside the Chuck pill on a redirect question. It wears
    /// Chuck's pill because Chuck asks it; the tag says why.
    pub redirect_tag: String,
    /// What the stronger-answer drawer shows on a REDIRECT carrying no stored
    /// example. Domain note: the honest "no receipt for this one — that's a
    /// Chuck question" line is WRONG here (task A5). A redirect is not a
    /// question somebody forgot to write an answer for; it is the one place in
    /// the drill where length is the right answer, and the drawer says so.
    pub redirect_stronger_line: String,

    // ── "I'd point to…" ──────────────────────────────────────────────────
    /// Opens this scenario's receipts under the answer box.
    pub points_to_label: String,
    /// Folds the receipt list again. A fold and not a save: what she picked
    /// rides with the answer, so there is no separate write to lose.
    pub points_to_done_label: String,
    /// Introduces the picked receipts on the reveal, in the second person.
    pub points_to_reveal_prefix: String,
    /// The same list on Chuck's sheet, in the third person — he is reading
    /// about her, not to her.
    pub points_to_sheet_prefix: String,

    // ── The unfinished sitting ───────────────────────────────────────────
    /// Stands where the date goes in the unfinished-session line when the
    /// sitting was started today. "today 09:57" is what a person says.
    pub unfinished_today_word: String,
    /// Why Answer is disabled on an empty box. It names the OTHER control
    /// deliberately: an "I don't recall." is a complete answer and stays one click.
    pub answer_empty_hint: String,
    /// Shown when a second tab answers a question the first already answered.
    /// It names the CAUSE, not a fault: two tabs is a thing a person does.
    pub answer_already_recorded: String,

    /// `Answered on {when}` — the ONE status a one-page deck row carries.
    ///
    /// ## Domain note: absent is absent, and renders nothing
    ///
    /// A question nobody has answered renders no line at all, not this template
    /// with an empty `{when}`. An empty status line under a question reads as a
    /// status that failed to load, which is a different fact from "not answered
    /// yet" and the wrong one to show the person least able to diagnose it.
    pub answered_on_template: String,

    // ── The answer box says it is saved (CC_TASK_PRACTICE_FIXES_v2.2.1) ──
    /// `Your answer — saved {when}` — the label over the answer box once an
    /// answer is saved. Filled SERVER-side (`practice_page::answer_saved_label`)
    /// with the day AND time in the case's timezone.
    ///
    /// Domain note: the box is pre-filled with her current answer, and on
    /// v2.2.0 nothing said so — she pressed Answer four times and read "nothing
    /// happened" (DEV, 2026-09-21). The label is what makes a pre-filled box
    /// read as "this is already on file" rather than as a draft.
    pub answer_saved_template: String,
    /// The one line under the buttons after a press with Answer analysis OFF.
    /// With analysis on the read itself is the confirmation, so this never shows.
    pub answer_saved_off_line: String,

    // ── The review loop (CC_TASK_REVIEW_LOOP_v1) ─────────────────────────
    //
    // Domain note: two audiences, one question. Chuck writes a note on the
    // answer he is reading and marks the deck reviewed; Marie reads the note
    // under her deck row. Both are about ONE question's current answer, which
    // is why they file here and not with the sitting blocks.
    /// The review bar under the deck title, the same for everyone: `{count}`
    /// answers awaiting `{reviewer}`'s review (CC_TASK_SIMPLE_COUNTS_v1).
    pub deck_review_awaiting_template: String,
    /// Its singular, read when the count is exactly 1.
    pub deck_review_awaiting_one: String,
    /// `· oldest waiting since {date}` — appended to either sentence above when
    /// the read returned a date (CC_TASK_REVIEW_PAGE_v1, ruling STOP-A).
    ///
    /// Domain note: the count alone does not say whether this is a backlog or
    /// this morning's work, and those call for different afternoons. The date
    /// comes from the SAME query as the count (`awaiting_review`'s
    /// `MIN(…) FILTER (…)`), so the bar cannot name a day the count excluded.
    /// Withheld entirely when there is no date rather than rendered empty.
    pub deck_review_oldest_template: String,
    /// The review bar's button. Opens the confirmation; it does not write.
    pub deck_review_done_label: String,
    /// The confirmation's question, for a count that is not 1 —
    /// `Mark all {count} answers in {code} as reviewed?`
    ///
    /// Domain note: the press moves ONE SHARED mark for the whole reviewer
    /// bench and there is no undo, so the question names both the number and
    /// the deck. Until v2.1.15 there was no question at all: one click on Done
    /// reviewing and the mark had moved for everybody.
    pub deck_review_confirm_template: String,
    /// Its singular, read when exactly one answer waits (`pickByCount`).
    ///
    /// Domain note: a separate row rather than a plural rule, for the reason
    /// the awaiting pair gives — this is the one screen whose whole job is to
    /// be read before an irreversible click, and "1 answers" is not a sentence
    /// to put in front of a lawyer at that moment.
    pub deck_review_confirm_one: String,
    /// The affirmative, and the ONLY control that moves the mark.
    ///
    /// Domain note: a question is answered Yes, never restated as the verb
    /// (Roman, 2026-09-20) — a second "Done reviewing" here would ask the
    /// reader to confirm a sentence they have just read.
    pub deck_review_confirm_yes_label: String,
    /// The retreat. Sends nothing; Escape does the same.
    pub deck_review_confirm_cancel_label: String,
    /// Shown when Done reviewing fails; the count stays as it was.
    pub deck_review_failed: String,
    /// Opens the note box on the answers page.
    pub note_add_label: String,
    /// Writes the note.
    pub note_save_label: String,
    /// Closes the note box without writing.
    pub note_cancel_label: String,
    /// Withdraws a note (it stays visible, struck through).
    pub note_strike_label: String,
    /// `struck {when}` — composed server-side under a withdrawn note.
    pub note_struck_template: String,
    /// Shown when a note write or strike fails.
    pub note_failed: String,
    /// Shown when opening this question FROM the For you list could not mark it
    /// read (CC_TASK_FOR_YOU_v1 L1).
    ///
    /// Domain note: the question itself is on screen and readable — only the
    /// bookkeeping failed. So this is a line, not a barrier: it says the row is
    /// still on her list, which is the true consequence, rather than implying
    /// the question failed to load.
    pub row_read_failed: String,
}

pub(crate) const KEY_PRACTICE_THIS_LABEL: &str = "practice_row_practice_this_label";
pub(crate) const KEY_ANSWERED_TODAY_TEMPLATE: &str = "practice_row_answered_today_template";
pub(crate) const KEY_SKIPPED_TODAY: &str = "practice_row_skipped_today";
pub(crate) const KEY_EARLIER_TEMPLATE: &str = "practice_row_earlier_template";
pub(crate) const KEY_ATTEMPT_SUFFIX_TEMPLATE: &str = "practice_row_attempt_suffix_template";
pub(crate) const KEY_REDIRECT_TAG: &str = "practice_redirect_tag";
pub(crate) const KEY_REDIRECT_STRONGER_LINE: &str = "practice_redirect_stronger_line";
pub(crate) const KEY_POINTS_TO_LABEL: &str = "practice_points_to_label";
pub(crate) const KEY_POINTS_TO_DONE_LABEL: &str = "practice_points_to_done_label";
pub(crate) const KEY_POINTS_TO_REVEAL_PREFIX: &str = "practice_points_to_reveal_prefix";
pub(crate) const KEY_POINTS_TO_SHEET_PREFIX: &str = "practice_points_to_sheet_prefix";
pub(crate) const KEY_UNFINISHED_TODAY_WORD: &str = "practice_unfinished_today_word";

/// Every key in this block, so a missing one is caught at boot BY NAME rather
/// than as a blank control in front of Marie mid-session.
pub(crate) const KEY_ANSWER_EMPTY_HINT: &str = "practice_answer_empty_hint";

pub(crate) const KEY_ANSWER_ALREADY_RECORDED: &str = "practice_answer_already_recorded";
pub(crate) const KEY_ANSWERED_ON_TEMPLATE: &str = "practice_row_answered_on_template";
pub(crate) const KEY_ANSWER_SAVED_TEMPLATE: &str = "practice_row_answer_saved_template";
pub(crate) const KEY_ANSWER_SAVED_OFF_LINE: &str = "practice_row_answer_saved_off_line";

pub(crate) const KEY_DECK_REVIEW_AWAITING_TEMPLATE: &str = "practice_deck_review_awaiting_template";
pub(crate) const KEY_DECK_REVIEW_AWAITING_ONE: &str = "practice_deck_review_awaiting_one";
pub(crate) const KEY_DECK_REVIEW_OLDEST_TEMPLATE: &str = "practice_deck_review_oldest_template";
pub(crate) const KEY_DECK_REVIEW_DONE_LABEL: &str = "practice_deck_review_done_label";
pub(crate) const KEY_DECK_REVIEW_CONFIRM_TEMPLATE: &str = "practice_deck_review_confirm_template";
pub(crate) const KEY_DECK_REVIEW_CONFIRM_ONE: &str = "practice_deck_review_confirm_one";
pub(crate) const KEY_DECK_REVIEW_CONFIRM_YES_LABEL: &str = "practice_deck_review_confirm_yes_label";
pub(crate) const KEY_DECK_REVIEW_CONFIRM_CANCEL_LABEL: &str =
    "practice_deck_review_confirm_cancel_label";
pub(crate) const KEY_DECK_REVIEW_FAILED: &str = "practice_deck_review_failed";
pub(crate) const KEY_NOTE_ADD_LABEL: &str = "practice_row_note_add_label";
pub(crate) const KEY_NOTE_SAVE_LABEL: &str = "practice_row_note_save_label";
pub(crate) const KEY_NOTE_CANCEL_LABEL: &str = "practice_row_note_cancel_label";
pub(crate) const KEY_NOTE_STRIKE_LABEL: &str = "practice_row_note_strike_label";
pub(crate) const KEY_NOTE_STRUCK_TEMPLATE: &str = "practice_row_note_struck_template";
pub(crate) const KEY_NOTE_FAILED: &str = "practice_row_note_failed";
pub(crate) const KEY_ROW_READ_FAILED: &str = "practice_row_read_failed";

pub const PRACTICE_ROW_WORDING_KEYS: &[&str] = &[
    KEY_ANSWER_ALREADY_RECORDED,
    KEY_ANSWER_EMPTY_HINT,
    KEY_ANSWERED_ON_TEMPLATE,
    KEY_ANSWER_SAVED_TEMPLATE,
    KEY_ANSWER_SAVED_OFF_LINE,
    KEY_PRACTICE_THIS_LABEL,
    KEY_ANSWERED_TODAY_TEMPLATE,
    KEY_SKIPPED_TODAY,
    KEY_EARLIER_TEMPLATE,
    KEY_ATTEMPT_SUFFIX_TEMPLATE,
    KEY_REDIRECT_TAG,
    KEY_REDIRECT_STRONGER_LINE,
    KEY_POINTS_TO_LABEL,
    KEY_POINTS_TO_DONE_LABEL,
    KEY_POINTS_TO_REVEAL_PREFIX,
    KEY_POINTS_TO_SHEET_PREFIX,
    KEY_UNFINISHED_TODAY_WORD,
    KEY_DECK_REVIEW_AWAITING_TEMPLATE,
    KEY_DECK_REVIEW_AWAITING_ONE,
    KEY_DECK_REVIEW_OLDEST_TEMPLATE,
    KEY_DECK_REVIEW_DONE_LABEL,
    KEY_DECK_REVIEW_CONFIRM_TEMPLATE,
    KEY_DECK_REVIEW_CONFIRM_ONE,
    KEY_DECK_REVIEW_CONFIRM_YES_LABEL,
    KEY_DECK_REVIEW_CONFIRM_CANCEL_LABEL,
    KEY_DECK_REVIEW_FAILED,
    KEY_NOTE_ADD_LABEL,
    KEY_NOTE_SAVE_LABEL,
    KEY_NOTE_CANCEL_LABEL,
    KEY_NOTE_STRIKE_LABEL,
    KEY_NOTE_STRUCK_TEMPLATE,
    KEY_NOTE_FAILED,
    KEY_ROW_READ_FAILED,
];

/// Build a [`PracticeRowWording`] from the stored rows, or say which key is
/// wrong.
///
/// ## Rust Learning: taking the reader by reference
///
/// The caller is [`super::wording_practice::build_practice_wording`], which owns
/// its own `read` closure and still needs it afterwards. `&F` implements `Fn`
/// whenever `F` does, so one closure serves all the nested blocks without being
/// cloned — and every block is judged by exactly the same rule.
///
/// # Errors
/// Returns whatever `read` returns for the first key that is missing, of the
/// wrong declared kind, or blank.
pub fn build_practice_row_wording<E>(
    read: impl Fn(&str) -> Result<String, E>,
) -> Result<PracticeRowWording, E> {
    Ok(PracticeRowWording {
        answer_already_recorded: read(KEY_ANSWER_ALREADY_RECORDED)?,
        answered_on_template: read(KEY_ANSWERED_ON_TEMPLATE)?,
        answer_saved_template: read(KEY_ANSWER_SAVED_TEMPLATE)?,
        answer_saved_off_line: read(KEY_ANSWER_SAVED_OFF_LINE)?,
        answer_empty_hint: read(KEY_ANSWER_EMPTY_HINT)?,
        practice_this_label: read(KEY_PRACTICE_THIS_LABEL)?,
        answered_today_template: read(KEY_ANSWERED_TODAY_TEMPLATE)?,
        skipped_today: read(KEY_SKIPPED_TODAY)?,
        earlier_template: read(KEY_EARLIER_TEMPLATE)?,
        attempt_suffix_template: read(KEY_ATTEMPT_SUFFIX_TEMPLATE)?,
        redirect_tag: read(KEY_REDIRECT_TAG)?,
        redirect_stronger_line: read(KEY_REDIRECT_STRONGER_LINE)?,
        points_to_label: read(KEY_POINTS_TO_LABEL)?,
        points_to_done_label: read(KEY_POINTS_TO_DONE_LABEL)?,
        points_to_reveal_prefix: read(KEY_POINTS_TO_REVEAL_PREFIX)?,
        points_to_sheet_prefix: read(KEY_POINTS_TO_SHEET_PREFIX)?,
        unfinished_today_word: read(KEY_UNFINISHED_TODAY_WORD)?,
        deck_review_awaiting_template: read(KEY_DECK_REVIEW_AWAITING_TEMPLATE)?,
        deck_review_awaiting_one: read(KEY_DECK_REVIEW_AWAITING_ONE)?,
        deck_review_oldest_template: read(KEY_DECK_REVIEW_OLDEST_TEMPLATE)?,
        deck_review_done_label: read(KEY_DECK_REVIEW_DONE_LABEL)?,
        deck_review_confirm_template: read(KEY_DECK_REVIEW_CONFIRM_TEMPLATE)?,
        deck_review_confirm_one: read(KEY_DECK_REVIEW_CONFIRM_ONE)?,
        deck_review_confirm_yes_label: read(KEY_DECK_REVIEW_CONFIRM_YES_LABEL)?,
        deck_review_confirm_cancel_label: read(KEY_DECK_REVIEW_CONFIRM_CANCEL_LABEL)?,
        deck_review_failed: read(KEY_DECK_REVIEW_FAILED)?,
        note_add_label: read(KEY_NOTE_ADD_LABEL)?,
        note_save_label: read(KEY_NOTE_SAVE_LABEL)?,
        note_cancel_label: read(KEY_NOTE_CANCEL_LABEL)?,
        note_strike_label: read(KEY_NOTE_STRIKE_LABEL)?,
        note_struck_template: read(KEY_NOTE_STRUCK_TEMPLATE)?,
        note_failed: read(KEY_NOTE_FAILED)?,
        row_read_failed: read(KEY_ROW_READ_FAILED)?,
    })
}

#[cfg(test)]
#[path = "wording_practice_row_tests.rs"]
pub(crate) mod tests;
