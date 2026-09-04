//! The settings store — one snapshot, one read path (task 1.6, v2 §2b).
//!
//! The configuration law says every tunable is "read from stored configuration at
//! runtime" and that "edits take effect on next read". This module is how both
//! are true at once without putting a database query on a hot path.
//!
//! ## The snapshot, and why it is THREADED rather than global
//!
//! Boot reads all seven parameters once and builds a [`Settings`] — a small,
//! immutable, `Copy`-cheap struct of already-parsed numbers. It lives in
//! `AppState`, and handlers hand `&Settings` to the pure functions that need it.
//!
//! The alternative was a process-global (`static SETTINGS: OnceLock<…>`) so the
//! existing seams could keep their signatures. It was rejected, and the deciding
//! argument is worth keeping: the seams are called from functions this codebase
//! documents as PURE — `build_card` says "Pure — no I/O" and the §7 completeness
//! test asserts against it. A global read inside a pure function is hidden state,
//! and worse, a unit test calling that function without booting would find the
//! global empty, leaving only two ways out: panic (forbidden), or fall back to a
//! compiled-in default — *the exact defect this task exists to delete*. Threading
//! one parameter has no such corner. "Pure — takes one more input" is still pure.
//!
//! ## The freshness law
//!
//! A write updates the row, appends the ledger entry, and re-reads the whole
//! store into a new snapshot, all before the response is sent. The next read of
//! `AppState` therefore sees the new value: "edits take effect on next read",
//! literally, with zero database reads on the card path.
//!
//! **This assumes a single-process backend.** One process holds one snapshot, so
//! the swap is total. If a second process ever serves this API, the swap needs a
//! cross-process story — a notification channel, or a short TTL re-read. Not
//! today's problem; recorded so tomorrow's reader knows it was seen and not
//! missed.
//!
//! ## The failure law
//!
//! A parameter missing, unreadable, out of bounds, or self-contradictory is a
//! REFUSAL, at boot and on write. There is deliberately no fallback: after this
//! task no compiled-in default exists to fall back to, and inventing one at the
//! moment of failure would silently reinstate the defect §2b bans.

use std::collections::HashMap;

// The write path's imports — the pool, the clock, the change ledger, the
// template check and the candidate parsers — left with it in the 2026-08-25
// split. What this file still needs is the row shape, the snapshot type and the
// readers that turn one stored string into a number.
use crate::domain::evidence_tier::{EvidenceTier, EvidenceTierMap};
use crate::domain::settings::{SettingError, Settings};
use crate::repositories::pipeline_repository::{AppSettingRecord, PipelineRepoError};

