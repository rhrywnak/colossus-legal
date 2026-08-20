// =============================================================================
// backend/src/domain/wording_practice.rs — the words Marie ANSWERS into
// =============================================================================
//
// Screens S0 and S1 of PRACTICE_MOCKUP_v2 (CC_TASK_PRACTICE_SESSION_V0_v1,
// 2026-08-17): the start card she chooses a side on, the question she reads, and
// the named gaps and failures the page speaks when there is nothing to show.
//
// ## Why they are rows and not literals
//
// Task §8 states the rule for this build, and it is the same rule the scan, the
// rehearsal page and the accusation section already answer to (v2 §2b): every
// string that is not DATA is configuration. What is different here is WHO reads
// them. These sentences are read by a witness the night before she testifies,
// and Roman will want to change their tone after watching Marie use it — which
// is a Settings edit and a restart, not a build.
//
// ## Why this block ends where it does
//
// `wording_practice_report` holds what she is SHOWN AFTERWARDS — the reveal, and
// Chuck's sheet. Two modules rather than one because Rule 17 measures code lines
// and seventy-nine fields plus their builder exceeds it — but the seam is not
// arbitrary. The drill's language is addressed to Marie in the moment ("say it
// out loud, then type it"); the report's is addressed to her afterwards and to
// Chuck on paper, and those two registers will move independently the first time
// he asks for a column renamed.
//
// ## The pause note arrives in two rows
//
// The mockup italicises one word ("what was the *question*?"). A row carrying
// `<i>` would put markup in the store; a row carrying the whole sentence would
// lose the emphasis. So it arrives as prefix and emphasis and the component
// supplies the tag. The migration's header argues this at length.
//
// ## What is NOT here
//
// `practice_read_prompt_file`, `practice_read_model`, `practice_read_max_tokens`
// and `practice_tactic_names` are seeded by the same migration and live on
// `Settings` directly. The first three are parameters — nobody reads them on a
// screen. The fourth is a VOCABULARY: a comma-separated list read by the same
// rule `theme_scan_prefilter_statement_types` is.

