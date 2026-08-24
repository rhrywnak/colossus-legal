//! Chuck's review sheets — every string on the paper, and the two on screen.
//!
//! The sixth practice wording block, nested under [`super::wording_practice::PracticeWording`]
//! beside `flow`, `row`, `editor` and `review`. Its own file for the reason each
//! of those has one: a block belongs beside the surface that reads it, and
//! `wording_practice` was already at 170 lines before this task.
//!
//! ## ⚑ `practice_print_button` IS A DIFFERENT FEATURE
//!
//! That key exists, reads "Print Chuck's sheet", and belongs to the END-OF-SITTING
//! sheet (`frontend PracticeSheet`). Nothing here touches it, and every key below
//! is namespaced away from it — because an operator re-wording one and hitting the
//! other is a silent change to a surface they were not looking at.
//!
//! ## Domain note: what this paper is FOR, which decides what is on it
//!
//! Chuck develops questions away from his laptop. The whole value of the printout
//! is that a note he writes today still points at the right question next week,
//! after the deck has been re-ordered — which is what the deck key in the blue box
//! is for. No watch-for, no stronger answer, no note, no answer of Marie's and no
//! read appears on it: this is question development, not practice material.

/// Every string the print view speaks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PracticePrintWording {
    /// The control right of the scenario title. Opens the view in a new tab.
    pub questions_label: String,
    /// Why that control is disabled — an empty deck, or one that is entirely
    /// hidden. Standing rule: no control on a practice page is dim and silent.
    pub questions_empty_hint: String,

    // ── Chuck's reading copy: the answers sheet ──────────────────────────
    // Seeded by L2's migration on 2026-08-23 and DECLARED NOWHERE until .408,
    // which is why the practice page rendered blank on .407: the wire mirror is
    // built field-by-field from these blocks, so a key no block declares has no
    // field and never reaches the browser. The row existed the whole time.
    /// The third button in the title row. Chuck prints questions to mark up and
    /// answers to read — two documents for two acts.
    pub answers_label: String,
    /// The answers view's tab title, so two print tabs are tellable apart.
    pub answers_page_title: String,
    /// The line under the header on every answers sheet.
    pub answers_howto: String,
    /// Printed where Marie has not answered. The question still prints — omitting
    /// it would make the two sheets disagree about how many questions there are.
    pub answer_missing: String,

    /// The print view's own button. Hidden in print.
    ///
    /// Domain note: the view does NOT call `window.print()` on load. Chuck opens
    /// the tab, READS the sheets, and then decides — a page that starts printing
    /// before he has looked at it is a page he cannot review, and reviewing is the
    /// entire purpose.
    pub now_label: String,
    /// The way back from the print view. Hidden in print.
    pub back_label: String,
    /// The browser tab's title. `{code}` only — a tab title is read over a
    /// shoulder, so no accusation text goes in it.
    pub page_title: String,

    pub sheet_cross_title: String,
    pub sheet_direct_title: String,
    pub sheet_redirect_title: String,
    /// `{code} · “{title}”` — the line under a sheet's title.
    pub sheet_subtitle_template: String,
    /// Sheet 3's subtitle, replacing the accusation.
    ///
    /// Domain note: it must not claim one redirect per defense question. That is
    /// false whenever the counts differ, and on S-7 they do — six cross, two
    /// redirects.
    pub sheet_redirect_subtitle: String,

    /// `printed {when}`.
    pub printed_template: String,
    /// `deck as of {date} · {n} of {m} questions`.
    ///
    /// `{n}` is THIS SHEET's count and `{m}` is the WHOLE DECK. Both, because a
    /// sheet showing only its own count cannot tell Chuck how much of the deck he
    /// is not holding. Hidden questions are in neither number.
    pub deck_as_of_template: String,

    /// Sheet 1's instruction, carrying `{code}` and the route back into the app.
    pub howto_cross: String,
    pub howto_direct: String,
    /// Sheet 3's instruction. Carries NO count.
    pub howto_redirect: String,
    /// Appended to the above ONLY when at least one redirect on the sheet carries
    /// `draft_by`. Withheld otherwise — a sheet claiming draftness its own rows do
    /// not show would be the paper contradicting itself.
    pub howto_redirect_drafts: String,

    /// `After the defense asks {key}: {question}`.
    pub after_template: String,
    /// When a redirect's `follows_key` names nothing in this scenario.
    ///
    /// `follows_key` is a KEY, not a foreign key, so nothing in the database stops
    /// the question it names being hidden or removed.
    pub after_missing: String,

    /// `{code} · {sheet} · {n} questions` — the foot of each sheet.
    pub footer_template: String,
    /// `sheet {n} of {m}`.
    ///
    /// ## Domain note: SHEETS, not PAGES
    ///
    /// A sheet with enough questions runs onto a second and third piece of paper —
    /// S-7 has eight directs — and "page 2 of 3" printed on both halves of one
    /// sheet is a lie the browser cannot correct. Physical pagination belongs to
    /// the browser; this number belongs to the document.
    pub sheet_number_template: String,

    /// `This deck has` — opens the absent-kinds line.
    pub missing_prefix: String,
    pub missing_cross: String,
    pub missing_direct: String,
    pub missing_redirect: String,
    /// Joins two fragments. Stored WITHOUT a trailing space — the store trims
    /// every value, so the renderer supplies the joining space.
    pub missing_joiner: String,
    /// `{n} questions are hidden and are not shown.`
    pub hidden_template: String,
}

