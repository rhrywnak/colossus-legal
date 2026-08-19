// =============================================================================
// backend/src/domain/wording_practice_review.rs — the words about a PAST answer
// =============================================================================
//
// Part B of CC_TASK_PRACTICE_V1_CHUCK_REVIEW_v1 (mockup v4, §B3–B4): the notes
// Chuck, Marie and Roman write to each other, and the review page that stacks
// every attempt at one question.
//
// ## Why this is its own block and not more of the editor's
//
// Rule 17, and a real seam. The editor's words are about CHANGING the deck; these
// are about what already happened — an answer given, a note written, an attempt
// re-read. Nothing here can alter a thing: the review page is read-only by
// Roman's own ruling (an answer is a moment; she answers again instead), and a
// note is never deleted, only struck.
//
// ## The one arrow, again
//
// `notes_heading_template` carries NO disclosure arrow: the panel draws its own,
// and it turns when the panel opens, which a character in a string cannot do.
// That is the A8 lesson, applied before it could be repeated.
// `review_practice_again` DOES carry a `▸`, and correctly — it is a link to
// another screen, not a disclosure control, and nothing else draws an arrow
// beside it.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PracticeReviewWording {
    pub notes_heading_template: String,
    pub notes_scenario_title: String,
    pub notes_question_title: String,
    pub notes_hint: String,
    pub notes_placeholder: String,
    pub notes_attempt_placeholder: String,
    pub notes_save_label: String,
    pub notes_strike_label: String,
    pub notes_struck_template: String,
    pub notes_empty: String,
    pub notes_failed: String,
    pub notes_author_unset: String,
    pub row_review_link: String,
    pub review_progress_template: String,
    pub review_attempts_kicker: String,
    pub review_attempt_template: String,
    pub review_detail_template: String,
    pub review_boxes_none: String,
    pub review_no_attempts: String,
    pub review_practice_again: String,
    pub review_stronger_heading: String,
}

pub(crate) const KEY_NOTES_HEADING_TEMPLATE: &str = "practice_notes_heading_template";
pub(crate) const KEY_NOTES_SCENARIO_TITLE: &str = "practice_notes_scenario_title";
pub(crate) const KEY_NOTES_QUESTION_TITLE: &str = "practice_notes_question_title";
pub(crate) const KEY_NOTES_HINT: &str = "practice_notes_hint";
pub(crate) const KEY_NOTES_PLACEHOLDER: &str = "practice_notes_placeholder";
pub(crate) const KEY_NOTES_ATTEMPT_PLACEHOLDER: &str = "practice_notes_attempt_placeholder";
pub(crate) const KEY_NOTES_SAVE_LABEL: &str = "practice_notes_save_label";
pub(crate) const KEY_NOTES_STRIKE_LABEL: &str = "practice_notes_strike_label";
pub(crate) const KEY_NOTES_STRUCK_TEMPLATE: &str = "practice_notes_struck_template";
pub(crate) const KEY_NOTES_EMPTY: &str = "practice_notes_empty";
pub(crate) const KEY_NOTES_FAILED: &str = "practice_notes_failed";
pub(crate) const KEY_NOTES_AUTHOR_UNSET: &str = "practice_notes_author_unset";
pub(crate) const KEY_ROW_REVIEW_LINK: &str = "practice_row_review_link";
pub(crate) const KEY_REVIEW_PROGRESS_TEMPLATE: &str = "practice_review_progress_template";
pub(crate) const KEY_REVIEW_ATTEMPTS_KICKER: &str = "practice_review_attempts_kicker";
pub(crate) const KEY_REVIEW_ATTEMPT_TEMPLATE: &str = "practice_review_attempt_template";
pub(crate) const KEY_REVIEW_DETAIL_TEMPLATE: &str = "practice_review_detail_template";
pub(crate) const KEY_REVIEW_BOXES_NONE: &str = "practice_review_boxes_none";
pub(crate) const KEY_REVIEW_NO_ATTEMPTS: &str = "practice_review_no_attempts";
pub(crate) const KEY_REVIEW_PRACTICE_AGAIN: &str = "practice_review_practice_again";
pub(crate) const KEY_REVIEW_STRONGER_HEADING: &str = "practice_review_stronger_heading";

