// =============================================================================
// backend/src/domain/wording_practice_report.rs — the words that ANSWER HER BACK
// =============================================================================
//
// Screens S2 and S3 of PRACTICE_MOCKUP_v2: the reveal Marie reads after every
// answer, and the sheet Chuck reads at the end. Sibling of `wording_practice`,
// which holds the drill's own words; that module's header argues the seam.
//
// ## Domain note: two readers, one block
//
// The reveal is read by a witness alone; the sheet is read by her lawyer on
// paper. They are one block because the sheet's vocabulary is DERIVED from the
// reveal's — "repeat" is the mark on the sheet AND the word emphasised in its
// sub-line, and "opened" reports the drawer the reveal offered. Splitting them
// would let those drift, which is the one way a printed sheet can quietly stop
// describing the session it came from.
//
// ## Two sentences arrive in three rows each
//
// The mockup italicises one word inside "an example of *how*" and "the ones
// marked *repeat*". A row carrying `<i>` would put markup in the store; a row
// carrying the whole sentence would lose the emphasis. Prefix · emphasis ·
// suffix, and the component supplies the tag.

/// The words the reveal and Chuck's sheet speak.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PracticeReportWording {
    // ── S2 · the reveal ──────────────────────────────────────────────────
    /// Over her own answer, quoted back.
    pub what_you_said_kicker: String,
    /// The tag that marks the one sentence as the machine's, not Chuck's.
    pub read_tag: String,
    /// What the read is and what it is not. The last clause is the one that
    /// matters: the boxes are hers and cannot be wrong about her.
    pub read_footnote: String,
    /// Stands in the read's place when the call failed. Domain note: the boxes,
    /// the points, the pair and the watch-for all still stand — the session is
    /// not degraded by a model being down, it just says one less thing.
    pub read_unavailable: String,
    /// What the read says when it DECLINES rather than guesses.
    ///
    /// Domain note: this is not [`Self::read_unavailable`], and the difference is
    /// the whole point of the abstain arm. That line stands in when no read was
    /// attempted or none arrived — the machine was silent. This one is the read
    /// SPEAKING, saying it will not judge an answer on what it was given. When the
    /// model is the one declining, its own plain-English reason follows this line.
    pub read_abstain_line: String,
    /// The stored read for the one-click "I don't recall." control.
    ///
    /// Domain note: no model is called for it. The button sends a sentence this
    /// system wrote, and paying a model to judge our own words bought a sentence
    /// about a sentence at full token cost. It opens with the OK word, which is
    /// the correct verdict: "I don't recall" is a COMPLETE answer when it is true.
    pub read_dont_recall_line: String,
    /// Over the three talking points, which are read live from the scenario
    /// record.
    pub points_kicker: String,
    /// Opens a point's receipt line. The renderer supplies the following space.
    pub receipt_prefix: String,
    /// What a point with no paired exhibit says. The honest-gap law: a named
    /// absence, never a blank line under the point.
    pub point_no_receipt: String,
    /// Over the two-column pair.
    pub pair_kicker: String,
    /// Left column of the pair.
    pub pair_said_label: String,
    /// Right column of the pair — the half that wins the point.
    pub pair_admitted_label: String,
    /// Over the four boxes.
    pub check_kicker: String,
    /// Self-check box 1.
    pub check_only_asked: String,
    /// Self-check box 2 — the false premise, card 4.
    pub check_accepted_premise: String,
    /// Self-check box 3.
    pub check_explained_unasked: String,
    /// Self-check box 4.
    pub check_guessed: String,
    /// The collapsed drawer's own label. Opening it is recorded on the answer
    /// row and printed on Chuck's sheet (ruling R3).
    pub stronger_summary: String,
    /// The not-a-script line, up to its emphasised word.
    pub stronger_note_prefix: String,
    /// The italicised word in the not-a-script line. ABA Formal Op. 508: themes
    /// and key points, never a word-for-word script.
    pub stronger_note_emphasis: String,
    /// The rest of the not-a-script line. It opens with its own comma because
    /// the emphasised word precedes it with no space.
    pub stronger_note_suffix: String,
    /// What the drawer says when no point of hers answers the question.
    pub stronger_no_receipt: String,
    /// What the reveal says when the write settling an answer — her four boxes
    /// and fine/repeat — failed. Domain note: it names what IS safe first,
    /// because the sentence a witness needs most in that moment is that her
    /// typed answer did not disappear. She stays on this screen and the button
    /// still works.
    pub mark_not_recorded: String,
    /// What the reveal says when the write recording an opened drawer failed.
    /// Domain note: it is said in terms of the CONSEQUENCE, not the mechanism —
    /// Marie can do nothing about a failed POST, and what actually changed is
    /// that one cell on Chuck's sheet will be wrong. Her answer, her boxes and
    /// her mark are unaffected, and the sentence says nothing to suggest
    /// otherwise.
    pub help_not_recorded: String,
    /// Marks the answer fine and advances.
    pub next_button: String,
    /// Marks the answer repeat and re-queues the question in this same session.
    pub again_button: String,

    // ── S3 · Chuck's sheet ───────────────────────────────────────────────
    /// The sheet's eyebrow, composed server-side.
    pub sheet_kicker_template: String,
    /// The sheet's heading. {repeat} arrives as a whole clause so the zero case
    /// reads as a sentence rather than "0 to repeat".
    pub sheet_heading_template: String,
    /// The repeat clause when there is something to repeat.
    pub sheet_repeat_clause_template: String,
    /// The repeat clause when there is not.
    pub sheet_nothing_to_repeat: String,
    /// The sheet's sub-line, up to the emphasised mark.
    pub sheet_sub_prefix: String,
    /// The rest of the sheet's sub-line.
    pub sheet_sub_suffix: String,
    /// Sheet column 1.
    pub sheet_col_number: String,
    /// Sheet column 2.
    pub sheet_col_from: String,
    /// Sheet column 3.
    pub sheet_col_tactic: String,
    /// Sheet column 4.
    pub sheet_col_question: String,
    /// Sheet column 5 — verbatim, never summarised.
    pub sheet_col_answer: String,
    /// Sheet column 6.
    pub sheet_col_mark: String,
    /// Sheet column 7 — whether she opened the drawer.
    pub sheet_col_help: String,
    /// The From cell on a cross question.
    pub sheet_from_george: String,
    /// The From cell on a braid.
    pub sheet_from_george_braid: String,
    /// The From cell on a direct question.
    pub sheet_from_chuck: String,
    /// The Mark cell when nothing needs repeating.
    pub mark_fine: String,
    /// The Mark cell when it does — and the word emphasised in the sub-line.
    pub mark_repeat: String,
    /// The Help cell when she opened the drawer.
    pub help_opened: String,
    /// The Help cell when she did not.
    pub help_none: String,
    /// The Tactic cell on a question that carries none.
    pub tactic_none: String,
    /// Returns to the start screen.
    pub sheet_again_button: String,
    /// The ONLY print in this tool. FRE/MRE 612: nothing is printed for the
    /// witness until Chuck rules.
    pub print_button: String,
    /// The closing line of the sheet.
    pub homelab_line: String,
}