/// The words the start card and the question screen speak.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PracticeWording {
    /// What mockup v3 added: the deck listed on the start card, the resume line,
    /// the top bar, and the sheet's two new clauses. Nested rather than hung off
    /// `Settings` because that struct sits one line under Rule 17's limit and
    /// this task has no business splitting it — see the flow module's header.
    pub flow: super::wording_practice_flow::PracticeFlowWording,
    /// What v1 (the Chuck review) added: the words about ONE question — the way
    /// into it alone, what happened to it last time, what kind it is, and what
    /// she would point to. Nested for the same reason `flow` is, and the block's
    /// own header argues the seam.
    pub row: super::wording_practice_row::PracticeRowWording,
    /// What Part B added, and the one block addressed to CHUCK rather than to
    /// Marie: the deck editor, the record it writes, and the box that tells her
    /// what changed. Nested for the same Rule 17 reason as its two siblings.
    pub editor: super::wording_practice_editor::PracticeEditorWording,
    /// The words about a PAST answer — the notes panel and the review page.
    pub review: super::wording_practice_review::PracticeReviewWording,
    // ── S0 · the start card ──────────────────────────────────────────────
    /// The eyebrow over the scenario title on the practice start screen.
    pub kicker: String,
    /// The one paragraph under the title on the start screen. It sets the terms
    /// of the session — no clock, nobody watching — which is the whole
    /// difference between a drill and a test.
    pub intro: String,
    /// Heading over the three side choices.
    pub who_heading: String,
    /// The cross-examination choice.
    pub who_george_title: String,
    /// What the cross choice contains.
    pub who_george_detail: String,
    /// The direct-examination choice.
    pub who_chuck_title: String,
    /// What the direct choice contains.
    pub who_chuck_detail: String,
    /// The both-sides choice.
    pub who_mixed_title: String,
    /// What the mixed choice contains.
    pub who_mixed_detail: String,
    /// The small grey term under each side card — `cross`, `direct`.
    ///
    /// Separate from the title because the two are different registers: the
    /// title is the sentence Marie reads, the term is the word Chuck reads, and
    /// they are set in different sizes and colours. One string carrying both
    /// could not be styled as two things.
    pub who_george_term: String,
    pub who_chuck_term: String,
    pub who_redirect_term: String,
    /// Breaks Chuck's direct questions from his redirects in the deck list.
    pub redirects_subheader: String,
    /// Heading over the count pills. Only 5 is live in v0; the others render
    /// dimmed exactly as the mockup does, which is honest about what this build
    /// offers.
    pub how_many_heading: String,
    /// The third count pill. {n} is the deck's own size, filled in the browser
    /// from the questions it was served. The mockup wrote "all 12" against a
    /// twelve-question deck; S-5's is ten, and a pill naming a number no deck
    /// has is the kind of small wrongness a witness stops trusting a screen
    /// over.
    pub count_all_template: String,
    /// The control that opens a session.
    pub start_label: String,
    /// The bold word opening the standing card.
    pub always_label: String,
    /// The five rules that never move. Domain note: this card is also an INPUT
    /// to the one-sentence read — the model is told to judge against it — so
    /// editing this row changes what the system says, not only what the screen
    /// shows.
    pub always_line: String,
    /// The line beside Start, composed from the LOG's most recently ENDED
    /// session for this scenario. {when}, {count} and {repeat} are filled
    /// server-side.
    pub last_session_template: String,
    /// What stands where the last-session line goes when the log holds no ended
    /// session for this scenario. A named absence, never a blank.
    pub no_last_session: String,

    // ── S1 · the question ────────────────────────────────────────────────
    /// The progress line on the question and reveal screens.
    pub progress_template: String,
    /// The pill on a cross question.
    pub pill_george: String,
    /// The pill on a direct question.
    pub pill_chuck: String,
    /// The pill on a compound question that braids several barrage rows.
    pub pill_braid: String,
    /// The bold half of the answer prompt.
    pub answer_label: String,
    /// The rest of the answer prompt. Say-then-type is the whole method (design
    /// §1); speech-to-text is deliberately not in this build.
    pub answer_hint: String,
    /// The textarea placeholder.
    pub answer_placeholder: String,
    /// Submits the typed answer.
    pub answer_button: String,
    /// The control that answers without typing. Domain note: it is a control
    /// and not a hint because "I don't recall" being a COMPLETE answer is the
    /// single hardest thing for a witness to believe.
    pub dont_recall_button: String,
    /// What the control types into the box and stores as her answer — without
    /// the quotation marks the button wears.
    pub dont_recall_text: String,
    /// Shows the pause note.
    pub pause_button: String,
    /// The pause note up to its emphasised word. Split so the store carries no
    /// markup — see this migration's header.
    pub pause_note_prefix: String,
    /// The italicised close of the pause note.
    pub pause_note_emphasis: String,

    // ── The gaps, the failures, and the way in ───────────────────────────
    /// What the practice page says for a scenario whose deck is empty. Domain
    /// note: this is what S-6 shows today, and it must not read as a failure —
    /// it is an accurate statement about a deck nobody has seeded.
    pub empty_deck: String,
    /// What the page shows when the deck request fails. Distinct from an empty
    /// deck, which is not a failure at all.
    pub load_failed: String,
    /// What the question screen shows when the answer POST fails. It says the
    /// write did not happen, because a witness who believes an answer was
    /// logged when it was not is the worst outcome this screen has.
    pub answer_failed: String,
    /// Appended to the tactic tag on a question that braids several barrage
    /// rows, so the tag reads "compound · braid". A row of its own because the
    /// WORD is the reader's only clue that this question is answered by naming
    /// strands rather than by answering it.
    pub tactic_braid_suffix: String,
}

