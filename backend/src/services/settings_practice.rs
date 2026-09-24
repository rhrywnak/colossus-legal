//! Reading the practice read's parameter block out of the settings store.
//!
//! Split from `settings_store` on 2026-08-17 for the reason `settings_wording`
//! was split from it on 2026-08-07: adding a block took that module past the
//! 300-line limit (Rule 17). The seam is the one those two modules already draw
//! — `build_settings` decides the numbers this system JUDGES by, `settings_wording`
//! the words it SPEAKS, and this the terms on which one witness surface asks a
//! model a question.
//!
//! Nothing here does I/O. It takes the rows the store has already read and turns
//! them into the typed block, or names the first key that is wrong.

use std::collections::HashMap;

use crate::domain::llm_effort::parse_stored_effort;
use crate::domain::practice_params::{
    PracticeReadParams, KEY_PRACTICE_CASE_TIMEZONE, KEY_PRACTICE_FOR_YOU_DECK_THRESHOLD,
    KEY_PRACTICE_READ_EFFORT, KEY_PRACTICE_READ_FINE_TOKEN, KEY_PRACTICE_READ_MAX_POINTERS,
    KEY_PRACTICE_READ_MAX_TOKENS, KEY_PRACTICE_READ_MAX_WORDS,
    KEY_PRACTICE_READ_MAX_WORDS_AFTER_FINE, KEY_PRACTICE_READ_MAX_WORDS_CALL,
    KEY_PRACTICE_READ_MAX_WORDS_POINTER, KEY_PRACTICE_READ_MAX_WORDS_WHY, KEY_PRACTICE_READ_MODEL,
    KEY_PRACTICE_READ_PROMPT_FILE, KEY_PRACTICE_REVIEWER_DISPLAY_NAMES,
    KEY_PRACTICE_REVIEWER_USERNAMES, KEY_PRACTICE_TACTIC_NAMES, KEY_PRACTICE_WITNESS_USERNAME,
};
use crate::domain::settings::SettingError;
use crate::repositories::pipeline_repository::AppSettingRecord;
use crate::services::settings_row_readers::{token_count_of, token_list_of, verbatim_list_of};
use crate::services::settings_store::{require, text_of};

/// Assemble the practice read's parameter block, or name the row that is wrong.
///
/// Same seam every other block obeys: the STORE owns what a row is (declared
/// kind, non-blank, comma-separated tokens) and the DOMAIN owns what the values
/// MEAN. Three different readers are needed here — text, count and list — which
/// is why this is a function rather than one of the single-closure wording
/// builders.
///
/// # Errors
/// [`SettingError`] naming the first key that is missing, of the wrong declared
/// kind, blank, or out of bounds.
pub(crate) fn build_practice_read_params(
    rows: &HashMap<String, AppSettingRecord>,
) -> Result<PracticeReadParams, SettingError> {
    let bench = reviewer_bench(rows)?;
    Ok(PracticeReadParams {
        prompt_file: text_of(require(rows, KEY_PRACTICE_READ_PROMPT_FILE)?)?,
        model: text_of(require(rows, KEY_PRACTICE_READ_MODEL)?)?,
        max_tokens: token_count_of(require(rows, KEY_PRACTICE_READ_MAX_TOKENS)?)?,
        max_words: token_count_of(require(rows, KEY_PRACTICE_READ_MAX_WORDS)?)?,
        max_words_after_fine: token_count_of(require(
            rows,
            KEY_PRACTICE_READ_MAX_WORDS_AFTER_FINE,
        )?)?,
        max_words_call: token_count_of(require(rows, KEY_PRACTICE_READ_MAX_WORDS_CALL)?)?,
        max_words_why: token_count_of(require(rows, KEY_PRACTICE_READ_MAX_WORDS_WHY)?)?,
        max_words_pointer: token_count_of(require(rows, KEY_PRACTICE_READ_MAX_WORDS_POINTER)?)?,
        max_pointers: token_count_of(require(rows, KEY_PRACTICE_READ_MAX_POINTERS)?)?,
        fine_token: text_of(require(rows, KEY_PRACTICE_READ_FINE_TOKEN)?)?,
        tactic_names: token_list_of(require(rows, KEY_PRACTICE_TACTIC_NAMES)?)?,
        case_timezone: text_of(require(rows, KEY_PRACTICE_CASE_TIMEZONE)?)?,
        reviewer_usernames: bench.logins,
        reviewer_display_names: bench.names,
        witness_username: text_of(require(rows, KEY_PRACTICE_WITNESS_USERNAME)?)?,
        // `token_count_of` checks the declared kind and the row's own
        // min/max — the migration seeds 1..=100, so a Settings edit to 0
        // is refused at the store rather than collapsing the page here.
        for_you_deck_threshold: token_count_of(require(
            rows,
            KEY_PRACTICE_FOR_YOU_DECK_THRESHOLD,
        )?)?,
        effort: effort_of(rows, KEY_PRACTICE_READ_EFFORT)?,
    })
}

