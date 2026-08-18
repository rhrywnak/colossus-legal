// =============================================================================
// backend/src/domain/wording_practice_flow.rs — the words that let her MOVE
// =============================================================================
//
// What mockup v3 adds (CC_TASK_PRACTICE_FLOW_V1_v1, 2026-08-18): the deck listed
// on the start card with its two per-row controls, the resume line for a sitting
// she walked out of, the top bar that lets her walk out of one, and the two
// clauses Chuck's sheet gains.
//
// ## Why a third practice block and not more fields on the first two
//
// Rule 17 measures code lines, and `wording_practice` (the drill) and
// `wording_practice_report` (the reveal and the sheet) are both near the limit.
// But the seam is not only arithmetic: these are the words of NAVIGATION —
// getting into a sitting, out of one, and back. They are the only practice
// strings addressed to a reader who is deciding whether to continue, and they
// are the ones Chuck's ruling on "should Marie see the deck at all" will move.
//
// ## Why this block hangs off `PracticeWording` rather than off `Settings`
//
// `Settings` sits at 299 non-comment lines. A field there and its `for_test`
// line would put it at 301 and break Rule 17 in a file this task has no business
// splitting. Nesting costs nothing: one field, built by the same closure, read
// through `settings.practice_wording.flow`. The alternative — splitting the
// settings struct on the afternoon of a deadline — is how an unrelated surface
// gets broken.

/// The words v3's controls speak.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PracticeFlowWording {
    // ── S0 · the deck, listed ────────────────────────────────────────────
    /// The bold label over the question list.
    pub deck_heading: String,
    /// `· {n} — {george} George's side · {chuck} Chuck`, filled from the
    /// questions the WHO filter is showing rather than from the whole deck.
    pub deck_count_template: String,
    /// `· {k} skipped today`, appended only when k > 0 — a separate row so the
    /// common case renders no empty clause and no stray separator.
    pub deck_skipped_suffix_template: String,
    /// Folds the list for this page-load only.
    pub deck_hide_link: String,
    /// The same link once folded.
    pub deck_show_link: String,
    /// The one sentence of instruction, with `{skip}` and `{flag}` filled from
    /// the two control labels below so the sentence cannot name a button that
    /// has been renamed.
    pub deck_instruction_template: String,

    // ── S0 · the two controls on a row ───────────────────────────────────
    /// Keeps a question out of THIS sitting.
    pub skip_today_label: String,
    /// The same control once the row is out.
    pub skipped_today_label: String,
    /// Opens the one-line note.
    pub flag_label: String,
    /// The same control once a note is stored.
    pub flag_edit_label: String,
    /// The placeholder in the flag input — it says who reads the note, which is
    /// the whole reason the control is not an edit box.
    pub flag_placeholder: String,
    /// Writes the note to the question.
    pub flag_save_label: String,
    /// Closes the input without writing.
    pub flag_cancel_label: String,
    /// `flagged: “{note}”` as it renders under the source line. The ⚑ is drawn
    /// by the stylesheet: it is decoration, not language.
    pub flag_shown_template: String,
    /// What Start reads when every question in the filter is skipped today.
    pub nothing_left_label: String,

    // ── S0 · the unfinished sitting ──────────────────────────────────────
    /// The bold opening of the blue resume box.
    pub unfinished_label: String,
    /// `· {when} · {who} · {answered} of {total} answered.`
    pub unfinished_detail_template: String,
    /// Re-enters the open sitting at the next undealt question.
    pub resume_label: String,
    /// Closes the open sitting and returns a clean start card.
    pub start_over_label: String,
    /// The sub-line saying the destructive-sounding control is not destructive.
    pub start_over_hint: String,

    // ── S1 / S2 · the top bar ────────────────────────────────────────────
    /// The marked exit from a sitting.
    pub back_label: String,
    /// The grey hint beside Back on the QUESTION screen.
    pub back_hint_question: String,
    /// The grey hint beside Back on the REVEAL screen — a different sentence
    /// because the fact is different: the row was written when she answered.
    pub back_hint_reveal: String,
    /// Mid-sitting skip.
    pub skip_question_label: String,
    /// Closes the sitting and shows Chuck's sheet.
    pub end_session_label: String,
    /// What the answer row stores for a mid-sitting skip. Never an empty string,
    /// so a skipped question and an unanswered one stay different rows.
    pub skipped_answer_text: String,

    // ── S3 · Chuck's sheet ───────────────────────────────────────────────
    /// The third mark, in the muted style.
    pub mark_skipped: String,
    /// `{s} skipped.`, appended to the headline when s > 0.
    pub sheet_skipped_clause_template: String,
    /// Appended when the sitting was ended before its queue was exhausted.
    pub sheet_ended_early_clause: String,
    /// The bold heading of the flag list at the foot of the sheet.
    pub flag_summary_heading: String,
    /// The sentence after that heading.
    pub flag_summary_hint: String,
    /// `{id} — “{question}” → {note}` — one flagged question, as it prints.
    pub flag_summary_item_template: String,
}