// KEYS: the stable identifiers of the rows above. Renaming one is a migration,
// and until it runs the boot loader refuses to start — which is the point: a
// witness surface with a blank where a sentence should be is not a degraded
// screen, it is a screen nobody can trust.
pub(crate) const KEY_WHAT_YOU_SAID_KICKER: &str = "practice_what_you_said_kicker";
pub(crate) const KEY_READ_TAG: &str = "practice_read_tag";
pub(crate) const KEY_READ_FOOTNOTE: &str = "practice_read_footnote";
pub(crate) const KEY_READ_UNAVAILABLE: &str = "practice_read_unavailable";
pub(crate) const KEY_READ_ABSTAIN_LINE: &str = "practice_read_abstain_line";
pub(crate) const KEY_READ_DONT_RECALL_LINE: &str = "practice_read_dont_recall_line";
pub(crate) const KEY_POINTS_KICKER: &str = "practice_points_kicker";
pub(crate) const KEY_RECEIPT_PREFIX: &str = "practice_receipt_prefix";
pub(crate) const KEY_POINT_NO_RECEIPT: &str = "practice_point_no_receipt";
pub(crate) const KEY_PAIR_KICKER: &str = "practice_pair_kicker";
pub(crate) const KEY_PAIR_SAID_LABEL: &str = "practice_pair_said_label";
pub(crate) const KEY_PAIR_ADMITTED_LABEL: &str = "practice_pair_admitted_label";
pub(crate) const KEY_CHECK_KICKER: &str = "practice_check_kicker";
pub(crate) const KEY_CHECK_ONLY_ASKED: &str = "practice_check_only_asked";
pub(crate) const KEY_CHECK_ACCEPTED_PREMISE: &str = "practice_check_accepted_premise";
pub(crate) const KEY_CHECK_EXPLAINED_UNASKED: &str = "practice_check_explained_unasked";
pub(crate) const KEY_CHECK_GUESSED: &str = "practice_check_guessed";
pub(crate) const KEY_STRONGER_SUMMARY: &str = "practice_stronger_summary";
pub(crate) const KEY_STRONGER_NOTE_PREFIX: &str = "practice_stronger_note_prefix";
pub(crate) const KEY_STRONGER_NOTE_EMPHASIS: &str = "practice_stronger_note_emphasis";
pub(crate) const KEY_STRONGER_NOTE_SUFFIX: &str = "practice_stronger_note_suffix";
pub(crate) const KEY_STRONGER_NO_RECEIPT: &str = "practice_stronger_no_receipt";
pub(crate) const KEY_MARK_NOT_RECORDED: &str = "practice_mark_not_recorded";
pub(crate) const KEY_HELP_NOT_RECORDED: &str = "practice_help_not_recorded";
pub(crate) const KEY_NEXT_BUTTON: &str = "practice_next_button";
pub(crate) const KEY_AGAIN_BUTTON: &str = "practice_again_button";
pub(crate) const KEY_SHEET_KICKER_TEMPLATE: &str = "practice_sheet_kicker_template";
pub(crate) const KEY_SHEET_HEADING_TEMPLATE: &str = "practice_sheet_heading_template";
pub(crate) const KEY_SHEET_REPEAT_CLAUSE_TEMPLATE: &str = "practice_sheet_repeat_clause_template";
pub(crate) const KEY_SHEET_NOTHING_TO_REPEAT: &str = "practice_sheet_nothing_to_repeat";
pub(crate) const KEY_SHEET_SUB_PREFIX: &str = "practice_sheet_sub_prefix";
pub(crate) const KEY_SHEET_SUB_SUFFIX: &str = "practice_sheet_sub_suffix";
pub(crate) const KEY_SHEET_COL_NUMBER: &str = "practice_sheet_col_number";
pub(crate) const KEY_SHEET_COL_FROM: &str = "practice_sheet_col_from";
pub(crate) const KEY_SHEET_COL_TACTIC: &str = "practice_sheet_col_tactic";
pub(crate) const KEY_SHEET_COL_QUESTION: &str = "practice_sheet_col_question";
pub(crate) const KEY_SHEET_COL_ANSWER: &str = "practice_sheet_col_answer";
pub(crate) const KEY_SHEET_COL_MARK: &str = "practice_sheet_col_mark";
pub(crate) const KEY_SHEET_COL_HELP: &str = "practice_sheet_col_help";
pub(crate) const KEY_SHEET_FROM_GEORGE: &str = "practice_sheet_from_george";
pub(crate) const KEY_SHEET_FROM_GEORGE_BRAID: &str = "practice_sheet_from_george_braid";
pub(crate) const KEY_SHEET_FROM_CHUCK: &str = "practice_sheet_from_chuck";
pub(crate) const KEY_MARK_FINE: &str = "practice_mark_fine";
pub(crate) const KEY_MARK_REPEAT: &str = "practice_mark_repeat";
pub(crate) const KEY_HELP_OPENED: &str = "practice_help_opened";
pub(crate) const KEY_HELP_NONE: &str = "practice_help_none";
pub(crate) const KEY_TACTIC_NONE: &str = "practice_tactic_none";
pub(crate) const KEY_SHEET_AGAIN_BUTTON: &str = "practice_sheet_again_button";
pub(crate) const KEY_PRINT_BUTTON: &str = "practice_print_button";
pub(crate) const KEY_HOMELAB_LINE: &str = "practice_homelab_line";

