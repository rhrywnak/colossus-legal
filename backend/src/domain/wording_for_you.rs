// =============================================================================
// backend/src/domain/wording_for_you.rs — the words on "For you"
// =============================================================================
//
// CC_TASK_FOR_YOU_v1 layer L1, from the ratified mockup
// `FOR_YOU_MOCKUP_v1_2026-09-22.html` (boards 1, 2, 3 and 5).
//
// ## Why its own block, and not more fields on a practice block
//
// Rule 17, first: `dto::practice_wording` is at 299 of 300 lines, so these
// strings could not ride the practice mirror even if they belonged there.
//
// But the seam is real. Every practice block speaks about a DECK — a sitting, a
// question, a reveal, a sheet. These speak about ONE PERSON'S inbox across every
// deck in the case: the same page serves the witness and the reviewers, and what
// changes between them is which side's items it lists, not which words it knows.
//
// ## Every sentence a row speaks is composed SERVER-side
//
// The browser holds no templates (the law `wording_practice_row` states for the
// deck row, applied here): a row arrives with its deck line, its body and its
// byline already written, and the page renders them. That is what makes "which
// sentence names which kind of item" a Settings edit rather than a deployment.
//
// ## Domain note: the names on this page are stored, not compiled
//
// `{who}` on a row is a person's display name. A reviewer's comes from
// `practice_reviewer_display_names`, index-aligned with the login list; the
// witness's is [`ForYouWording::witness_name`] below, because no other settings
// row carries it and the War Room's own "for Marie" is likewise a stored string.
// No login ever reaches the screen: `cpenzien left a note` is a database
// identifier shown to a lawyer.

/// The words the "For you" page speaks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForYouWording {
    // ── The page ─────────────────────────────────────────────────────────
    /// The page's own title, and the heading above the list.
    pub title: String,
    /// The line under the title when the WITNESS is reading: `{who}` is the
    /// reviewer bench, joined as every other surface joins it.
    pub subtitle_witness: String,
    /// The same line when a REVIEWER is reading; `{who}` is the witness.
    pub subtitle_reviewer: String,
    /// What the page says to somebody who is neither. Not an error and not an
    /// empty page with no explanation: an honest sentence naming whose page
    /// this is.
    ///
    /// ## Domain note: there is deliberately no "the list failed to load" row
    ///
    /// The words arrive INSIDE the payload, so a read that fails carries no
    /// sentence to say so with. The page states the technical cause instead and
    /// logs it — the carve-out `PracticeReviewPage` records for the same
    /// situation. A stored row for that case would be a row nothing could ever
    /// reach.
    pub not_your_list: String,

    // ── The two tabs ─────────────────────────────────────────────────────
    /// `Unread · {count}` — what is waiting.
    pub tab_unread_template: String,
    /// `Everything · {count}` — the same list with the read rows still in it,
    /// so a note already opened can be found again.
    pub tab_everything_template: String,

    // ── The day headings ─────────────────────────────────────────────────
    //
    // Domain note: WHICH day a row belongs to is decided on the server, in the
    // case's own timezone (`practice_case_timezone`). The browser is handed the
    // answer and prints one of these three; it never compares dates, because a
    // laptop in another timezone would group them differently from the deck
    // they came out of.
    pub group_today: String,
    pub group_yesterday: String,
    pub group_earlier: String,

    // ── One row ──────────────────────────────────────────────────────────
    /// `{code} · {deck} — “{question}”` — a row's first line.
    pub deck_line_template: String,
    /// The same line for a note left on a whole scenario rather than on one
    /// question: there is no question to quote.
    pub deck_line_no_question_template: String,
    /// `Answered: “{text}”` — the body of a row about a new answer.
    pub body_answer_template: String,
    /// `Now reads: “{text}”` — the body of a row about a reworded question.
    pub body_change_template: String,

    // ── The DECK row (L3) ────────────────────────────────────────────────
    /// A deck holding at least `practice_for_you_deck_threshold` unread items
    /// is ONE row, and this is what it says: `{count}` is how many wait on it.
    /// Its first line is [`Self::deck_line_no_question_template`] — the deck
    /// without a question, because a deck row stands for several.
    pub deck_body_template: String,
    /// Its singular. Unreachable while the threshold is above 1, and reachable
    /// the moment Roman sets the threshold to 1 in Settings — which is exactly
    /// when "1 answers waiting" would be on screen.
    pub deck_body_one: String,
    /// The deck row's byline: `{when}` is the OLDEST item waiting on it, which
    /// is the fact that says how long the deck has been sitting there.
    pub deck_byline_template: String,

    // ── A reply (L3) ─────────────────────────────────────────────────────
    /// A note written in answer to another note, as a ROW reads it: `{parent}`
    /// is what was written first, `{text}` the reply. One line, both halves —
    /// the exchange is drawn in two lines on the question page, where there is
    /// room for it, and quoted in one here, where there is not.
    pub body_reply_template: String,
    /// `{who} · {what}` — the small line under a row. `{what}` is one of the
    /// five clauses below, which is what makes each row say what KIND of thing
    /// it is (ruling Q1).
    pub byline_template: String,
    /// `{what}` for a note on the witness's own answer, as SHE reads it.
    pub byline_note_on_answer_witness: String,
    /// The same note as a REVIEWER reads it — about her answer, not his.
    pub byline_note_on_answer_reviewer: String,
    /// `{what}` for a note left on the question itself.
    pub byline_note_on_question: String,
    /// `{what}` for a new answer.
    pub byline_answer: String,
    /// `{what}` for a reworded or edited question.
    pub byline_change: String,
    /// Appended on the Everything tab to a row this person has already read.
    /// Never shown on the unread tab, where by construction there is no moment
    /// to name.
    pub byline_read_suffix_template: String,

    // ── Nothing waiting ──────────────────────────────────────────────────
    /// The empty state's first line.
    pub empty_title: String,
    /// Its second: `{who}` is the other side, so the page says where the next
    /// row will come from rather than only that there is none.
    pub empty_hint_template: String,
    /// Its third — `{when}` is the day of the most recent item on this side,
    /// read or not. Withheld entirely when there has never been one, rather
    /// than rendered with an empty date.
    pub empty_last_template: String,

    // ── The one name no other row carries ────────────────────────────────
    /// What this page calls the witness. See the module header: the reviewers'
    /// names are a settings row already, and hers was not.
    pub witness_name: String,
    /// The name a row shows for an item written before this application
    /// recorded who wrote it. Such rows wait for nobody (the L0 migration marks
    /// them read for everyone), but they still appear on the Everything tab,
    /// and a blank where a name goes reads as a page that failed to load.
    pub unknown_author: String,
}