/// Every key in this block, so a missing one is caught at boot BY NAME rather
/// than as a blank control in front of the person editing.
pub const PRACTICE_REVIEW_WORDING_KEYS: &[&str] = &[
    KEY_NOTES_HEADING_TEMPLATE,
    KEY_NOTES_SCENARIO_TITLE,
    KEY_NOTES_QUESTION_TITLE,
    KEY_NOTES_HINT,
    KEY_NOTES_PLACEHOLDER,
    KEY_NOTES_ATTEMPT_PLACEHOLDER,
    KEY_NOTES_SAVE_LABEL,
    KEY_NOTES_STRIKE_LABEL,
    KEY_NOTES_STRUCK_TEMPLATE,
    KEY_NOTES_EMPTY,
    KEY_NOTES_FAILED,
    KEY_NOTES_AUTHOR_UNSET,
    KEY_ROW_REVIEW_LINK,
    KEY_REVIEW_PROGRESS_TEMPLATE,
    KEY_REVIEW_ATTEMPTS_KICKER,
    KEY_REVIEW_ATTEMPT_TEMPLATE,
    KEY_REVIEW_DETAIL_TEMPLATE,
    KEY_REVIEW_BOXES_NONE,
    KEY_REVIEW_NO_ATTEMPTS,
    KEY_REVIEW_PRACTICE_AGAIN,
    KEY_REVIEW_STRONGER_HEADING,
];

/// Build a [`PracticeReviewWording`] from the stored rows, or say which key is wrong.
///
/// # Errors
/// Returns whatever `read` returns for the first key that is missing, of the
/// wrong declared kind, or blank.
pub fn build_practice_review_wording<E>(
    read: impl Fn(&str) -> Result<String, E>,
) -> Result<PracticeReviewWording, E> {
    Ok(PracticeReviewWording {
        notes_heading_template: read(KEY_NOTES_HEADING_TEMPLATE)?,
        notes_scenario_title: read(KEY_NOTES_SCENARIO_TITLE)?,
        notes_question_title: read(KEY_NOTES_QUESTION_TITLE)?,
        notes_hint: read(KEY_NOTES_HINT)?,
        notes_placeholder: read(KEY_NOTES_PLACEHOLDER)?,
        notes_attempt_placeholder: read(KEY_NOTES_ATTEMPT_PLACEHOLDER)?,
        notes_save_label: read(KEY_NOTES_SAVE_LABEL)?,
        notes_strike_label: read(KEY_NOTES_STRIKE_LABEL)?,
        notes_struck_template: read(KEY_NOTES_STRUCK_TEMPLATE)?,
        notes_empty: read(KEY_NOTES_EMPTY)?,
        notes_failed: read(KEY_NOTES_FAILED)?,
        notes_author_unset: read(KEY_NOTES_AUTHOR_UNSET)?,
        row_review_link: read(KEY_ROW_REVIEW_LINK)?,
        review_progress_template: read(KEY_REVIEW_PROGRESS_TEMPLATE)?,
        review_attempts_kicker: read(KEY_REVIEW_ATTEMPTS_KICKER)?,
        review_attempt_template: read(KEY_REVIEW_ATTEMPT_TEMPLATE)?,
        review_detail_template: read(KEY_REVIEW_DETAIL_TEMPLATE)?,
        review_boxes_none: read(KEY_REVIEW_BOXES_NONE)?,
        review_no_attempts: read(KEY_REVIEW_NO_ATTEMPTS)?,
        review_practice_again: read(KEY_REVIEW_PRACTICE_AGAIN)?,
        review_stronger_heading: read(KEY_REVIEW_STRONGER_HEADING)?,
    })
}

#[cfg(test)]
#[path = "wording_practice_review_tests.rs"]
pub(crate) mod tests;