// KEYS: the stable identifiers of the seven stored parameters. These are not
// tunables — they are the NAMES of tunables, the join key between this code and
// the `app_settings` rows the migration seeded, in the same category as a column
// name or a serde tag. Renaming one is a migration, and until that migration runs
// the boot loader refuses to start rather than guessing.
const KEY_BAND_HIGH: &str = "confidence_band_high";
const KEY_BAND_MEDIUM: &str = "confidence_band_medium";
const KEY_CONTEXT_WINDOW: &str = "quote_context_window_chars";
const KEY_TALKING_POINTS_CAP: &str = "talking_points_cap";
const KEY_READINESS_N: &str = "readiness_item_threshold_n";
const KEY_CARD_TEST_RATIO: &str = "card_test_ratio";
const KEY_REANCHOR_TOLERANCE: &str = "reanchor_close_match_tolerance";
const KEY_LINK_SHORT_LIST_MAX: &str = "link_short_list_max";
const KEY_CHRONOLOGY_PHASE_WINDOW: &str = "chronology_phase_window_events";
const KEY_CHRONOLOGY_PICKER_MAX: &str = "chronology_document_picker_max";
const KEY_TIMELINE_MIN_DATES: &str = "rehearsal_timeline_min_distinct_dates";
const KEY_ROWS_EXPAND_MAX: &str = "rehearsal_instance_rows_expand_max";
// Task 2.15 Tier 2. The first is TEXT but is NOT wording — it names a file, not a
// sentence a human reads — so it belongs in this list rather than in a wording
// key list, and it is the reason the doc below says "not-wording" rather than
// "numeric".
pub(crate) const KEY_THEME_SCAN_PROMPT_FILE: &str = "theme_scan_prompt_file";
const KEY_PREFILTER_MIN_CHARS: &str = "theme_scan_prefilter_min_chars";
// The judge's per-candidate token budget (2026-08-09). A row rather than a
// constant because the value that is right for a model depends on THE MODEL, and
// the compiled-in 512 was measured killing 7 of 104 verdicts — see the deleted
// `THEME_SCAN_MAX_TOKENS` comment in `services::theme_scan` for the full story.
const KEY_SCAN_MAX_TOKENS: &str = "theme_scan_max_tokens";
const KEY_SCAN_DEFAULT_MODEL: &str = "theme_scan_default_model";
// L2b (2026-09-01). Which parties a ranked gather may reach — strict, widened
// or off. A row rather than a constant because when a card is missing from a
// gather the first question is "filter problem or ranking problem?", and only a
// human who can flip this to `off` and look can answer it. The vocabulary is
// validated in `domain::gather_filter`, so an illegal value is a boot refusal.
pub(crate) const KEY_GATHER_SUBJECT_FILTER: &str = "gather_subject_filter";
// L2b, after review (2026-09-01). How deep each half of a ranked gather reads
// before fusion. It shipped as a compiled 200 and was flagged: a retrieval
// limit is per-deployment by Rule 13's own list, and no STRUCTURAL claim about
// it would have been true — L3 exists partly to find out whether 200 is right.
pub(crate) const KEY_GATHER_READ_DEPTH: &str = "gather_read_depth";
// L2b probe selectivity (2026-09-01). The share of a scenario's admitted set
// above which a trigram probe is dropped before it is read. Measured cause:
// `Court` matched 534 of S-11's 1030 admitted cards and agreed with everything.
pub(crate) const KEY_GATHER_PROBE_MAX_SHARE: &str = "gather_probe_max_share";
// The companion floor: how many probes survive when every one of them is over
// the share. A row and not a constant for the reason Rule 13 gives — the guard
// ("never zero") is an invariant, but the NUMBER above zero is a judgement with
// no external anchor.
pub(crate) const KEY_GATHER_PROBE_FLOOR: &str = "gather_probe_floor";
// ONE_CARD_GRAMMAR (2026-08-09). Both decide how much of a card's content is
// SHOWN before it folds — the question's visible length, and how many element
// chips stand before "+N more". They are §2b tunables rather than presentational
// constants because they change what a human can read without a click, which on
// the 13-inch hardware law is the difference between a rulable card and a wall.
const KEY_CARD_QUESTION_TRUNCATE: &str = "card_question_truncate_chars";
const KEY_CARD_ELEMENT_CHIPS_K: &str = "card_element_chips_visible_k";
const KEY_PREFILTER_STATEMENT_TYPES: &str = "theme_scan_prefilter_statement_types";
// Task 396 P1. Three TEXT rows that are NOT wording — they name extraction
// vocabulary, not sentences anybody reads — so they belong in this list beside
// `theme_scan_prefilter_statement_types`, which is the same shape for the same
// reason. Together they are the `(statement_type, evidence_strength)` → tier map
// the Proof Matrix's headline number is computed from.
const KEY_TIER_STRONG_PAIRS: &str = "matrix_tier_strong_pairs";
const KEY_TIER_HEDGED_PAIRS: &str = "matrix_tier_hedged_pairs";
const KEY_TIER_OTHER_PAIRS: &str = "matrix_tier_other_pairs";
// PRACTICE v0 (2026-08-17). Seven rows that are TEXT-or-number but NOT wording:
// they decide what the one-sentence read is TOLD, by WHICH model, and what shape
// of reply reaches a witness. Judgment parameters in exactly the sense
// `theme_scan_prompt_file` is; the words Marie READS live in the two
// `PRACTICE_*_WORDING_KEYS` lists. Their keys live beside the block they build.

