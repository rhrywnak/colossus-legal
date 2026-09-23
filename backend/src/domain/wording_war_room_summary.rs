// =============================================================================
// backend/src/domain/wording_war_room_summary.rs — the War Room's summary card
// =============================================================================
//
// CC_TASK_SIMPLE_COUNTS_v1, ruled on WAR_ROOM_SUMMARY_CARD_RULED_2026-09-17. The
// words of the ONE summary card that replaced the four-tile strip and the metric
// band: a quiet top row, and three cells — one queue each, each owned by a named
// person.
//
// ## Why a nested block and not more fields on `WarRoomWording`
//
// Rule 17: nineteen more fields, keys, list entries and builder lines would take
// `wording_war_room.rs` past 300 lines. The seam is real as well: the card
// speaks about the CASE (three queues across every scenario), the rest of that
// block about ONE scenario's status card. Same nesting as `wording_practice_row`
// inside `wording_practice`.
//
// ## Domain note: three owners, one truth
//
// Every number these words frame is identical for every viewer. The reviewer's
// name is NOT here: it is the `practice_reviewer_display_name` settings row, so
// changing attorney never means editing a sentence.

/// The words the War Room's summary card speaks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WarRoomSummaryWording {
    /// The top row's label beside the answered bar.
    pub answered_label: String,
    /// After the bold answered number: `{total}` and `{pct}`.
    pub answered_rest_template: String,
    /// Marie's cell label.
    pub unanswered_label: String,
    /// The reviewer's cell label.
    pub review_label: String,
    /// The review tile's owner chip, where the other two print MARIE and ROMAN.
    ///
    /// Domain note: it says YOU because the tile is only ever drawn for the
    /// person whose backlog it counts (ruled 2026-09-22). Its predecessor was
    /// not a stored row at all — the chip took the reviewer bench's display
    /// names, which named somebody else on a number that is the reader's own.
    pub review_chip: String,
    /// Roman's cell label.
    pub candidates_label: String,
    /// The owner chip on Marie's cell. (The reviewer's chip prints the display-name settings row.)
    pub owner_marie: String,
    /// The owner chip on Roman's cell.
    pub owner_roman: String,
    /// Marie's context: `{n}` scenarios with unanswered questions, `{codes}` up to three untouched.
    pub unanswered_context_template: String,
    /// Its singular, at exactly one scenario.
    pub unanswered_context_one: String,
    /// Marie's context when no scenario is untouched.
    pub unanswered_context_none_untouched: String,
    /// Its singular.
    pub unanswered_context_none_untouched_one: String,
    /// The reviewer's context: the oldest waiting day and the largest pile.
    pub review_context_template: String,
    /// One pile in Roman's context line.
    pub candidates_pile_template: String,
    /// Between piles. Stored trimmed; the page adds the spaces.
    pub list_joiner: String,
    /// Between codes sharing one pile size.
    pub tie_joiner: String,
    /// Between untouched codes; the page adds the space after it.
    pub code_joiner: String,
    /// Marie's context at zero (GO ruling 2: a satisfied queue says so).
    pub unanswered_zero: String,
    /// The reviewer's context at zero.
    pub review_zero: String,
    /// Roman's context at zero.
    pub candidates_zero: String,
    /// Appended to Marie's context: `{n}` new or changed — the SUM of the cards'
    /// own pill number (CC_GO_QUESTION_CHAT_v1, STOP 2 ruling). Hidden at zero.
    pub unanswered_changed_clause: String,
}

// KEYS: the stable identifiers. Renaming one is a migration, and until it runs
// the boot loader refuses to start.
// STRUCTURAL: these are the names of rows in `app_settings`, not values read
// from them. A key is wire vocabulary shared with the database — the VALUES are
// what Standing Rule 2 governs, and every one of them is a stored row read at
// boot. (The marker arrives with `KEY_REVIEW_CHIP`, which is the first key this
// block has gained since the rule was written down in `wording_fact_card`.)
pub(crate) const KEY_ANSWERED_LABEL: &str = "war_room_summary_answered_label";
pub(crate) const KEY_ANSWERED_REST_TEMPLATE: &str = "war_room_summary_answered_rest_template";
pub(crate) const KEY_UNANSWERED_LABEL: &str = "war_room_summary_unanswered_label";
pub(crate) const KEY_REVIEW_LABEL: &str = "war_room_summary_review_label";
pub(crate) const KEY_REVIEW_CHIP: &str = "war_room_summary_review_chip";
pub(crate) const KEY_CANDIDATES_LABEL: &str = "war_room_summary_candidates_label";
pub(crate) const KEY_OWNER_MARIE: &str = "war_room_owner_marie";
pub(crate) const KEY_OWNER_ROMAN: &str = "war_room_owner_roman";
pub(crate) const KEY_UNANSWERED_CONTEXT_TEMPLATE: &str =
    "war_room_summary_unanswered_context_template";
pub(crate) const KEY_UNANSWERED_CONTEXT_ONE: &str = "war_room_summary_unanswered_context_one";
pub(crate) const KEY_UNANSWERED_CONTEXT_NONE_UNTOUCHED: &str =
    "war_room_summary_unanswered_context_none_untouched";