// KEYS: the stable identifiers of the rows above. Renaming one is a migration,
// and until it runs the boot loader refuses to start — which is the point: a
// witness surface with a blank where a sentence should be is not a degraded
// screen, it is a screen nobody can trust.
pub(crate) const KEY_KICKER: &str = "practice_kicker";
pub(crate) const KEY_INTRO: &str = "practice_intro";
pub(crate) const KEY_WHO_HEADING: &str = "practice_who_heading";
pub(crate) const KEY_WHO_GEORGE_TITLE: &str = "practice_who_george_title";
pub(crate) const KEY_WHO_GEORGE_DETAIL: &str = "practice_who_george_detail";
pub(crate) const KEY_WHO_CHUCK_TITLE: &str = "practice_who_chuck_title";
pub(crate) const KEY_WHO_CHUCK_DETAIL: &str = "practice_who_chuck_detail";
pub(crate) const KEY_WHO_MIXED_TITLE: &str = "practice_who_mixed_title";
pub(crate) const KEY_WHO_MIXED_DETAIL: &str = "practice_who_mixed_detail";
pub(crate) const KEY_WHO_GEORGE_TERM: &str = "practice_who_george_term";
pub(crate) const KEY_WHO_CHUCK_TERM: &str = "practice_who_chuck_term";
pub(crate) const KEY_WHO_REDIRECT_TERM: &str = "practice_who_redirect_term";
pub(crate) const KEY_REDIRECTS_SUBHEADER: &str = "practice_redirects_subheader";
pub(crate) const KEY_HOW_MANY_HEADING: &str = "practice_how_many_heading";
pub(crate) const KEY_COUNT_ALL_TEMPLATE: &str = "practice_count_all_template";
pub(crate) const KEY_START_LABEL: &str = "practice_start_label";
pub(crate) const KEY_ALWAYS_LABEL: &str = "practice_always_label";
pub(crate) const KEY_ALWAYS_LINE: &str = "practice_always_line";
pub(crate) const KEY_LAST_SESSION_TEMPLATE: &str = "practice_last_session_template";
pub(crate) const KEY_NO_LAST_SESSION: &str = "practice_no_last_session";
pub(crate) const KEY_PROGRESS_TEMPLATE: &str = "practice_progress_template";
pub(crate) const KEY_PILL_GEORGE: &str = "practice_pill_george";
pub(crate) const KEY_PILL_CHUCK: &str = "practice_pill_chuck";
pub(crate) const KEY_PILL_BRAID: &str = "practice_pill_braid";
pub(crate) const KEY_ANSWER_LABEL: &str = "practice_answer_label";
pub(crate) const KEY_ANSWER_HINT: &str = "practice_answer_hint";
pub(crate) const KEY_ANSWER_PLACEHOLDER: &str = "practice_answer_placeholder";
pub(crate) const KEY_ANSWER_BUTTON: &str = "practice_answer_button";
pub(crate) const KEY_DONT_RECALL_BUTTON: &str = "practice_dont_recall_button";
pub(crate) const KEY_DONT_RECALL_TEXT: &str = "practice_dont_recall_text";
pub(crate) const KEY_PAUSE_BUTTON: &str = "practice_pause_button";
pub(crate) const KEY_PAUSE_NOTE_PREFIX: &str = "practice_pause_note_prefix";
pub(crate) const KEY_PAUSE_NOTE_EMPHASIS: &str = "practice_pause_note_emphasis";
pub(crate) const KEY_EMPTY_DECK: &str = "practice_empty_deck";
pub(crate) const KEY_LOAD_FAILED: &str = "practice_load_failed";
pub(crate) const KEY_ANSWER_FAILED: &str = "practice_answer_failed";
pub(crate) const KEY_TACTIC_BRAID_SUFFIX: &str = "practice_tactic_braid_suffix";

/// Every key in this block, so a missing one is caught at boot BY NAME rather
/// than as a blank control in front of Marie mid-session.
pub const PRACTICE_WORDING_KEYS: &[&str] = &[
    KEY_KICKER,
    KEY_INTRO,
    KEY_WHO_HEADING,
    KEY_WHO_GEORGE_TITLE,
    KEY_WHO_GEORGE_DETAIL,
    KEY_WHO_CHUCK_TITLE,
    KEY_WHO_CHUCK_DETAIL,
    KEY_WHO_MIXED_TITLE,
    KEY_WHO_MIXED_DETAIL,
    KEY_WHO_GEORGE_TERM,
    KEY_WHO_CHUCK_TERM,
    KEY_WHO_REDIRECT_TERM,
    KEY_REDIRECTS_SUBHEADER,
    KEY_HOW_MANY_HEADING,
    KEY_COUNT_ALL_TEMPLATE,
    KEY_START_LABEL,
    KEY_ALWAYS_LABEL,
    KEY_ALWAYS_LINE,
    KEY_LAST_SESSION_TEMPLATE,
    KEY_NO_LAST_SESSION,
    KEY_PROGRESS_TEMPLATE,
    KEY_PILL_GEORGE,
    KEY_PILL_CHUCK,
    KEY_PILL_BRAID,
    KEY_ANSWER_LABEL,
    KEY_ANSWER_HINT,
    KEY_ANSWER_PLACEHOLDER,
    KEY_ANSWER_BUTTON,
    KEY_DONT_RECALL_BUTTON,
    KEY_DONT_RECALL_TEXT,
    KEY_PAUSE_BUTTON,
    KEY_PAUSE_NOTE_PREFIX,
    KEY_PAUSE_NOTE_EMPHASIS,
    KEY_EMPTY_DECK,
    KEY_LOAD_FAILED,
    KEY_ANSWER_FAILED,
    KEY_TACTIC_BRAID_SUFFIX,
];

