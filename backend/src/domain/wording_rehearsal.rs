// =============================================================================
// backend/src/domain/wording_rehearsal.rs — the rehearsal page's own words
// =============================================================================
//
// Task 2.11 B2. REHEARSAL_B2_ADDENDUM_v1 §4, signed 2026-08-06: "every heading,
// label, gap message, and the Always card from wording rows."
//
// The shipped page carried roughly eighteen literals in code — the purpose line,
// all four block labels, the position sentence, both empty states, and the
// standing card itself. Every one of them is a row now. There is not one
// user-facing literal below outside the `cfg(test)` fixture.
//
// ## The fourth stored-string module, and why the pattern keeps repeating
//
// `domain::wording` (48 keys, the curation surfaces) · `domain::wording_accusation`
// (27, the working view's accusation section) · this one (40, the rehearsal
// page). The sibling-module pattern was approved 2026-08-06 for the reason
// `wording_accusation`'s header gives: `domain::wording` is over Rule 17's limit
// and cannot absorb another block, and — more usefully — these are the words ONE
// surface speaks, changing for reasons no other surface shares.
//
// What is deliberately NOT duplicated is the placeholder table.
// `wording::REQUIRED_PLACEHOLDERS` remains the one place the settings write path
// looks; this module's templates are listed there. A second table would be a
// second lookup that can silently miss, which is the exact failure that table
// exists to prevent.
//
// ## Why the test fixture is a TABLE and not a second hand-typed struct
//
// `wording_accusation` spells its fixture twice — once as a struct literal, once
// as a match returning each field. At 27 keys that was tolerable; at 39 it would
// be eighty lines of duplication that can drift against itself. Here one
// `TEST_SEED` table feeds both `for_test` (through the same `build_` function
// production uses) and `for_test_values`, so there is one copy and the builder is
// exercised by every test that touches the fixture.

use crate::domain::settings::SettingError;

/// Every stored string the rehearsal page renders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RehearsalWording {
    // ── The page's chrome ───────────────────────────────────────────────────
    /// The heading at the top of the page.
    pub page_heading: String,
    /// What this view is for, and what decides its contents.
    pub purpose_line: String,
    /// "Scenario {n} of {total}".
    pub position_template: String,
    /// Moves to the previous ready scenario.
    pub previous_label: String,
    /// Moves to the next one.
    pub next_label: String,
    /// Shown when the case has no ready scenario at all.
    pub nothing_ready_notice: String,
    /// Shown for the rehearsal address of a scenario nobody declared ready.
    /// Carries `{code}` — the only thing this page may say about a scenario it
    /// is not showing.
    pub not_ready_notice: String,
    /// Opens every collapsible section.
    pub expand_all_label: String,
    /// Folds them all away. The Always card is never folded.
    pub collapse_all_label: String,

    // ── The seven block headings ────────────────────────────────────────────
    pub block_what_heading: String,
    pub block_accusation_heading: String,
    pub block_timeline_heading: String,
    pub block_points_heading: String,
    pub block_watch_heading: String,
    pub always_heading: String,
    /// The standing card's four lines, ` · `-separated. See [`always_lines`].
    pub always_lines: String,

    // ── The headers that must stay honest when a section is folded ──────────
    /// "said {times} times · {gaps} gaps" — visible open OR folded.
    pub accusation_header_template: String,
    /// "{entries} dated items".
    pub timeline_header_template: String,
    /// "{shown} of {cap}".
    pub points_header_template: String,
    /// "{count} to watch for".
    pub watch_header_template: String,

    // ── The named gaps ──────────────────────────────────────────────────────
    pub what_gap: String,
    pub accusation_text_gap: String,
    /// Shown when nobody has marked an instance — a different state from "no
    /// accusation written", with a different remedy.
    pub no_instances_notice: String,
    /// The prep list. Carries `{who}`, `{when}`, `{where}`.
    pub gap_no_answer: String,
    /// The Remove law, statement side.
    pub gap_accusation_removed: String,
    /// The Remove law, answer side. Carries `{who}` and `{when}`.
    pub gap_answer_removed: String,
    /// A marked instance the record store can no longer produce — a SYSTEM
    /// fault, worded so it does not read as a human's act.
    pub gap_instance_unavailable: String,
    pub points_gap: String,
    pub watch_gap: String,
    pub instance_when_gap: String,
    pub instance_who_gap: String,
    /// Carries `{undated}` and `{total}`.
    pub timeline_gap_template: String,
    pub timeline_filter_prompt: String,
    pub timeline_filter_all_label: String,

    // ── Sources ─────────────────────────────────────────────────────────────
    /// "{document}, p. {page}".
    pub source_label_template: String,
    pub source_open_label: String,

    // ── Which sections start open. Parsed, never rendered. ──────────────────
    /// Task 2.11 C: "What this is" folds like the others as of the signed
    /// mockup. B2 had it fixed open, on the argument that folding one sentence
    /// saves nothing — the mockup gives it a caret, and the mockup is the spec.
    pub what_default_state: String,
    pub accusation_default_state: String,
    pub timeline_default_state: String,
    pub points_default_state: String,
    pub watch_default_state: String,
}