pub(crate) const KEY_QUESTIONS_LABEL: &str = "practice_print_questions_label";
pub(crate) const KEY_QUESTIONS_EMPTY_HINT: &str = "practice_print_questions_empty_hint";
pub(crate) const KEY_ANSWERS_LABEL: &str = "practice_print_answers_label";
pub(crate) const KEY_ANSWERS_PAGE_TITLE: &str = "practice_print_answers_page_title";
pub(crate) const KEY_ANSWERS_HOWTO: &str = "practice_print_answers_howto";
pub(crate) const KEY_ANSWER_MISSING: &str = "practice_print_answer_missing";
pub(crate) const KEY_NOW_LABEL: &str = "practice_print_now_label";
pub(crate) const KEY_BACK_LABEL: &str = "practice_print_back_label";
pub(crate) const KEY_PAGE_TITLE: &str = "practice_print_page_title";
pub(crate) const KEY_SHEET_CROSS_TITLE: &str = "practice_print_sheet_cross_title";
pub(crate) const KEY_SHEET_DIRECT_TITLE: &str = "practice_print_sheet_direct_title";
pub(crate) const KEY_SHEET_REDIRECT_TITLE: &str = "practice_print_sheet_redirect_title";
pub(crate) const KEY_SHEET_SUBTITLE_TEMPLATE: &str = "practice_print_sheet_subtitle_template";
pub(crate) const KEY_SHEET_REDIRECT_SUBTITLE: &str = "practice_print_sheet_redirect_subtitle";
pub(crate) const KEY_PRINTED_TEMPLATE: &str = "practice_print_printed_template";
pub(crate) const KEY_DECK_AS_OF_TEMPLATE: &str = "practice_print_deck_as_of_template";
pub(crate) const KEY_HOWTO_CROSS: &str = "practice_print_howto_cross";
pub(crate) const KEY_HOWTO_DIRECT: &str = "practice_print_howto_direct";
pub(crate) const KEY_HOWTO_REDIRECT: &str = "practice_print_howto_redirect";
pub(crate) const KEY_HOWTO_REDIRECT_DRAFTS: &str = "practice_print_howto_redirect_drafts";
pub(crate) const KEY_AFTER_TEMPLATE: &str = "practice_print_after_template";
pub(crate) const KEY_AFTER_MISSING: &str = "practice_print_after_missing";
pub(crate) const KEY_FOOTER_TEMPLATE: &str = "practice_print_footer_template";
pub(crate) const KEY_SHEET_NUMBER_TEMPLATE: &str = "practice_print_sheet_number_template";
pub(crate) const KEY_MISSING_PREFIX: &str = "practice_print_missing_prefix";
pub(crate) const KEY_MISSING_CROSS: &str = "practice_print_missing_cross";
pub(crate) const KEY_MISSING_DIRECT: &str = "practice_print_missing_direct";
pub(crate) const KEY_MISSING_REDIRECT: &str = "practice_print_missing_redirect";
pub(crate) const KEY_MISSING_JOINER: &str = "practice_print_missing_joiner";
pub(crate) const KEY_HIDDEN_TEMPLATE: &str = "practice_print_hidden_template";