pub(crate) const KEY_DECK_HEADING: &str = "practice_deck_heading";
pub(crate) const KEY_DECK_COUNT_TEMPLATE: &str = "practice_deck_count_template";
pub(crate) const KEY_DECK_SKIPPED_SUFFIX_TEMPLATE: &str = "practice_deck_skipped_suffix_template";
pub(crate) const KEY_DECK_HIDE_LINK: &str = "practice_deck_hide_link";
pub(crate) const KEY_DECK_SHOW_LINK: &str = "practice_deck_show_link";
pub(crate) const KEY_DECK_INSTRUCTION_TEMPLATE: &str = "practice_deck_instruction_template";
pub(crate) const KEY_SKIP_TODAY_LABEL: &str = "practice_skip_today_label";
pub(crate) const KEY_SKIPPED_TODAY_LABEL: &str = "practice_skipped_today_label";
pub(crate) const KEY_FLAG_LABEL: &str = "practice_flag_label";
pub(crate) const KEY_FLAG_EDIT_LABEL: &str = "practice_flag_edit_label";
pub(crate) const KEY_FLAG_PLACEHOLDER: &str = "practice_flag_placeholder";
pub(crate) const KEY_FLAG_SAVE_LABEL: &str = "practice_flag_save_label";
pub(crate) const KEY_FLAG_CANCEL_LABEL: &str = "practice_flag_cancel_label";
pub(crate) const KEY_FLAG_SHOWN_TEMPLATE: &str = "practice_flag_shown_template";
pub(crate) const KEY_NOTHING_LEFT_LABEL: &str = "practice_nothing_left_label";
pub(crate) const KEY_UNFINISHED_LABEL: &str = "practice_unfinished_label";
pub(crate) const KEY_UNFINISHED_DETAIL_TEMPLATE: &str = "practice_unfinished_detail_template";
pub(crate) const KEY_RESUME_LABEL: &str = "practice_resume_label";
pub(crate) const KEY_START_OVER_LABEL: &str = "practice_start_over_label";
pub(crate) const KEY_START_OVER_HINT: &str = "practice_start_over_hint";
pub(crate) const KEY_BACK_LABEL: &str = "practice_back_label";
pub(crate) const KEY_BACK_HINT_QUESTION: &str = "practice_back_hint_question";
pub(crate) const KEY_BACK_HINT_REVEAL: &str = "practice_back_hint_reveal";
pub(crate) const KEY_SKIP_QUESTION_LABEL: &str = "practice_skip_question_label";
pub(crate) const KEY_END_SESSION_LABEL: &str = "practice_end_session_label";
pub(crate) const KEY_SKIPPED_ANSWER_TEXT: &str = "practice_skipped_answer_text";
pub(crate) const KEY_MARK_SKIPPED: &str = "practice_mark_skipped";
pub(crate) const KEY_SHEET_SKIPPED_CLAUSE_TEMPLATE: &str = "practice_sheet_skipped_clause_template";
pub(crate) const KEY_SHEET_ENDED_EARLY_CLAUSE: &str = "practice_sheet_ended_early_clause";
pub(crate) const KEY_FLAG_SUMMARY_HEADING: &str = "practice_flag_summary_heading";
pub(crate) const KEY_FLAG_SUMMARY_HINT: &str = "practice_flag_summary_hint";
pub(crate) const KEY_FLAG_SUMMARY_ITEM_TEMPLATE: &str = "practice_flag_summary_item_template";