// KEYS: the stable identifiers of the forty stored strings. Not tunables —
// the NAMES of tunables, in the same category as a column name. Renaming one is a
// migration, and until it runs the boot loader refuses to start rather than guess.
pub(crate) const KEY_PAGE_HEADING: &str = "rehearsal_page_heading";
pub(crate) const KEY_PURPOSE_LINE: &str = "rehearsal_purpose_line";
pub const KEY_POSITION_TEMPLATE: &str = "rehearsal_position_template";
pub(crate) const KEY_PREVIOUS_LABEL: &str = "rehearsal_previous_label";
pub(crate) const KEY_NEXT_LABEL: &str = "rehearsal_next_label";
pub(crate) const KEY_NOTHING_READY: &str = "rehearsal_nothing_ready_notice";
pub const KEY_NOT_READY: &str = "rehearsal_not_ready_notice";
pub(crate) const KEY_EXPAND_ALL: &str = "rehearsal_expand_all_label";
pub(crate) const KEY_COLLAPSE_ALL: &str = "rehearsal_collapse_all_label";
pub(crate) const KEY_BLOCK_WHAT: &str = "rehearsal_block_what_heading";
pub(crate) const KEY_BLOCK_ACCUSATION: &str = "rehearsal_block_accusation_heading";
pub(crate) const KEY_BLOCK_TIMELINE: &str = "rehearsal_block_timeline_heading";
pub(crate) const KEY_BLOCK_POINTS: &str = "rehearsal_block_points_heading";
pub(crate) const KEY_BLOCK_WATCH: &str = "rehearsal_block_watch_heading";
pub(crate) const KEY_ALWAYS_HEADING: &str = "rehearsal_always_heading";
pub(crate) const KEY_ALWAYS_LINES: &str = "rehearsal_always_lines";
pub const KEY_ACCUSATION_HEADER: &str = "rehearsal_accusation_header_template";
pub const KEY_TIMELINE_HEADER: &str = "rehearsal_timeline_header_template";
pub const KEY_POINTS_HEADER: &str = "rehearsal_points_header_template";
pub const KEY_WATCH_HEADER: &str = "rehearsal_watch_header_template";
pub(crate) const KEY_WHAT_GAP: &str = "rehearsal_what_gap";
pub(crate) const KEY_ACCUSATION_TEXT_GAP: &str = "rehearsal_accusation_text_gap";
pub(crate) const KEY_NO_INSTANCES: &str = "rehearsal_no_instances_notice";
pub const KEY_GAP_NO_ANSWER: &str = "rehearsal_gap_no_answer";
pub(crate) const KEY_GAP_ACCUSATION_REMOVED: &str = "rehearsal_gap_accusation_removed";
pub const KEY_GAP_ANSWER_REMOVED: &str = "rehearsal_gap_answer_removed";
pub(crate) const KEY_GAP_UNAVAILABLE: &str = "rehearsal_gap_instance_unavailable";
pub(crate) const KEY_POINTS_GAP: &str = "rehearsal_points_gap";
pub(crate) const KEY_WATCH_GAP: &str = "rehearsal_watch_gap";
pub(crate) const KEY_INSTANCE_WHEN_GAP: &str = "rehearsal_instance_when_gap";
pub(crate) const KEY_INSTANCE_WHO_GAP: &str = "rehearsal_instance_who_gap";
pub const KEY_TIMELINE_GAP: &str = "rehearsal_timeline_gap_template";
pub(crate) const KEY_TIMELINE_FILTER_PROMPT: &str = "rehearsal_timeline_filter_prompt";
pub(crate) const KEY_TIMELINE_FILTER_ALL: &str = "rehearsal_timeline_filter_all_label";
pub const KEY_SOURCE_LABEL: &str = "rehearsal_source_label_template";
pub(crate) const KEY_SOURCE_OPEN: &str = "rehearsal_source_open_label";
pub(crate) const KEY_WHAT_STATE: &str = "rehearsal_what_default_state";
pub(crate) const KEY_ACCUSATION_STATE: &str = "rehearsal_accusation_default_state";
pub(crate) const KEY_TIMELINE_STATE: &str = "rehearsal_timeline_default_state";
pub(crate) const KEY_POINTS_STATE: &str = "rehearsal_points_default_state";
pub(crate) const KEY_WATCH_STATE: &str = "rehearsal_watch_default_state";