/// Every NOT-WORDING key this build reads, so a missing one is caught at boot by
/// name.
///
/// The anti-drift half of the configuration law: the store may hold parameters
/// this build does not know about (a future task's, seeded early), but this build
/// must never START without every parameter it does know about.
///
/// The stored SENTENCES live in the `*_WORDING_KEYS` lists rather than here. Two
/// kinds of list rather than one flat one because they answer different questions
/// — these decide how the system judges, those are the words it speaks — and
/// because a boot log reporting both counts says more than one number does.
///
/// `theme_scan_prompt_file` is text and still belongs HERE: it names a file the
/// scan reads, not a sentence anybody reads, and it decides what the judge is
/// told — which is a judgment parameter in every sense that matters.
///
/// PRACTICE v0's seven are the first NOT-WORDING keys to live in a list of their
/// own (`domain::practice_params::PRACTICE_PARAM_KEYS`) rather than here — see
/// that list's note. `settings_boot` consults both.
pub const REQUIRED_KEYS: &[&str] = &[
    KEY_BAND_HIGH,
    KEY_BAND_MEDIUM,
    KEY_CONTEXT_WINDOW,
    KEY_TALKING_POINTS_CAP,
    KEY_READINESS_N,
    KEY_CARD_TEST_RATIO,
    KEY_REANCHOR_TOLERANCE,
    KEY_LINK_SHORT_LIST_MAX,
    KEY_CHRONOLOGY_PHASE_WINDOW,
    KEY_CHRONOLOGY_PICKER_MAX,
    KEY_TIMELINE_MIN_DATES,
    KEY_ROWS_EXPAND_MAX,
    KEY_GATHER_PROBE_FLOOR,
    KEY_GATHER_PROBE_MAX_SHARE,
    KEY_GATHER_READ_DEPTH,
    KEY_GATHER_SUBJECT_FILTER,
    KEY_THEME_SCAN_PROMPT_FILE,
    KEY_PREFILTER_MIN_CHARS,
    KEY_SCAN_MAX_TOKENS,
    KEY_PREFILTER_STATEMENT_TYPES,
    KEY_SCAN_DEFAULT_MODEL,
    KEY_CARD_QUESTION_TRUNCATE,
    KEY_CARD_ELEMENT_CHIPS_K,
    KEY_TIER_STRONG_PAIRS,
    KEY_TIER_HEDGED_PAIRS,
    KEY_TIER_OTHER_PAIRS,
];

/// Why the store could not be read or written.
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    /// A stored parameter is missing, unreadable, or out of bounds.
    ///
    /// Carries the domain error verbatim: it already names the key, the value and
    /// the expectation, and rewrapping it in vaguer words would lose exactly the
    /// detail a human needs to fix the row.
    #[error("{source}")]
    Invalid {
        #[source]
        source: SettingError,
    },

    #[error("no parameter named '{key}' can be changed — this build stores no such key")]
    UnknownKey { key: String },

    /// A row that NAMES A FILE was set to one that does not resolve.
    ///
    /// ## Why this is refused at the page rather than discovered at the next scan
    ///
    /// `theme_scan_prompt_file` decides what the judge is told. A typo passes every
    /// other check a `text` row has (it is non-blank and carries no placeholders),
    /// so without this the change would commit, the next scan would fail with a
    /// missing-path error, and the next RESTART would refuse to boot — hours after
    /// the edit, on a screen that said "saved". Naming the path here is what lets
    /// the human fix it in the moment.
    #[error(
        "{key} names '{value}', and no such file is deployed at '{path}' — \
         deploy the file to the template directory first, or correct the name"
    )]
    FileNotFound {
        key: String,
        value: String,
        path: String,
    },

    #[error("{key} is already '{value}' — nothing to change")]
    Unchanged { key: String, value: String },

    #[error("failed to read the configuration store: {source}")]
    Read {
        #[source]
        source: PipelineRepoError,
    },

    /// The change WAS saved, but the in-memory snapshot could not be refreshed.
    ///
    /// ## Why this is its own variant and not a `Read`
    ///
    /// It was a `Read` at first, which made it indistinguishable from a failure
    /// BEFORE anything was written — both reached the operator as "failed to
    /// load the settings". Those are opposite states: in one, nothing happened
    /// and the fix is to retry; in the other the value is stored and a retry
    /// returns "already {value} — nothing to change", which reads as a
    /// contradiction of the error they just saw.
    ///
    /// It also has a different remedy. The stored value is correct and the page
    /// will show it, but the RUNNING process is serving the old snapshot until it
    /// is restarted or the next successful write swaps it — so the message says
    /// restart, not retry.
    #[error(
        "{key} was saved as '{value}', but the running configuration could not be          refreshed: {source}. The stored value is correct — reload this page to          confirm it. The service keeps using the previous value until it is          restarted."
    )]
    SavedButStale {
        key: String,
        value: String,
        #[source]
        source: Box<SettingsError>,
    },

    #[error("failed to record the configuration change: {source}")]
    Write {
        #[source]
        source: PipelineRepoError,
    },
}