/// The two reviewer lists, read together and checked against each other.
///
/// ## Why they are read as a PAIR and not as two independent rows
///
/// They are index-aligned: the third login's name is the third name. Read
/// separately, a store whose two rows have drifted apart produces a perfectly
/// valid snapshot that prints the wrong attorney's name beside the review
/// queue — the one failure here that looks like working software, because
/// nothing downstream can tell a wrong name from a right one. Read as a pair,
/// the drift is a boot refusal naming both lengths.
///
/// ## Rust Learning: a private struct to name a two-value return
///
/// `Result<(Vec<String>, Vec<String>), _>` would compile and would be one
/// transposition away from swapping logins for names — both are `Vec<String>`,
/// so the compiler would say nothing. Two named fields cost three lines and
/// remove the whole class of mistake, which is the same trade
/// `settings_wording::AllWording` makes for its fifteen blocks.
struct ReviewerBench {
    logins: Vec<String>,
    names: Vec<String>,
}

/// # Errors
/// [`SettingError`] naming the row that is missing, blank or malformed, or — when
/// both rows read cleanly — an [`SettingError::Unreadable`] on the login row
/// reporting that the two lists are not the same length.
fn reviewer_bench(rows: &HashMap<String, AppSettingRecord>) -> Result<ReviewerBench, SettingError> {
    // Case PRESERVED on both: the logins are identities compared exactly, and
    // the names are printed on a screen. The lower-casing reader beside this one
    // would turn `Chuck` into `chuck` in the review bar.
    let logins = verbatim_list_of(require(rows, KEY_PRACTICE_REVIEWER_USERNAMES)?)?;
    let names = verbatim_list_of(require(rows, KEY_PRACTICE_REVIEWER_DISPLAY_NAMES)?)?;
    if logins.len() != names.len() {
        tracing::error!(
            logins = logins.len(),
            names = names.len(),
            "the reviewer bench is misaligned: every login needs a display name in the same position"
        );
        // The refusal is reported against the LOGIN row because that is the list
        // an operator edits first when adding a reviewer, and the name row is
        // the one they then forget. `expected` is `&'static str`, so the two
        // lengths go to the log above rather than into the stored error.
        return Err(SettingError::Unreadable {
            key: KEY_PRACTICE_REVIEWER_USERNAMES.to_string(),
            value: logins.join(","),
            expected: "the same number of entries as practice_reviewer_display_names",
        });
    }
    Ok(ReviewerBench { logins, names })
}

/// One stored effort row as the wire value it means, or a named refusal.
///
/// ## Why the error names the key
///
/// A row reading `thorough` is a typo an operator made on the Settings page, and
/// the only useful thing to say is which row and what the words are. The refusal
/// stops the snapshot, exactly as a malformed number does — a bad effort reaches
/// the API as an HTTP 400 in the middle of a paid call otherwise.
pub(super) fn effort_of(
    rows: &HashMap<String, AppSettingRecord>,
    key: &str,
) -> Result<Option<crate::domain::llm_effort::Effort>, SettingError> {
    let raw = text_of(require(rows, key)?)?;
    // The parser's own sentence is logged; the STORED error carries a static
    // expectation because `SettingError::Unreadable` holds `&'static str` — and
    // leaking a boxed string per bad read would be a memory leak on a typo.
    parse_stored_effort(&raw).map_err(|detail| {
        tracing::error!(key, value = %raw, detail = %detail, "a stored effort is not a documented level");
        SettingError::Unreadable {
            key: key.to_string(),
            value: raw.clone(),
            expected: "one of low, medium, high, xhigh, max, or absent",
        }
    })
}