pub(crate) const KEY_UNANSWERED_CONTEXT_NONE_UNTOUCHED_ONE: &str =
    "war_room_summary_unanswered_context_none_untouched_one";
pub(crate) const KEY_REVIEW_CONTEXT_TEMPLATE: &str = "war_room_summary_review_context_template";
pub(crate) const KEY_CANDIDATES_PILE_TEMPLATE: &str = "war_room_summary_candidates_pile_template";
pub(crate) const KEY_LIST_JOINER: &str = "war_room_summary_list_joiner";
pub(crate) const KEY_TIE_JOINER: &str = "war_room_summary_tie_joiner";
pub(crate) const KEY_CODE_JOINER: &str = "war_room_summary_code_joiner";
pub(crate) const KEY_UNANSWERED_ZERO: &str = "war_room_summary_unanswered_zero";
pub(crate) const KEY_REVIEW_ZERO: &str = "war_room_summary_review_zero";
pub(crate) const KEY_CANDIDATES_ZERO: &str = "war_room_summary_candidates_zero";
pub(crate) const KEY_UNANSWERED_CHANGED_CLAUSE: &str = "war_room_summary_unanswered_changed_clause";

/// Every summary-card key this build reads, so a missing one is caught at boot
/// BY NAME rather than as a blank cell.
pub const WAR_ROOM_SUMMARY_WORDING_KEYS: &[&str] = &[
    KEY_ANSWERED_LABEL,
    KEY_ANSWERED_REST_TEMPLATE,
    KEY_UNANSWERED_LABEL,
    KEY_REVIEW_LABEL,
    KEY_REVIEW_CHIP,
    KEY_CANDIDATES_LABEL,
    KEY_OWNER_MARIE,
    KEY_OWNER_ROMAN,
    KEY_UNANSWERED_CONTEXT_TEMPLATE,
    KEY_UNANSWERED_CONTEXT_ONE,
    KEY_UNANSWERED_CONTEXT_NONE_UNTOUCHED,
    KEY_UNANSWERED_CONTEXT_NONE_UNTOUCHED_ONE,
    KEY_REVIEW_CONTEXT_TEMPLATE,
    KEY_CANDIDATES_PILE_TEMPLATE,
    KEY_LIST_JOINER,
    KEY_TIE_JOINER,
    KEY_CODE_JOINER,
    KEY_UNANSWERED_ZERO,
    KEY_REVIEW_ZERO,
    KEY_CANDIDATES_ZERO,
    KEY_UNANSWERED_CHANGED_CLAUSE,
];

/// Build a [`WarRoomSummaryWording`] from the stored rows, or say which key is wrong.
///
/// Called from `wording_war_room::build_war_room_wording` with the same reader,
/// so both blocks are judged by one rule — see
/// [`crate::domain::wording_practice_row::build_practice_row_wording`] for why the
/// closure is taken by reference.
///
/// # Errors
/// Returns whatever `read` returns for the first key that is missing, of the
/// wrong declared kind, or blank.
pub fn build_war_room_summary_wording<E>(
    read: impl Fn(&str) -> Result<String, E>,
) -> Result<WarRoomSummaryWording, E> {
    Ok(WarRoomSummaryWording {
        answered_label: read(KEY_ANSWERED_LABEL)?,
        answered_rest_template: read(KEY_ANSWERED_REST_TEMPLATE)?,
        unanswered_label: read(KEY_UNANSWERED_LABEL)?,
        review_label: read(KEY_REVIEW_LABEL)?,
        review_chip: read(KEY_REVIEW_CHIP)?,
        candidates_label: read(KEY_CANDIDATES_LABEL)?,
        owner_marie: read(KEY_OWNER_MARIE)?,
        owner_roman: read(KEY_OWNER_ROMAN)?,
        unanswered_context_template: read(KEY_UNANSWERED_CONTEXT_TEMPLATE)?,
        unanswered_context_one: read(KEY_UNANSWERED_CONTEXT_ONE)?,
        unanswered_context_none_untouched: read(KEY_UNANSWERED_CONTEXT_NONE_UNTOUCHED)?,
        unanswered_context_none_untouched_one: read(KEY_UNANSWERED_CONTEXT_NONE_UNTOUCHED_ONE)?,
        review_context_template: read(KEY_REVIEW_CONTEXT_TEMPLATE)?,
        candidates_pile_template: read(KEY_CANDIDATES_PILE_TEMPLATE)?,
        list_joiner: read(KEY_LIST_JOINER)?,
        tie_joiner: read(KEY_TIE_JOINER)?,
        code_joiner: read(KEY_CODE_JOINER)?,
        unanswered_zero: read(KEY_UNANSWERED_ZERO)?,
        review_zero: read(KEY_REVIEW_ZERO)?,
        candidates_zero: read(KEY_CANDIDATES_ZERO)?,
        unanswered_changed_clause: read(KEY_UNANSWERED_CHANGED_CLAUSE)?,
    })
}

#[cfg(test)]
#[path = "wording_war_room_summary_tests.rs"]
pub(crate) mod seed_tests;