pub(crate) const KEY_TITLE: &str = "for_you_title";
pub(crate) const KEY_SUBTITLE_WITNESS: &str = "for_you_subtitle_witness";
pub(crate) const KEY_SUBTITLE_REVIEWER: &str = "for_you_subtitle_reviewer";
pub(crate) const KEY_NOT_YOUR_LIST: &str = "for_you_not_your_list";
pub(crate) const KEY_TAB_UNREAD_TEMPLATE: &str = "for_you_tab_unread_template";
pub(crate) const KEY_TAB_EVERYTHING_TEMPLATE: &str = "for_you_tab_everything_template";
pub(crate) const KEY_GROUP_TODAY: &str = "for_you_group_today";
pub(crate) const KEY_GROUP_YESTERDAY: &str = "for_you_group_yesterday";
pub(crate) const KEY_GROUP_EARLIER: &str = "for_you_group_earlier";
pub(crate) const KEY_DECK_LINE_TEMPLATE: &str = "for_you_deck_line_template";
pub(crate) const KEY_DECK_LINE_NO_QUESTION_TEMPLATE: &str =
    "for_you_deck_line_no_question_template";
pub(crate) const KEY_BODY_ANSWER_TEMPLATE: &str = "for_you_body_answer_template";
pub(crate) const KEY_BODY_CHANGE_TEMPLATE: &str = "for_you_body_change_template";
pub(crate) const KEY_DECK_BODY_TEMPLATE: &str = "for_you_deck_body_template";
pub(crate) const KEY_DECK_BODY_ONE: &str = "for_you_deck_body_one";
pub(crate) const KEY_DECK_BYLINE_TEMPLATE: &str = "for_you_deck_byline_template";
pub(crate) const KEY_BODY_REPLY_TEMPLATE: &str = "for_you_body_reply_template";
pub(crate) const KEY_BYLINE_TEMPLATE: &str = "for_you_byline_template";
pub(crate) const KEY_BYLINE_NOTE_ON_ANSWER_WITNESS: &str = "for_you_byline_note_on_answer_witness";
pub(crate) const KEY_BYLINE_NOTE_ON_ANSWER_REVIEWER: &str =
    "for_you_byline_note_on_answer_reviewer";
pub(crate) const KEY_BYLINE_NOTE_ON_QUESTION: &str = "for_you_byline_note_on_question";
pub(crate) const KEY_BYLINE_ANSWER: &str = "for_you_byline_answer";
pub(crate) const KEY_BYLINE_CHANGE: &str = "for_you_byline_change";
pub(crate) const KEY_BYLINE_READ_SUFFIX_TEMPLATE: &str = "for_you_byline_read_suffix_template";
pub(crate) const KEY_EMPTY_TITLE: &str = "for_you_empty_title";
pub(crate) const KEY_EMPTY_HINT_TEMPLATE: &str = "for_you_empty_hint_template";
pub(crate) const KEY_EMPTY_LAST_TEMPLATE: &str = "for_you_empty_last_template";
pub(crate) const KEY_WITNESS_NAME: &str = "for_you_witness_name";
pub(crate) const KEY_UNKNOWN_AUTHOR: &str = "for_you_unknown_author";