impl From<SettingError> for SettingsError {
    /// ## Rust Learning: `From` for error conversion
    ///
    /// Implementing `From<SettingError>` is what lets `?` promote a parse failure
    /// into a `SettingsError` with no `map_err` at the call site. The trait pair
    /// `From`/`Into` is Rust's standard conversion idiom: implement `From`, get
    /// `Into` for free, and `?` uses it automatically.
    fn from(source: SettingError) -> Self {
        SettingsError::Invalid { source }
    }
}

// The per-row readers moved to the sibling `settings_row_readers` when this
// module reached the 300-line limit (2026-08-09). They answer "what does this ONE
// row say?"; what stays here answers "does the whole store make a usable
// snapshot?" — which is why the cross-row band invariant is below and not there.
use super::settings_row_readers::{
    count_of, float_of, gather_filter_of, ratio_of, token_count_of, token_list_of,
};
// Re-exported, not re-implemented: `settings_wording` imports both from THIS
// module's path, and the split is an internal reorganisation that has no business
// changing a sibling's import line.
pub(crate) use super::settings_row_readers::{require, text_of};

/// Build a [`Settings`] from the stored rows, or say precisely what is wrong.
///
/// Pure — takes rows, returns parsed values — so every refusal branch is
/// unit-testable without a database, which matters because these branches are the
/// configuration law's teeth.
///
/// # Errors
/// Returns [`SettingError`] naming the first parameter that is missing,
/// unreadable, out of bounds, or contradicts another.
pub fn build_settings(rows: &HashMap<String, AppSettingRecord>) -> Result<Settings, SettingError> {
    let confidence_band_high = float_of(require(rows, KEY_BAND_HIGH)?)?;
    let confidence_band_medium = float_of(require(rows, KEY_BAND_MEDIUM)?)?;

    // The one invariant that spans two rows, and therefore the one a column CHECK
    // cannot express. Checked HERE as well as in the write path because a `psql`
    // edit bypasses the write path entirely — and a store where high <= medium
    // makes the medium band unreachable, which would look like a banding bug
    // rather than a configuration mistake.
    if confidence_band_high <= confidence_band_medium {
        return Err(SettingError::BandsCrossed {
            high: confidence_band_high,
            medium: confidence_band_medium,
        });
    }

    let words = crate::services::settings_wording::build_all_wording(rows)?;
    let evidence_tier_map = build_evidence_tier_map(rows)?;

    Ok(Settings {
        confidence_band_high,
        confidence_band_medium,
        quote_context_window_chars: count_of(require(rows, KEY_CONTEXT_WINDOW)?)?,
        talking_points_cap: count_of(require(rows, KEY_TALKING_POINTS_CAP)?)?,
        readiness_item_threshold_n: count_of(require(rows, KEY_READINESS_N)?)?,
        card_test_ratio: ratio_of(require(rows, KEY_CARD_TEST_RATIO)?)?,
        reanchor_close_match_tolerance: float_of(require(rows, KEY_REANCHOR_TOLERANCE)?)?,
        link_short_list_max: count_of(require(rows, KEY_LINK_SHORT_LIST_MAX)?)?,
        wording: words.curation,
        accusation_wording: words.accusation,
        rehearsal_wording: words.rehearsal,
        rehearsal_timeline_min_distinct_dates: count_of(require(rows, KEY_TIMELINE_MIN_DATES)?)?,
        rehearsal_chrome_wording: words.chrome,
        authoring_wording: words.authoring,
        scenario_authoring_wording: words.scenario_authoring,
        gather_probe_floor: count_of(require(rows, KEY_GATHER_PROBE_FLOOR)?)?,
        gather_probe_max_share: ratio_of(require(rows, KEY_GATHER_PROBE_MAX_SHARE)?)?,
        gather_read_depth: count_of(require(rows, KEY_GATHER_READ_DEPTH)?)?,
        gather_subject_filter: gather_filter_of(require(rows, KEY_GATHER_SUBJECT_FILTER)?)?,
        theme_scan_prompt_file: text_of(require(rows, KEY_THEME_SCAN_PROMPT_FILE)?)?,
        theme_scan_prefilter_min_chars: count_of(require(rows, KEY_PREFILTER_MIN_CHARS)?)?,
        theme_scan_max_tokens: token_count_of(require(rows, KEY_SCAN_MAX_TOKENS)?)?,
        theme_scan_default_model: text_of(require(rows, KEY_SCAN_DEFAULT_MODEL)?)?,
        theme_scan_prefilter_statement_types: token_list_of(require(
            rows,
            KEY_PREFILTER_STATEMENT_TYPES,
        )?)?,
        scan_wording: words.scan,
        rehearsal_instance_rows_expand_max: count_of(require(rows, KEY_ROWS_EXPAND_MAX)?)?,
        card_grammar_wording: words.card_grammar,
        model_params_wording: words.model_params,
        card_question_truncate_chars: count_of(require(rows, KEY_CARD_QUESTION_TRUNCATE)?)?,
        card_element_chips_visible_k: count_of(require(rows, KEY_CARD_ELEMENT_CHIPS_K)?)?,
        chronology_wording: words.chronology,
        chronology_phase_window_events: count_of(require(rows, KEY_CHRONOLOGY_PHASE_WINDOW)?)?,
        chronology_document_picker_max: count_of(require(rows, KEY_CHRONOLOGY_PICKER_MAX)?)?,
        matrix_wording: words.matrix,
        war_room_wording: words.war_room,
        practice_wording: words.practice,
        practice_report_wording: words.practice_report,
        practice_read: crate::services::settings_practice::build_practice_read_params(rows)?,
        evidence_tier_map,
    })
}

