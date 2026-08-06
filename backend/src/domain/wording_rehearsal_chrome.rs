// =============================================================================
// backend/src/domain/wording_rehearsal_chrome.rs — the rehearsal page's controls
// =============================================================================
//
// Task 2.11 C, Phase B. The eighteen strings the rebuilt page needs that
// `wording_rehearsal` does not already hold.
//
// ## Why a sibling module and not eighteen more fields next door
//
// Rule 17, first — `wording_rehearsal` sits at 234 non-comment lines and eighteen
// keys cost seventy-two more, which puts it past the limit. But the split earns
// itself on meaning as well as on arithmetic, and that is the part worth reading:
//
//   `wording_rehearsal`        — what the page SAYS ABOUT THE CASE. Headings,
//                                gap sentences, count templates, source labels.
//                                Every one of them describes the record.
//   `wording_rehearsal_chrome` — what the page's CONTROLS AND MARKERS say. Tags,
//                                side labels, button words, the authorship
//                                templates, the authoring note.
//
// The two change for different reasons. Roman rewords a gap sentence because the
// legal framing shifted; he rewords a button because the control moved. Nothing
// in this file names a fact, a date, or a document.
//
// ## The placeholder table is NOT duplicated
//
// `wording_templates::REQUIRED_PLACEHOLDERS` remains the one place the settings
// write path looks. The two templates below are listed there, beside their
// siblings. A second table would be a second lookup that can silently miss.
//
// ## Where the fixture lives
//
// Beside the tests, as in `wording_rehearsal` — one `TEST_SEED` table feeding
// `for_test` through the same builder production uses, so the fixture cannot
// drift from the shape and the migration test cannot drift from the fixture.

/// Every stored string on the rehearsal page's controls and markers.
///
/// ## Rust Learning: why this is a plain struct and not a `HashMap`
///
/// A map would compile with any key and fail at RUNTIME on a typo — on a witness
/// surface, as a blank button. Eighteen named fields mean the compiler checks
/// every read, and [`build_rehearsal_chrome_wording`] is the single place a key
/// string appears at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RehearsalChromeWording {
    // ── The instance row's two tags ─────────────────────────────────────────
    /// The small green tag on an answered instance row.
    pub answered_tag: String,
    /// The small red tag on an unanswered one — the QUIET form of the gap.
    pub no_answer_tag: String,
    /// The red banner inside an OPENED unanswered row. Louder than the tag,
    /// still carrying no who/when/where — that sentence lives once, in the prep
    /// list, and repeating it there was the beta.381 defect.
    pub no_answer_banner: String,

    // ── The timeline's two sides ────────────────────────────────────────────
    pub timeline_side_theirs_label: String,
    pub timeline_side_ours_label: String,

    // ── Who wrote the two authored sentences ────────────────────────────────
    /// Carries `{who}` and `{when}`.
    pub what_attribution_template: String,
    /// Carries `{who}` and `{when}`.
    pub accusation_attribution_template: String,
    /// Shown instead when the sentence predates the columns that record its
    /// author. A named absence — never an invented name, never a blank.
    pub attribution_unknown_notice: String,

    // ── The way back, and the way around ────────────────────────────────────
    pub scenario_page_label: String,
    pub crumb_trial_prep_label: String,
    pub go_to_row_label: String,

    // ── The accusation block's furniture ────────────────────────────────────
    pub prep_list_heading: String,
    pub row_open_hint: String,

    // ── The two authoring sections, in rehearsal's voice ────────────────────
    pub add_point_label: String,
    pub add_watch_label: String,
    pub point_no_exhibit_notice: String,
    pub points_authoring_note: String,
    /// The empty-box hint when writing "What this is".
    pub what_placeholder: String,
}

// KEYS: the stable identifiers of the eighteen stored strings. Not tunables —
// the NAMES of tunables, in the same category as a column name. Renaming one is a
// migration, and until it runs the boot loader refuses to start rather than guess.
pub(crate) const KEY_ANSWERED_TAG: &str = "rehearsal_answered_tag";
pub(crate) const KEY_NO_ANSWER_TAG: &str = "rehearsal_no_answer_tag";
pub(crate) const KEY_NO_ANSWER_BANNER: &str = "rehearsal_no_answer_banner";
pub(crate) const KEY_SIDE_THEIRS: &str = "rehearsal_timeline_side_theirs_label";
pub(crate) const KEY_SIDE_OURS: &str = "rehearsal_timeline_side_ours_label";
pub const KEY_WHAT_ATTRIBUTION: &str = "rehearsal_what_attribution_template";
pub const KEY_ACCUSATION_ATTRIBUTION: &str = "rehearsal_accusation_attribution_template";
pub(crate) const KEY_ATTRIBUTION_UNKNOWN: &str = "rehearsal_attribution_unknown_notice";
pub(crate) const KEY_SCENARIO_PAGE: &str = "rehearsal_scenario_page_label";
pub(crate) const KEY_CRUMB_TRIAL_PREP: &str = "rehearsal_crumb_trial_prep_label";
pub(crate) const KEY_GO_TO_ROW: &str = "rehearsal_go_to_row_label";
pub(crate) const KEY_PREP_LIST_HEADING: &str = "rehearsal_prep_list_heading";
pub(crate) const KEY_ROW_OPEN_HINT: &str = "rehearsal_row_open_hint";
pub(crate) const KEY_ADD_POINT: &str = "rehearsal_add_point_label";
pub(crate) const KEY_ADD_WATCH: &str = "rehearsal_add_watch_label";
pub(crate) const KEY_POINT_NO_EXHIBIT: &str = "rehearsal_point_no_exhibit_notice";
pub(crate) const KEY_POINTS_AUTHORING_NOTE: &str = "rehearsal_points_authoring_note";
pub(crate) const KEY_WHAT_PLACEHOLDER: &str = "rehearsal_what_placeholder";

