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
pub const PRACTICE_ROW_WORDING_KEYS: &[&str] = &[
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
    })
}

#[cfg(test)]
#[path = "wording_practice_row_tests.rs"]
pub(crate) mod tests;