/// Assemble the `(statement_type, evidence_strength)` → tier map from its three
/// rows, or name the row and the entry that is malformed.
///
/// ## Why the pair rows are parsed HERE and not inside `EvidenceTierMap`
///
/// Same seam every other row obeys: the STORE owns what a row is (declared kind,
/// non-blank, comma-separated tokens — `token_list_of`), and the DOMAIN owns what
/// the tokens MEAN. So this function does the store half for three rows and hands
/// the already-split entries to `EvidenceTierMap::from_entries`, which knows
/// nothing about databases. It is the same division `build_all_wording` makes.
///
/// ## Rust Learning: converting a foreign error with `map_err`
///
/// `EvidenceTierMap::from_entries` returns its own `PairParseError` — it cannot
/// return a `SettingError` without depending on the settings vocabulary. The
/// conversion happens here, at the one boundary that knows both, and it preserves
/// the row key and the offending entry so the boot refusal still names them.
///
/// # Errors
/// Returns [`SettingError`] if any of the three rows is missing, is not declared
/// `text`, is blank, or holds an entry that is not
/// `statement_type+evidence_strength`.
fn build_evidence_tier_map(
    rows: &HashMap<String, AppSettingRecord>,
) -> Result<EvidenceTierMap, SettingError> {
    let strong = token_list_of(require(rows, KEY_TIER_STRONG_PAIRS)?)?;
    let hedged = token_list_of(require(rows, KEY_TIER_HEDGED_PAIRS)?)?;
    let other = token_list_of(require(rows, KEY_TIER_OTHER_PAIRS)?)?;

    EvidenceTierMap::from_entries(&[
        (EvidenceTier::Strong, KEY_TIER_STRONG_PAIRS, &strong),
        (EvidenceTier::Hedged, KEY_TIER_HEDGED_PAIRS, &hedged),
        (EvidenceTier::Other, KEY_TIER_OTHER_PAIRS, &other),
    ])
    .map_err(|source| SettingError::Unreadable {
        key: source.key,
        value: source.entry,
        expected: "statement_type+evidence_strength, both halves non-blank",
    })
}

/// Index rows by key.
pub(crate) fn by_key(rows: Vec<AppSettingRecord>) -> HashMap<String, AppSettingRecord> {
    rows.into_iter().map(|r| (r.key.clone(), r)).collect()
}

#[cfg(test)]
#[path = "settings_store_tests.rs"]
mod tests;