/// Build a [`PracticeWording`] from the stored rows, or say which key is wrong.
///
/// ## Rust Learning: a closure parameter generic over its error type
///
/// `read` is `impl Fn(&str) -> Result<String, E>` rather than a database handle.
/// That is what lets this module stay pure — it never touches Postgres — while
/// the SAME function is used twice with two different `E`: at boot the closure
/// reads the settings snapshot and returns `SettingError`, and in tests it reads
/// a fixture table and returns `String`. Neither caller can build a value the
/// other could not, because there is only one builder.
///
/// # Errors
/// Returns whatever `read` returns for the first key that is missing, of the
/// wrong declared kind, or blank.
pub fn build_practice_wording<E>(
    read: impl Fn(&str) -> Result<String, E>,
) -> Result<PracticeWording, E> {
    Ok(PracticeWording {
        flow: super::wording_practice_flow::build_practice_flow_wording(&read)?,
        row: super::wording_practice_row::build_practice_row_wording(&read)?,
        editor: super::wording_practice_editor::build_practice_editor_wording(&read)?,
        review: super::wording_practice_review::build_practice_review_wording(&read)?,
        kicker: read(KEY_KICKER)?,
        intro: read(KEY_INTRO)?,
        who_heading: read(KEY_WHO_HEADING)?,
        who_george_title: read(KEY_WHO_GEORGE_TITLE)?,
        who_george_detail: read(KEY_WHO_GEORGE_DETAIL)?,
        who_chuck_title: read(KEY_WHO_CHUCK_TITLE)?,
        who_chuck_detail: read(KEY_WHO_CHUCK_DETAIL)?,
        who_mixed_title: read(KEY_WHO_MIXED_TITLE)?,
        who_mixed_detail: read(KEY_WHO_MIXED_DETAIL)?,
        who_george_term: read(KEY_WHO_GEORGE_TERM)?,
        who_chuck_term: read(KEY_WHO_CHUCK_TERM)?,
        who_redirect_term: read(KEY_WHO_REDIRECT_TERM)?,
        redirects_subheader: read(KEY_REDIRECTS_SUBHEADER)?,
        how_many_heading: read(KEY_HOW_MANY_HEADING)?,
        count_all_template: read(KEY_COUNT_ALL_TEMPLATE)?,
        start_label: read(KEY_START_LABEL)?,
        always_label: read(KEY_ALWAYS_LABEL)?,
        always_line: read(KEY_ALWAYS_LINE)?,
        last_session_template: read(KEY_LAST_SESSION_TEMPLATE)?,
        no_last_session: read(KEY_NO_LAST_SESSION)?,
        progress_template: read(KEY_PROGRESS_TEMPLATE)?,
        pill_george: read(KEY_PILL_GEORGE)?,
        pill_chuck: read(KEY_PILL_CHUCK)?,
        pill_braid: read(KEY_PILL_BRAID)?,
        answer_label: read(KEY_ANSWER_LABEL)?,
        answer_hint: read(KEY_ANSWER_HINT)?,
        answer_placeholder: read(KEY_ANSWER_PLACEHOLDER)?,
        answer_button: read(KEY_ANSWER_BUTTON)?,
        dont_recall_button: read(KEY_DONT_RECALL_BUTTON)?,
        dont_recall_text: read(KEY_DONT_RECALL_TEXT)?,
        pause_button: read(KEY_PAUSE_BUTTON)?,
        pause_note_prefix: read(KEY_PAUSE_NOTE_PREFIX)?,
        pause_note_emphasis: read(KEY_PAUSE_NOTE_EMPHASIS)?,
        empty_deck: read(KEY_EMPTY_DECK)?,
        load_failed: read(KEY_LOAD_FAILED)?,
        answer_failed: read(KEY_ANSWER_FAILED)?,
        tactic_braid_suffix: read(KEY_TACTIC_BRAID_SUFFIX)?,
    })
}

#[cfg(test)]
#[path = "wording_practice_tests.rs"]
pub(crate) mod tests;