/// Every rehearsal wording key this build reads, so a missing one is caught at
/// boot BY NAME.
///
/// The fourth counted list. A boot log reading `parameters=9 wording=48
/// accusation=27 rehearsal=39` names which half of a half-run seed is missing,
/// which one summed number could not.
pub const REHEARSAL_WORDING_KEYS: &[&str] = &[
    KEY_PAGE_HEADING,
    KEY_PURPOSE_LINE,
    KEY_POSITION_TEMPLATE,
    KEY_PREVIOUS_LABEL,
    KEY_NEXT_LABEL,
    KEY_NOTHING_READY,
    KEY_NOT_READY,
    KEY_EXPAND_ALL,
    KEY_COLLAPSE_ALL,
    KEY_BLOCK_WHAT,
    KEY_BLOCK_ACCUSATION,
    KEY_BLOCK_TIMELINE,
    KEY_BLOCK_POINTS,
    KEY_BLOCK_WATCH,
    KEY_ALWAYS_HEADING,
    KEY_ALWAYS_LINES,
    KEY_ACCUSATION_HEADER,
    KEY_TIMELINE_HEADER,
    KEY_POINTS_HEADER,
    KEY_WATCH_HEADER,
    KEY_WHAT_GAP,
    KEY_ACCUSATION_TEXT_GAP,
    KEY_NO_INSTANCES,
    KEY_GAP_NO_ANSWER,
    KEY_GAP_ACCUSATION_REMOVED,
    KEY_GAP_ANSWER_REMOVED,
    KEY_GAP_UNAVAILABLE,
    KEY_POINTS_GAP,
    KEY_WATCH_GAP,
    KEY_INSTANCE_WHEN_GAP,
    KEY_INSTANCE_WHO_GAP,
    KEY_TIMELINE_GAP,
    KEY_TIMELINE_FILTER_PROMPT,
    KEY_TIMELINE_FILTER_ALL,
    KEY_SOURCE_LABEL,
    KEY_SOURCE_OPEN,
    KEY_WHAT_STATE,
    KEY_ACCUSATION_STATE,
    KEY_TIMELINE_STATE,
    KEY_POINTS_STATE,
    KEY_WATCH_STATE,
];

/// The separator between the standing card's lines.
///
/// ## Rust Learning: a `const` that is FORMAT, not wording
///
/// Standing Rule 2 asks whether a value varies across environments or cases.
/// This one cannot: it is the delimiter the row's own description tells an editor
/// to type, so changing it in code would silently stop parsing every value
/// already stored. It is the row's grammar, versioned with the parser — the same
/// standing as a serde tag.
const ALWAYS_LINE_SEPARATOR: &str = " · ";