/// Every key in this block, so a missing one is caught at boot BY NAME rather
/// than as a blank control in front of Marie mid-session.
pub const PRACTICE_REPORT_WORDING_KEYS: &[&str] = &[
    KEY_WHAT_YOU_SAID_KICKER,
    KEY_READ_TAG,
    KEY_READ_FOOTNOTE,
    KEY_READ_UNAVAILABLE,
    KEY_READ_ABSTAIN_LINE,
    KEY_READ_DONT_RECALL_LINE,
    KEY_POINTS_KICKER,
    KEY_RECEIPT_PREFIX,
    KEY_POINT_NO_RECEIPT,
    KEY_PAIR_KICKER,
    KEY_PAIR_SAID_LABEL,
    KEY_PAIR_ADMITTED_LABEL,
    KEY_CHECK_KICKER,
    KEY_CHECK_ONLY_ASKED,
    KEY_CHECK_ACCEPTED_PREMISE,
    KEY_CHECK_EXPLAINED_UNASKED,
    KEY_CHECK_GUESSED,
    KEY_STRONGER_SUMMARY,
    KEY_STRONGER_NOTE_PREFIX,
    KEY_STRONGER_NOTE_EMPHASIS,
    KEY_STRONGER_NOTE_SUFFIX,
    KEY_STRONGER_NO_RECEIPT,
    KEY_MARK_NOT_RECORDED,
    KEY_HELP_NOT_RECORDED,
    KEY_NEXT_BUTTON,
    KEY_AGAIN_BUTTON,
    KEY_SHEET_KICKER_TEMPLATE,
    KEY_SHEET_HEADING_TEMPLATE,
    KEY_SHEET_REPEAT_CLAUSE_TEMPLATE,
    KEY_SHEET_NOTHING_TO_REPEAT,
    KEY_SHEET_SUB_PREFIX,
    KEY_SHEET_SUB_SUFFIX,
    KEY_SHEET_COL_NUMBER,
    KEY_SHEET_COL_FROM,
    KEY_SHEET_COL_TACTIC,
    KEY_SHEET_COL_QUESTION,
    KEY_SHEET_COL_ANSWER,
    KEY_SHEET_COL_MARK,
    KEY_SHEET_COL_HELP,
    KEY_SHEET_FROM_GEORGE,
    KEY_SHEET_FROM_GEORGE_BRAID,
    KEY_SHEET_FROM_CHUCK,
    KEY_MARK_FINE,
    KEY_MARK_REPEAT,
    KEY_HELP_OPENED,
    KEY_HELP_NONE,
    KEY_TACTIC_NONE,
    KEY_SHEET_AGAIN_BUTTON,
    KEY_PRINT_BUTTON,
    KEY_HOMELAB_LINE,
];