/// Every key in this block, so a missing one is caught at boot BY NAME rather
/// than as a blank page in front of whoever opened it.
pub const FOR_YOU_WORDING_KEYS: &[&str] = &[
    KEY_TITLE,
    KEY_SUBTITLE_WITNESS,
    KEY_SUBTITLE_REVIEWER,
    KEY_NOT_YOUR_LIST,
    KEY_TAB_UNREAD_TEMPLATE,
    KEY_TAB_EVERYTHING_TEMPLATE,
    KEY_GROUP_TODAY,
    KEY_GROUP_YESTERDAY,
    KEY_GROUP_EARLIER,
    KEY_DECK_LINE_TEMPLATE,
    KEY_DECK_LINE_NO_QUESTION_TEMPLATE,
    KEY_BODY_ANSWER_TEMPLATE,
    KEY_BODY_CHANGE_TEMPLATE,
    KEY_DECK_BODY_TEMPLATE,
    KEY_DECK_BODY_ONE,
    KEY_DECK_BYLINE_TEMPLATE,
    KEY_BODY_REPLY_TEMPLATE,
    KEY_BYLINE_TEMPLATE,
    KEY_BYLINE_NOTE_ON_ANSWER_WITNESS,
    KEY_BYLINE_NOTE_ON_ANSWER_REVIEWER,
    KEY_BYLINE_NOTE_ON_QUESTION,
    KEY_BYLINE_ANSWER,
    KEY_BYLINE_CHANGE,
    KEY_BYLINE_READ_SUFFIX_TEMPLATE,
    KEY_EMPTY_TITLE,
    KEY_EMPTY_HINT_TEMPLATE,
    KEY_EMPTY_LAST_TEMPLATE,
    KEY_WITNESS_NAME,
    KEY_UNKNOWN_AUTHOR,
];

/// Build a [`ForYouWording`] from the stored rows, or say which key is wrong.
///
/// ## Rust Learning: `impl Fn(&str) -> Result<String, E>`
///
/// The caller decides what reading a key MEANS — at boot it is "find the row,
/// check it declares text, refuse a blank one"; in a test it is a lookup in a
/// map. This function knows only the names and the order, so there is exactly
/// one list of keys in the build and every block is judged by the same rule.
///
/// # Errors
/// Whatever `read` returns for the first key that is missing, of the wrong
/// declared kind, or blank.
pub fn build_for_you_wording<E>(
    read: impl Fn(&str) -> Result<String, E>,
) -> Result<ForYouWording, E> {
    Ok(ForYouWording {
        title: read(KEY_TITLE)?,
        subtitle_witness: read(KEY_SUBTITLE_WITNESS)?,
        subtitle_reviewer: read(KEY_SUBTITLE_REVIEWER)?,
        not_your_list: read(KEY_NOT_YOUR_LIST)?,
        tab_unread_template: read(KEY_TAB_UNREAD_TEMPLATE)?,
        tab_everything_template: read(KEY_TAB_EVERYTHING_TEMPLATE)?,
        group_today: read(KEY_GROUP_TODAY)?,
        group_yesterday: read(KEY_GROUP_YESTERDAY)?,
        group_earlier: read(KEY_GROUP_EARLIER)?,
        deck_line_template: read(KEY_DECK_LINE_TEMPLATE)?,
        deck_line_no_question_template: read(KEY_DECK_LINE_NO_QUESTION_TEMPLATE)?,
        body_answer_template: read(KEY_BODY_ANSWER_TEMPLATE)?,
        body_change_template: read(KEY_BODY_CHANGE_TEMPLATE)?,
        deck_body_template: read(KEY_DECK_BODY_TEMPLATE)?,
        deck_body_one: read(KEY_DECK_BODY_ONE)?,
        deck_byline_template: read(KEY_DECK_BYLINE_TEMPLATE)?,
        body_reply_template: read(KEY_BODY_REPLY_TEMPLATE)?,
        byline_template: read(KEY_BYLINE_TEMPLATE)?,
        byline_note_on_answer_witness: read(KEY_BYLINE_NOTE_ON_ANSWER_WITNESS)?,
        byline_note_on_answer_reviewer: read(KEY_BYLINE_NOTE_ON_ANSWER_REVIEWER)?,
        byline_note_on_question: read(KEY_BYLINE_NOTE_ON_QUESTION)?,
        byline_answer: read(KEY_BYLINE_ANSWER)?,
        byline_change: read(KEY_BYLINE_CHANGE)?,
        byline_read_suffix_template: read(KEY_BYLINE_READ_SUFFIX_TEMPLATE)?,
        empty_title: read(KEY_EMPTY_TITLE)?,
        empty_hint_template: read(KEY_EMPTY_HINT_TEMPLATE)?,
        empty_last_template: read(KEY_EMPTY_LAST_TEMPLATE)?,
        witness_name: read(KEY_WITNESS_NAME)?,
        unknown_author: read(KEY_UNKNOWN_AUTHOR)?,
    })
}

#[cfg(test)]
#[path = "wording_for_you_tests.rs"]
pub(crate) mod tests;
