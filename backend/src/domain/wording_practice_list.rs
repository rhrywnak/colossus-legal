//! The one-page question list — the practice bar, and the footnote under it.
//!
//! The seventh practice wording block, nested under
//! [`super::wording_practice::PracticeWording`] beside `flow`, `row`, `editor`
//! and `print`. Its own file for the reason each of those has one: a block
//! belongs beside the surface that reads it.
//!
//! ## Why a NEW block rather than four more keys in `flow`
//!
//! `wording_practice_flow` is the SITTING's block — Start, Resume, Skip today,
//! the end-of-sitting sheet — and CC_TASK_PRACTICE_ONE_PAGE retires the sitting
//! from the interface. Filing the words of the surface that REPLACES it inside
//! the block it replaces would leave the next reader unable to tell which of the
//! two a given key belongs to, at exactly the moment that distinction decides
//! whether a key is live or dead.
//!
//! ## ⚑ The dropdown's two options are NOT here
//!
//! "The defense asks" and "Chuck asks" are `practice_who_george_title` and
//! `practice_who_chuck_title`, which already hold those exact words for the
//! side cards. The cards go; the strings stay where they are. A second pair
//! would be two places to edit and one of them eventually not edited — and the
//! two would disagree on the one screen that shows the choice.
//!
//! ## Domain note: practice mode writes NOTHING
//!
//! Two of these four strings describe a walk that makes no model call and no
//! database write of any kind. It offers only questions Marie has already
//! answered, because practising an answer she has not written is nothing to
//! practise. If a future change makes that surface write, these words stop being
//! true and are the first thing to fix.

/// Every string the one-page list speaks that no older block already holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PracticeListWording {
    /// The label at the left of the practice bar.
    pub practice_mode_label: String,
    /// The button that begins a practice walk.
    pub start_practising_label: String,
    /// The one line of hint beside it.
    ///
    /// Standing rule of 2026-08-19: no control on a practice page is dim and
    /// silent. A person must be able to tell what a control does before pressing
    /// it — and "Start practising" beside a Start button that used to open a
    /// sitting is exactly the case where a reader needs telling.
    pub practice_hint: String,
    /// The small line under the question list.
    ///
    /// It exists because the row's marks were REMOVED. A reader who remembers
    /// `answered today · repeat · attempt 2` needs to be told once that their
    /// absence is not a fault in the page.
    pub status_footnote: String,

    /// Shown under a critique that is ONE SENTENCE rather than three parts.
    ///
    /// Measured 2026-08-23: 12 of 14 stored answers carry only a composed
    /// sentence, so this is the common rendering and not a rare one. Without
    /// the line, one answer showing three parts and the next showing one
    /// sentence reads as breakage. It points at the fix — pressing Answer on
    /// unchanged text re-runs the read and attaches the parts to the same row.
    pub read_plain_hint: String,

    // ── L3: the question page, the critique, and the practice walk ──
    pub read_working_label: String,
    pub read_usually_quick: String,
    pub read_still_working: String,
    pub read_stop_waiting: String,
    pub read_why_label: String,
    pub read_pointers_label: String,
    pub read_source_missing: String,
    pub read_unreviewed: String,
    pub read_wrong_label: String,
    pub earlier_versions_template: String,
    pub earlier_version_one: String,
    pub your_answer_dated_template: String,
    pub show_answer_label: String,
    pub next_question_label: String,
    pub change_answer_label: String,
    pub practice_counter_template: String,
    pub practice_say_aloud: String,
    pub practice_then_press_template: String,
    pub practice_skip_hint: String,
    pub practice_end_title: String,
    pub practice_end_count_template: String,
    pub practise_again_label: String,
    pub practice_none_answered: String,
    pub deck_question_missing: String,
}