/// Build a [`PracticeReportWording`] from the stored rows, or say which key is wrong.
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
pub fn build_practice_report_wording<E>(
    read: impl Fn(&str) -> Result<String, E>,
) -> Result<PracticeReportWording, E> {
    Ok(PracticeReportWording {
        what_you_said_kicker: read(KEY_WHAT_YOU_SAID_KICKER)?,
        read_tag: read(KEY_READ_TAG)?,
        read_footnote: read(KEY_READ_FOOTNOTE)?,
        read_unavailable: read(KEY_READ_UNAVAILABLE)?,
        read_abstain_line: read(KEY_READ_ABSTAIN_LINE)?,
        read_dont_recall_line: read(KEY_READ_DONT_RECALL_LINE)?,
        points_kicker: read(KEY_POINTS_KICKER)?,
        receipt_prefix: read(KEY_RECEIPT_PREFIX)?,
        point_no_receipt: read(KEY_POINT_NO_RECEIPT)?,
        pair_kicker: read(KEY_PAIR_KICKER)?,
        pair_said_label: read(KEY_PAIR_SAID_LABEL)?,
        pair_admitted_label: read(KEY_PAIR_ADMITTED_LABEL)?,
        check_kicker: read(KEY_CHECK_KICKER)?,
        check_only_asked: read(KEY_CHECK_ONLY_ASKED)?,
        check_accepted_premise: read(KEY_CHECK_ACCEPTED_PREMISE)?,
        check_explained_unasked: read(KEY_CHECK_EXPLAINED_UNASKED)?,
        check_guessed: read(KEY_CHECK_GUESSED)?,
        stronger_summary: read(KEY_STRONGER_SUMMARY)?,
        stronger_note_prefix: read(KEY_STRONGER_NOTE_PREFIX)?,
        stronger_note_emphasis: read(KEY_STRONGER_NOTE_EMPHASIS)?,
        stronger_note_suffix: read(KEY_STRONGER_NOTE_SUFFIX)?,
        stronger_no_receipt: read(KEY_STRONGER_NO_RECEIPT)?,
        mark_not_recorded: read(KEY_MARK_NOT_RECORDED)?,
        help_not_recorded: read(KEY_HELP_NOT_RECORDED)?,
        next_button: read(KEY_NEXT_BUTTON)?,
        again_button: read(KEY_AGAIN_BUTTON)?,
        sheet_kicker_template: read(KEY_SHEET_KICKER_TEMPLATE)?,
        sheet_heading_template: read(KEY_SHEET_HEADING_TEMPLATE)?,
        sheet_repeat_clause_template: read(KEY_SHEET_REPEAT_CLAUSE_TEMPLATE)?,
        sheet_nothing_to_repeat: read(KEY_SHEET_NOTHING_TO_REPEAT)?,
        sheet_sub_prefix: read(KEY_SHEET_SUB_PREFIX)?,
        sheet_sub_suffix: read(KEY_SHEET_SUB_SUFFIX)?,
        sheet_col_number: read(KEY_SHEET_COL_NUMBER)?,
        sheet_col_from: read(KEY_SHEET_COL_FROM)?,
        sheet_col_tactic: read(KEY_SHEET_COL_TACTIC)?,
        sheet_col_question: read(KEY_SHEET_COL_QUESTION)?,
        sheet_col_answer: read(KEY_SHEET_COL_ANSWER)?,
        sheet_col_mark: read(KEY_SHEET_COL_MARK)?,
        sheet_col_help: read(KEY_SHEET_COL_HELP)?,
        sheet_from_george: read(KEY_SHEET_FROM_GEORGE)?,
        sheet_from_george_braid: read(KEY_SHEET_FROM_GEORGE_BRAID)?,
        sheet_from_chuck: read(KEY_SHEET_FROM_CHUCK)?,
        mark_fine: read(KEY_MARK_FINE)?,
        mark_repeat: read(KEY_MARK_REPEAT)?,
        help_opened: read(KEY_HELP_OPENED)?,
        help_none: read(KEY_HELP_NONE)?,
        tactic_none: read(KEY_TACTIC_NONE)?,
        sheet_again_button: read(KEY_SHEET_AGAIN_BUTTON)?,
        print_button: read(KEY_PRINT_BUTTON)?,
        homelab_line: read(KEY_HOMELAB_LINE)?,
    })
}

#[cfg(test)]
#[path = "wording_practice_report_tests.rs"]
pub(crate) mod tests;