/// Whether a section starts open.
///
/// ## Rust Learning: a two-variant enum instead of `value == "open"`
///
/// The store has no boolean kind (ruled 2026-08-05), so the tempting decode is a
/// string comparison — which silently treats every typo ("Open", "opne", "true")
/// as the other variant. On this surface that means a section a witness needs
/// quietly folding shut with nothing in the log. Parsing into a closed enum makes
/// an unrecognised token a named failure at boot, the same discipline
/// `BackgroundDefaultState` uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionState {
    Open,
    Collapsed,
}

impl SectionState {
    /// Whether a section holding this state renders open.
    pub fn is_open(self) -> bool {
        matches!(self, SectionState::Open)
    }

    /// Read one stored state token, naming the key that carried it.
    ///
    /// # Errors
    /// Returns [`SettingError::Unreadable`] naming the key and what was expected.
    pub fn parse(key: &str, token: &str) -> Result<Self, SettingError> {
        match token.trim() {
            "open" => Ok(SectionState::Open),
            "collapsed" => Ok(SectionState::Collapsed),
            other => Err(SettingError::Unreadable {
                key: key.to_string(),
                value: other.to_string(),
                expected: "either 'open' or 'collapsed'",
            }),
        }
    }
}

impl RehearsalWording {
    /// The standing card's lines, split and trimmed.
    ///
    /// ## Why this is fallible, and why an empty result is a REFUSAL
    ///
    /// A blank standing card is the one failure this page cannot absorb: §10
    /// makes it the block that is never scrolled away from, and it carries ABA
    /// Formal Opinion 508's substance — the reason rehearsal is theme-level
    /// rather than a script. A value edited to nothing would render an empty
    /// bordered box with a heading over it, which is worse than an error because
    /// it looks deliberate.
    ///
    /// # Errors
    /// Returns [`SettingError::Blank`] naming the key when the value yields no
    /// non-empty lines.
    pub fn always_lines(&self) -> Result<Vec<String>, SettingError> {
        let lines: Vec<String> = self
            .always_lines
            .split(ALWAYS_LINE_SEPARATOR)
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect();

        if lines.is_empty() {
            return Err(SettingError::Blank {
                key: KEY_ALWAYS_LINES.to_string(),
            });
        }
        Ok(lines)
    }