pub(crate) const KEY_PRACTICE_MODE_LABEL: &str = "practice_practice_mode_label";
pub(crate) const KEY_START_PRACTISING_LABEL: &str = "practice_start_practising_label";
pub(crate) const KEY_PRACTICE_HINT: &str = "practice_practice_hint";
pub(crate) const KEY_STATUS_FOOTNOTE: &str = "practice_deck_status_footnote";
pub(crate) const KEY_READ_PLAIN_HINT: &str = "practice_read_plain_hint";
pub(crate) const KEY_READ_WORKING_LABEL: &str = "practice_read_working_label";
pub(crate) const KEY_READ_USUALLY_QUICK: &str = "practice_read_usually_quick";
pub(crate) const KEY_READ_STILL_WORKING: &str = "practice_read_still_working";
pub(crate) const KEY_READ_STOP_WAITING: &str = "practice_read_stop_waiting";
pub(crate) const KEY_READ_WHY_LABEL: &str = "practice_read_why_label";
pub(crate) const KEY_READ_POINTERS_LABEL: &str = "practice_read_pointers_label";
pub(crate) const KEY_READ_SOURCE_MISSING: &str = "practice_read_source_missing";
pub(crate) const KEY_READ_UNREVIEWED: &str = "practice_read_unreviewed";
pub(crate) const KEY_READ_WRONG_LABEL: &str = "practice_read_wrong_label";
pub(crate) const KEY_EARLIER_VERSIONS_TEMPLATE: &str = "practice_earlier_versions_template";
pub(crate) const KEY_EARLIER_VERSION_ONE: &str = "practice_earlier_version_one";
pub(crate) const KEY_YOUR_ANSWER_DATED_TEMPLATE: &str = "practice_your_answer_dated_template";
pub(crate) const KEY_SHOW_ANSWER_LABEL: &str = "practice_show_answer_label";
pub(crate) const KEY_NEXT_QUESTION_LABEL: &str = "practice_next_question_label";
pub(crate) const KEY_CHANGE_ANSWER_LABEL: &str = "practice_change_answer_label";
pub(crate) const KEY_PRACTICE_COUNTER_TEMPLATE: &str = "practice_practice_counter_template";
pub(crate) const KEY_PRACTICE_SAY_ALOUD: &str = "practice_practice_say_aloud";
pub(crate) const KEY_PRACTICE_THEN_PRESS_TEMPLATE: &str = "practice_practice_then_press_template";
pub(crate) const KEY_PRACTICE_SKIP_HINT: &str = "practice_practice_skip_hint";
pub(crate) const KEY_PRACTICE_END_TITLE: &str = "practice_practice_end_title";
pub(crate) const KEY_PRACTICE_END_COUNT_TEMPLATE: &str = "practice_practice_end_count_template";
pub(crate) const KEY_PRACTISE_AGAIN_LABEL: &str = "practice_practise_again_label";
pub(crate) const KEY_PRACTICE_NONE_ANSWERED: &str = "practice_practice_none_answered";
pub(crate) const KEY_DECK_QUESTION_MISSING: &str = "practice_deck_question_missing";

/// Declared to the boot loader. A key here with no row in any migration makes
/// the backend REFUSE TO START — which is what the sibling test file exists to
/// catch before a deploy does.
pub(crate) const PRACTICE_LIST_WORDING_KEYS: &[&str] = &[
    KEY_PRACTICE_MODE_LABEL,
    KEY_START_PRACTISING_LABEL,
    KEY_PRACTICE_HINT,
    KEY_STATUS_FOOTNOTE,
    KEY_READ_PLAIN_HINT,
    KEY_READ_WORKING_LABEL,
    KEY_READ_USUALLY_QUICK,
    KEY_READ_STILL_WORKING,
    KEY_READ_STOP_WAITING,
    KEY_READ_WHY_LABEL,
    KEY_READ_POINTERS_LABEL,
    KEY_READ_SOURCE_MISSING,
    KEY_READ_UNREVIEWED,
    KEY_READ_WRONG_LABEL,
    KEY_EARLIER_VERSIONS_TEMPLATE,
    KEY_EARLIER_VERSION_ONE,
    KEY_YOUR_ANSWER_DATED_TEMPLATE,
    KEY_SHOW_ANSWER_LABEL,
    KEY_NEXT_QUESTION_LABEL,
    KEY_CHANGE_ANSWER_LABEL,
    KEY_PRACTICE_COUNTER_TEMPLATE,
    KEY_PRACTICE_SAY_ALOUD,
    KEY_PRACTICE_THEN_PRESS_TEMPLATE,
    KEY_PRACTICE_SKIP_HINT,
    KEY_PRACTICE_END_TITLE,
    KEY_PRACTICE_END_COUNT_TEMPLATE,
    KEY_PRACTISE_AGAIN_LABEL,
    KEY_PRACTICE_NONE_ANSWERED,
    KEY_DECK_QUESTION_MISSING,
];