/// Every key in this block, so a missing one is caught at boot BY NAME rather
/// than as a blank heading on a sheet somebody has already taken to a meeting.
pub const PRACTICE_PRINT_WORDING_KEYS: &[&str] = &[
    KEY_QUESTIONS_LABEL,
    KEY_QUESTIONS_EMPTY_HINT,
    KEY_ANSWERS_LABEL,
    KEY_ANSWERS_PAGE_TITLE,
    KEY_ANSWERS_HOWTO,
    KEY_ANSWER_MISSING,
    KEY_NOW_LABEL,
    KEY_BACK_LABEL,
    KEY_PAGE_TITLE,
    KEY_SHEET_CROSS_TITLE,
    KEY_SHEET_DIRECT_TITLE,
    KEY_SHEET_REDIRECT_TITLE,
    KEY_SHEET_SUBTITLE_TEMPLATE,
    KEY_SHEET_REDIRECT_SUBTITLE,
    KEY_PRINTED_TEMPLATE,
    KEY_DECK_AS_OF_TEMPLATE,
    KEY_HOWTO_CROSS,
    KEY_HOWTO_DIRECT,
    KEY_HOWTO_REDIRECT,
    KEY_HOWTO_REDIRECT_DRAFTS,
    KEY_AFTER_TEMPLATE,
    KEY_AFTER_MISSING,
    KEY_FOOTER_TEMPLATE,
    KEY_SHEET_NUMBER_TEMPLATE,
    KEY_MISSING_PREFIX,
    KEY_MISSING_CROSS,
    KEY_MISSING_DIRECT,
    KEY_MISSING_REDIRECT,
    KEY_MISSING_JOINER,
    KEY_HIDDEN_TEMPLATE,
];

/// Build the block, or name the first row that is wrong.
///
/// ## Rust Learning: `impl Fn(&str) -> Result<String, E>` and a generic error
///
/// The reader is passed IN rather than this function reaching for the store, and
/// the error type is generic — so the same builder serves production (where a
/// missing row is a boot refusal) and the test fixture (where it is a `String`).
/// That is what makes `PracticePrintWording::for_test` construct itself through
/// the PRODUCTION path: a fixture the real builder would reject cannot exist.
///
/// # Errors
/// Whatever `read` returns for the first key that is missing, of the wrong
/// declared kind, or blank.
pub fn build_practice_print_wording<E>(
    read: impl Fn(&str) -> Result<String, E>,
) -> Result<PracticePrintWording, E> {
    Ok(PracticePrintWording {
        questions_label: read(KEY_QUESTIONS_LABEL)?,
        questions_empty_hint: read(KEY_QUESTIONS_EMPTY_HINT)?,
        answers_label: read(KEY_ANSWERS_LABEL)?,
        answers_page_title: read(KEY_ANSWERS_PAGE_TITLE)?,
        answers_howto: read(KEY_ANSWERS_HOWTO)?,
        answer_missing: read(KEY_ANSWER_MISSING)?,
        now_label: read(KEY_NOW_LABEL)?,
        back_label: read(KEY_BACK_LABEL)?,
        page_title: read(KEY_PAGE_TITLE)?,
        sheet_cross_title: read(KEY_SHEET_CROSS_TITLE)?,
        sheet_direct_title: read(KEY_SHEET_DIRECT_TITLE)?,
        sheet_redirect_title: read(KEY_SHEET_REDIRECT_TITLE)?,
        sheet_subtitle_template: read(KEY_SHEET_SUBTITLE_TEMPLATE)?,
        sheet_redirect_subtitle: read(KEY_SHEET_REDIRECT_SUBTITLE)?,
        printed_template: read(KEY_PRINTED_TEMPLATE)?,
        deck_as_of_template: read(KEY_DECK_AS_OF_TEMPLATE)?,
        howto_cross: read(KEY_HOWTO_CROSS)?,
        howto_direct: read(KEY_HOWTO_DIRECT)?,
        howto_redirect: read(KEY_HOWTO_REDIRECT)?,
        howto_redirect_drafts: read(KEY_HOWTO_REDIRECT_DRAFTS)?,
        after_template: read(KEY_AFTER_TEMPLATE)?,
        after_missing: read(KEY_AFTER_MISSING)?,
        footer_template: read(KEY_FOOTER_TEMPLATE)?,
        sheet_number_template: read(KEY_SHEET_NUMBER_TEMPLATE)?,
        missing_prefix: read(KEY_MISSING_PREFIX)?,
        missing_cross: read(KEY_MISSING_CROSS)?,
        missing_direct: read(KEY_MISSING_DIRECT)?,
        missing_redirect: read(KEY_MISSING_REDIRECT)?,
        missing_joiner: read(KEY_MISSING_JOINER)?,
        hidden_template: read(KEY_HIDDEN_TEMPLATE)?,
    })
}

#[cfg(test)]
#[path = "wording_practice_print_tests.rs"]
mod tests;