    /// The five section states, parsed once — in the order the page renders them.
    ///
    /// ## Why the order is load-bearing
    ///
    /// The caller reads this array positionally into [`crate::dto::rehearsal::
    /// RehearsalCollapse`]. Page order (what → accusation → timeline → points →
    /// watch) means a reader comparing this list to the screen is comparing two
    /// things in the same sequence; any other order invites the copy-paste that
    /// opens the wrong section.
    ///
    /// # Errors
    /// Returns [`SettingError`] naming the first key whose token is not one of
    /// the two this build understands.
    pub fn section_states(&self) -> Result<[(&'static str, SectionState); 5], SettingError> {
        Ok([
            (
                KEY_WHAT_STATE,
                SectionState::parse(KEY_WHAT_STATE, &self.what_default_state)?,
            ),
            (
                KEY_ACCUSATION_STATE,
                SectionState::parse(KEY_ACCUSATION_STATE, &self.accusation_default_state)?,
            ),
            (
                KEY_TIMELINE_STATE,
                SectionState::parse(KEY_TIMELINE_STATE, &self.timeline_default_state)?,
            ),
            (
                KEY_POINTS_STATE,
                SectionState::parse(KEY_POINTS_STATE, &self.points_default_state)?,
            ),
            (
                KEY_WATCH_STATE,
                SectionState::parse(KEY_WATCH_STATE, &self.watch_default_state)?,
            ),
        ])
    }
}

/// Build a [`RehearsalWording`] from the stored rows, or say which key is wrong.
///
/// Same shape and same reasoning as `wording::build_wording` — see its
/// `## Rust Learning` note on taking a closure that can fail. This module knows
/// the KEYS and the SHAPE and nothing about databases.
///
/// # Errors
/// Returns whatever `read` returns for the first key that is missing, of the
/// wrong declared kind, or blank.
pub fn build_rehearsal_wording<E>(
    read: impl Fn(&str) -> Result<String, E>,
) -> Result<RehearsalWording, E> {
    Ok(RehearsalWording {
        page_heading: read(KEY_PAGE_HEADING)?,
        purpose_line: read(KEY_PURPOSE_LINE)?,
        position_template: read(KEY_POSITION_TEMPLATE)?,
        previous_label: read(KEY_PREVIOUS_LABEL)?,
        next_label: read(KEY_NEXT_LABEL)?,
        nothing_ready_notice: read(KEY_NOTHING_READY)?,
        not_ready_notice: read(KEY_NOT_READY)?,
        expand_all_label: read(KEY_EXPAND_ALL)?,
        collapse_all_label: read(KEY_COLLAPSE_ALL)?,
        block_what_heading: read(KEY_BLOCK_WHAT)?,
        block_accusation_heading: read(KEY_BLOCK_ACCUSATION)?,
        block_timeline_heading: read(KEY_BLOCK_TIMELINE)?,
        block_points_heading: read(KEY_BLOCK_POINTS)?,
        block_watch_heading: read(KEY_BLOCK_WATCH)?,
        always_heading: read(KEY_ALWAYS_HEADING)?,
        always_lines: read(KEY_ALWAYS_LINES)?,
        accusation_header_template: read(KEY_ACCUSATION_HEADER)?,
        timeline_header_template: read(KEY_TIMELINE_HEADER)?,
        points_header_template: read(KEY_POINTS_HEADER)?,
        watch_header_template: read(KEY_WATCH_HEADER)?,
        what_gap: read(KEY_WHAT_GAP)?,
        accusation_text_gap: read(KEY_ACCUSATION_TEXT_GAP)?,
        no_instances_notice: read(KEY_NO_INSTANCES)?,
        gap_no_answer: read(KEY_GAP_NO_ANSWER)?,
        gap_accusation_removed: read(KEY_GAP_ACCUSATION_REMOVED)?,
        gap_answer_removed: read(KEY_GAP_ANSWER_REMOVED)?,
        gap_instance_unavailable: read(KEY_GAP_UNAVAILABLE)?,
        points_gap: read(KEY_POINTS_GAP)?,
        watch_gap: read(KEY_WATCH_GAP)?,
        instance_when_gap: read(KEY_INSTANCE_WHEN_GAP)?,
        instance_who_gap: read(KEY_INSTANCE_WHO_GAP)?,
        timeline_gap_template: read(KEY_TIMELINE_GAP)?,
        timeline_filter_prompt: read(KEY_TIMELINE_FILTER_PROMPT)?,
        timeline_filter_all_label: read(KEY_TIMELINE_FILTER_ALL)?,
        source_label_template: read(KEY_SOURCE_LABEL)?,
        source_open_label: read(KEY_SOURCE_OPEN)?,
        what_default_state: read(KEY_WHAT_STATE)?,
        accusation_default_state: read(KEY_ACCUSATION_STATE)?,
        timeline_default_state: read(KEY_TIMELINE_STATE)?,
        points_default_state: read(KEY_POINTS_STATE)?,
        watch_default_state: read(KEY_WATCH_STATE)?,
    })
}

// `pub(crate)` for the same reason `wording`'s test module is (task 2.11 B1):
// `settings_store_tests` needs this module's seeded fixture, and a second copy of
// thirty-nine sentences is a second thing to drift. The fixture lives next door
// with the test that pins it to the migration, so the copy and its proof are one
// file apart and never more.
#[cfg(test)]
#[path = "wording_rehearsal_tests.rs"]
pub(crate) mod tests;