/// Every key in this block, so a missing one is caught at boot BY NAME rather
/// than as a blank control in front of Marie mid-session.
pub const PRACTICE_FLOW_WORDING_KEYS: &[&str] = &[
    KEY_DECK_HEADING,
    KEY_DECK_COUNT_TEMPLATE,
    KEY_DECK_SKIPPED_SUFFIX_TEMPLATE,
    KEY_DECK_HIDE_LINK,
    KEY_DECK_SHOW_LINK,
    KEY_DECK_INSTRUCTION_TEMPLATE,
    KEY_SKIP_TODAY_LABEL,
    KEY_SKIPPED_TODAY_LABEL,
    KEY_FLAG_LABEL,
    KEY_FLAG_EDIT_LABEL,
    KEY_FLAG_PLACEHOLDER,
    KEY_FLAG_SAVE_LABEL,
    KEY_FLAG_CANCEL_LABEL,
    KEY_FLAG_SHOWN_TEMPLATE,
    KEY_NOTHING_LEFT_LABEL,
    KEY_UNFINISHED_LABEL,
    KEY_UNFINISHED_DETAIL_TEMPLATE,
    KEY_RESUME_LABEL,
    KEY_START_OVER_LABEL,
    KEY_START_OVER_HINT,
    KEY_BACK_LABEL,
    KEY_BACK_HINT_QUESTION,
    KEY_BACK_HINT_REVEAL,
    KEY_SKIP_QUESTION_LABEL,
    KEY_END_SESSION_LABEL,
    KEY_SKIPPED_ANSWER_TEXT,
    KEY_MARK_SKIPPED,
    KEY_SHEET_SKIPPED_CLAUSE_TEMPLATE,
    KEY_SHEET_ENDED_EARLY_CLAUSE,
    KEY_FLAG_SUMMARY_HEADING,
    KEY_FLAG_SUMMARY_HINT,
    KEY_FLAG_SUMMARY_ITEM_TEMPLATE,
];

/// Build a [`PracticeFlowWording`] from the stored rows, or say which key is
/// wrong.
///
/// ## Rust Learning: taking the reader by reference
///
/// The caller is [`super::wording_practice::build_practice_wording`], which owns
/// its own `read` closure and still needs it afterwards. Passing `&read` works
/// because `&F` implements `Fn` whenever `F` does — so one closure serves both
/// blocks without being cloned or rebuilt, and both are judged by exactly the
/// same rule.
///
/// # Errors
/// Returns whatever `read` returns for the first key that is missing, of the
/// wrong declared kind, or blank.
pub fn build_practice_flow_wording<E>(
    read: impl Fn(&str) -> Result<String, E>,
) -> Result<PracticeFlowWording, E> {
    Ok(PracticeFlowWording {
        deck_heading: read(KEY_DECK_HEADING)?,
        deck_count_template: read(KEY_DECK_COUNT_TEMPLATE)?,
        deck_skipped_suffix_template: read(KEY_DECK_SKIPPED_SUFFIX_TEMPLATE)?,
        deck_hide_link: read(KEY_DECK_HIDE_LINK)?,
        deck_show_link: read(KEY_DECK_SHOW_LINK)?,
        deck_instruction_template: read(KEY_DECK_INSTRUCTION_TEMPLATE)?,
        skip_today_label: read(KEY_SKIP_TODAY_LABEL)?,
        skipped_today_label: read(KEY_SKIPPED_TODAY_LABEL)?,
        flag_label: read(KEY_FLAG_LABEL)?,
        flag_edit_label: read(KEY_FLAG_EDIT_LABEL)?,
        flag_placeholder: read(KEY_FLAG_PLACEHOLDER)?,
        flag_save_label: read(KEY_FLAG_SAVE_LABEL)?,
        flag_cancel_label: read(KEY_FLAG_CANCEL_LABEL)?,
        flag_shown_template: read(KEY_FLAG_SHOWN_TEMPLATE)?,
        nothing_left_label: read(KEY_NOTHING_LEFT_LABEL)?,
        unfinished_label: read(KEY_UNFINISHED_LABEL)?,
        unfinished_detail_template: read(KEY_UNFINISHED_DETAIL_TEMPLATE)?,
        resume_label: read(KEY_RESUME_LABEL)?,
        start_over_label: read(KEY_START_OVER_LABEL)?,
        start_over_hint: read(KEY_START_OVER_HINT)?,
        back_label: read(KEY_BACK_LABEL)?,
        back_hint_question: read(KEY_BACK_HINT_QUESTION)?,
        back_hint_reveal: read(KEY_BACK_HINT_REVEAL)?,
        skip_question_label: read(KEY_SKIP_QUESTION_LABEL)?,
        end_session_label: read(KEY_END_SESSION_LABEL)?,
        skipped_answer_text: read(KEY_SKIPPED_ANSWER_TEXT)?,
        mark_skipped: read(KEY_MARK_SKIPPED)?,
        sheet_skipped_clause_template: read(KEY_SHEET_SKIPPED_CLAUSE_TEMPLATE)?,
        sheet_ended_early_clause: read(KEY_SHEET_ENDED_EARLY_CLAUSE)?,
        flag_summary_heading: read(KEY_FLAG_SUMMARY_HEADING)?,
        flag_summary_hint: read(KEY_FLAG_SUMMARY_HINT)?,
        flag_summary_item_template: read(KEY_FLAG_SUMMARY_ITEM_TEMPLATE)?,
    })
}

#[cfg(test)]
#[path = "wording_practice_flow_tests.rs"]
pub(crate) mod tests;