/// Every chrome key this build reads, so a missing one is caught at boot BY NAME.
///
/// The fifth counted list. A boot log reading
/// `parameters=10 wording=48 accusation=27 rehearsal=41 rehearsal_chrome=18
/// authoring=23` names which part of a half-run seed is missing, which one summed
/// number could not.
pub const REHEARSAL_CHROME_KEYS: &[&str] = &[
    KEY_ANSWERED_TAG,
    KEY_NO_ANSWER_TAG,
    KEY_NO_ANSWER_BANNER,
    KEY_SIDE_THEIRS,
    KEY_SIDE_OURS,
    KEY_WHAT_ATTRIBUTION,
    KEY_ACCUSATION_ATTRIBUTION,
    KEY_ATTRIBUTION_UNKNOWN,
    KEY_SCENARIO_PAGE,
    KEY_CRUMB_TRIAL_PREP,
    KEY_GO_TO_ROW,
    KEY_PREP_LIST_HEADING,
    KEY_ROW_OPEN_HINT,
    KEY_ADD_POINT,
    KEY_ADD_WATCH,
    KEY_POINT_NO_EXHIBIT,
    KEY_POINTS_AUTHORING_NOTE,
    KEY_WHAT_PLACEHOLDER,
];

/// Build a [`RehearsalChromeWording`] from the stored rows, or say which key is
/// wrong.
///
/// ## Rust Learning: a closure that can fail, and why the error type is generic
///
/// `read` returns `Result<String, E>` for a caller-chosen `E`. Production passes
/// a closure that yields `SettingError`; the tests pass one that cannot fail
/// (`Infallible`) so they can feed each key its own name back and prove the
/// wiring one field at a time. One builder, both callers, no second copy of the
/// key-to-field mapping — which is the mapping a copy-paste silently breaks.
///
/// # Errors
/// Returns whatever `read` returns for the first key that is missing, of the
/// wrong declared kind, or blank.
pub fn build_rehearsal_chrome_wording<E>(
    read: impl Fn(&str) -> Result<String, E>,
) -> Result<RehearsalChromeWording, E> {
    Ok(RehearsalChromeWording {
        answered_tag: read(KEY_ANSWERED_TAG)?,
        no_answer_tag: read(KEY_NO_ANSWER_TAG)?,
        no_answer_banner: read(KEY_NO_ANSWER_BANNER)?,
        timeline_side_theirs_label: read(KEY_SIDE_THEIRS)?,
        timeline_side_ours_label: read(KEY_SIDE_OURS)?,
        what_attribution_template: read(KEY_WHAT_ATTRIBUTION)?,
        accusation_attribution_template: read(KEY_ACCUSATION_ATTRIBUTION)?,
        attribution_unknown_notice: read(KEY_ATTRIBUTION_UNKNOWN)?,
        scenario_page_label: read(KEY_SCENARIO_PAGE)?,
        crumb_trial_prep_label: read(KEY_CRUMB_TRIAL_PREP)?,
        go_to_row_label: read(KEY_GO_TO_ROW)?,
        prep_list_heading: read(KEY_PREP_LIST_HEADING)?,
        row_open_hint: read(KEY_ROW_OPEN_HINT)?,
        add_point_label: read(KEY_ADD_POINT)?,
        add_watch_label: read(KEY_ADD_WATCH)?,
        point_no_exhibit_notice: read(KEY_POINT_NO_EXHIBIT)?,
        points_authoring_note: read(KEY_POINTS_AUTHORING_NOTE)?,
        what_placeholder: read(KEY_WHAT_PLACEHOLDER)?,
    })
}

// `pub(crate)` for the same reason as its siblings: `settings_store_tests` needs
// this module's seeded fixture, and a second copy of eighteen sentences is a
// second thing to drift.
#[cfg(test)]
#[path = "wording_rehearsal_chrome_tests.rs"]
pub(crate) mod tests;
