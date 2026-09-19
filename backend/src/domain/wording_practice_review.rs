// =============================================================================
// backend/src/domain/wording_practice_review.rs — the Review answers page
// =============================================================================
//
// CC_TASK_REVIEW_PAGE_v1 §1. The eighth practice wording block, nested under
// [`super::wording_practice::PracticeWording`] beside `flow`, `row`, `discuss`,
// `editor`, `print` and `list`.
//
// ## Why an eighth block and not eight more fields on `row`
//
// Rule 17 first, as every sibling's header says: `dto::practice_wording` — the
// wire mirror, one field per stored key — sat at 294 of its 300 lines before
// this task, and eight more fields declared inline would have carried it over.
// The mirror takes this block as ONE flattened field instead, which costs it
// three lines and keeps every sentence on the wire exactly where it was.
//
// But the seam is a real one too. `row` speaks about ONE QUESTION wherever that
// question is shown — the deck row, the question page, the notes panel. These
// strings exist only on the page where Chuck reads a whole deck end to end, and
// they are addressed to HIM: "Add a note on this answer", "Not answered yet".
// The two registers will move independently the first time he asks for one of
// them to be shorter.
//
// ## Domain note: what this page is FOR, which decides what is on it
//
// Marie answers a deck over a week; Chuck reads it in one sitting before trial
// and writes on what he reads. That is the loop's READ half, and it had no
// screen — the only way through a 42-answer deck was to open each question's
// own page in turn. Nothing here judges, scores or hides anything: the page
// shows what she wrote, what has already been said about it, and a box.

/// Every string the Review answers page speaks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PracticeReviewWording {
    /// The page's NAME, in three places: the control in the deck's button row,
    /// the last crumb above the page, and the eyebrow over its title.
    ///
    /// ## Why one row and not three
    ///
    /// They are one name. Three rows would let a reader arrive at "Review
    /// answers" from a button called something else, which is the small
    /// wrongness that makes a person check whether they are on the right page.
    /// The eyebrow is upper-cased by the stylesheet, not by the store — a row
    /// shouting in capitals cannot be reused by the two places that must not.
    pub title: String,

    /// Shown in place of an answer where Marie has not written one.
    ///
    /// Domain note: it says where a note written there will LAND. On every
    /// other card the note lands on the answer; here it lands on the question,
    /// and Marie sees it before she answers. Nothing on screen distinguishes
    /// the two otherwise, so the sentence does.
    pub unanswered: String,

    /// The add-note box's placeholder under an ANSWERED question.
    pub note_placeholder_answer: String,
    /// The add-note box's placeholder under an UNANSWERED one. A second row
    /// rather than one that says "this", because the two boxes write to two
    /// different places and the only warning a reader gets is the wording.
    pub note_placeholder_question: String,

    /// `Answered {when} · {author}` — the line under one answer.
    ///
    /// Filled SERVER-side, like every other date on this surface: the browser
    /// holds no templates and no date format, so how this reads is a Settings
    /// edit. `{when}` is the day in the case's own timezone.
    pub answered_template: String,
    /// Stands in for `{author}` when the sitting carries no name.
    ///
    /// Sittings from before 2026-08-19 do not carry one. Said out loud rather
    /// than left blank (Standing Rule 1): a missing name and a blank name are
    /// different facts, and only one of them is worth asking about.
    pub author_unknown: String,

    /// Shown instead of the page when either of its two reads fails.
    pub load_failed: String,
    /// Shown when the deck loaded and has no questions. Distinct from the
    /// failure above on purpose — an empty deck and a deck that would not load
    /// are different states, and a page that renders both as nothing invites
    /// somebody to act on the wrong one.
    pub empty_deck: String,

    /// Between two reviewers' names wherever `{reviewer}` prints the bench.
    ///
    /// Stored WITHOUT its spaces. The store trims every value, so a row cannot
    /// carry a leading one; the server supplies a space on each side, exactly
    /// as `war_room_summary_list_joiner` is used.
    pub name_joiner: String,
}

pub(crate) const KEY_REVIEW_TITLE: &str = "practice_review_title";
pub(crate) const KEY_REVIEW_UNANSWERED: &str = "practice_review_unanswered";
pub(crate) const KEY_REVIEW_NOTE_PLACEHOLDER_ANSWER: &str =
    "practice_review_note_placeholder_answer";
pub(crate) const KEY_REVIEW_NOTE_PLACEHOLDER_QUESTION: &str =
    "practice_review_note_placeholder_question";
pub(crate) const KEY_REVIEW_ANSWERED_TEMPLATE: &str = "practice_review_answered_template";
pub(crate) const KEY_REVIEW_AUTHOR_UNKNOWN: &str = "practice_review_author_unknown";
pub(crate) const KEY_REVIEW_LOAD_FAILED: &str = "practice_review_load_failed";
pub(crate) const KEY_REVIEW_EMPTY_DECK: &str = "practice_review_empty_deck";
pub(crate) const KEY_REVIEW_NAME_JOINER: &str = "practice_review_name_joiner";

/// Every key in this block, so a missing one is caught at boot BY NAME rather
/// than as a blank placeholder in a box Chuck is about to type into.
pub const PRACTICE_REVIEW_WORDING_KEYS: &[&str] = &[
    KEY_REVIEW_TITLE,
    KEY_REVIEW_UNANSWERED,
    KEY_REVIEW_NOTE_PLACEHOLDER_ANSWER,
    KEY_REVIEW_NOTE_PLACEHOLDER_QUESTION,
    KEY_REVIEW_ANSWERED_TEMPLATE,
    KEY_REVIEW_AUTHOR_UNKNOWN,
    KEY_REVIEW_LOAD_FAILED,
    KEY_REVIEW_EMPTY_DECK,
    KEY_REVIEW_NAME_JOINER,
];

/// Build the block, or name the first row that is wrong.
///
/// ## Rust Learning: `impl Fn(&str) -> Result<String, E>` and a generic error
///
/// The reader is passed IN rather than this function reaching for the store,
/// and the error type is generic — so one builder serves production, where a
/// missing row is a boot refusal, and the test fixture, where it is a `String`.
/// That is what lets [`PracticeReviewWording::for_test`] construct itself
/// through the PRODUCTION path: a fixture the real builder would reject cannot
/// exist, which is the whole point of writing it this way.
///
/// # Errors
/// Whatever `read` returns for the first key that is missing, of the wrong
/// declared kind, or blank.
pub fn build_practice_review_wording<E>(
    read: impl Fn(&str) -> Result<String, E>,
) -> Result<PracticeReviewWording, E> {
    Ok(PracticeReviewWording {
        title: read(KEY_REVIEW_TITLE)?,
        unanswered: read(KEY_REVIEW_UNANSWERED)?,
        note_placeholder_answer: read(KEY_REVIEW_NOTE_PLACEHOLDER_ANSWER)?,
        note_placeholder_question: read(KEY_REVIEW_NOTE_PLACEHOLDER_QUESTION)?,
        answered_template: read(KEY_REVIEW_ANSWERED_TEMPLATE)?,
        author_unknown: read(KEY_REVIEW_AUTHOR_UNKNOWN)?,
        load_failed: read(KEY_REVIEW_LOAD_FAILED)?,
        empty_deck: read(KEY_REVIEW_EMPTY_DECK)?,
        name_joiner: read(KEY_REVIEW_NAME_JOINER)?,
    })
}

#[cfg(test)]
#[path = "wording_practice_review_tests.rs"]
pub(crate) mod tests;