/// Build a [`PracticeListWording`] from the stored rows, or say which key is
/// wrong.
///
/// ## Rust Learning: taking the reader by reference
///
/// The caller is [`super::wording_practice::build_practice_wording`], which owns
/// its own `read` closure and still needs it afterwards. `&F` implements `Fn`
/// whenever `F` does, so one closure serves every nested block without being
/// cloned — and every block is judged by exactly the same rule.
///
/// # Errors
/// Returns whatever `read` returns for the first key that is missing, of the
/// wrong declared kind, or blank.
pub fn build_practice_list_wording<E>(
    read: impl Fn(&str) -> Result<String, E>,
) -> Result<PracticeListWording, E> {
    Ok(PracticeListWording {
        practice_mode_label: read(KEY_PRACTICE_MODE_LABEL)?,
        start_practising_label: read(KEY_START_PRACTISING_LABEL)?,
        practice_hint: read(KEY_PRACTICE_HINT)?,
        status_footnote: read(KEY_STATUS_FOOTNOTE)?,
        read_plain_hint: read(KEY_READ_PLAIN_HINT)?,
        read_working_label: read(KEY_READ_WORKING_LABEL)?,
        read_usually_quick: read(KEY_READ_USUALLY_QUICK)?,
        read_still_working: read(KEY_READ_STILL_WORKING)?,
        read_stop_waiting: read(KEY_READ_STOP_WAITING)?,
        read_why_label: read(KEY_READ_WHY_LABEL)?,
        read_pointers_label: read(KEY_READ_POINTERS_LABEL)?,
        read_source_missing: read(KEY_READ_SOURCE_MISSING)?,
        read_unreviewed: read(KEY_READ_UNREVIEWED)?,
        read_wrong_label: read(KEY_READ_WRONG_LABEL)?,
        earlier_versions_template: read(KEY_EARLIER_VERSIONS_TEMPLATE)?,
        earlier_version_one: read(KEY_EARLIER_VERSION_ONE)?,
        your_answer_dated_template: read(KEY_YOUR_ANSWER_DATED_TEMPLATE)?,
        show_answer_label: read(KEY_SHOW_ANSWER_LABEL)?,
        next_question_label: read(KEY_NEXT_QUESTION_LABEL)?,
        change_answer_label: read(KEY_CHANGE_ANSWER_LABEL)?,
        practice_counter_template: read(KEY_PRACTICE_COUNTER_TEMPLATE)?,
        practice_say_aloud: read(KEY_PRACTICE_SAY_ALOUD)?,
        practice_then_press_template: read(KEY_PRACTICE_THEN_PRESS_TEMPLATE)?,
        practice_skip_hint: read(KEY_PRACTICE_SKIP_HINT)?,
        practice_end_title: read(KEY_PRACTICE_END_TITLE)?,
        practice_end_count_template: read(KEY_PRACTICE_END_COUNT_TEMPLATE)?,
        practise_again_label: read(KEY_PRACTISE_AGAIN_LABEL)?,
        practice_none_answered: read(KEY_PRACTICE_NONE_ANSWERED)?,
        deck_question_missing: read(KEY_DECK_QUESTION_MISSING)?,
    })
}

#[cfg(test)]
#[path = "wording_practice_list_tests.rs"]
mod tests;
