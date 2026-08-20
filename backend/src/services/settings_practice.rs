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

use crate::domain::practice_params::{
    PracticeReadParams, KEY_PRACTICE_CASE_TIMEZONE, KEY_PRACTICE_READ_FINE_TOKEN,
    KEY_PRACTICE_READ_MAX_POINTERS, KEY_PRACTICE_READ_MAX_TOKENS, KEY_PRACTICE_READ_MAX_WORDS,
    KEY_PRACTICE_READ_MAX_WORDS_AFTER_FINE, KEY_PRACTICE_READ_MAX_WORDS_CALL,
    KEY_PRACTICE_READ_MAX_WORDS_POINTER, KEY_PRACTICE_READ_MAX_WORDS_WHY, KEY_PRACTICE_READ_MODEL,
    KEY_PRACTICE_READ_PROMPT_FILE, KEY_PRACTICE_TACTIC_NAMES,
};
use crate::domain::settings::SettingError;
use crate::repositories::pipeline_repository::AppSettingRecord;
use crate::services::settings_row_readers::{token_count_of, token_list_of};
use crate::services::settings_store::{require, text_of};

/// Assemble the practice read's twelve parameters, or name the row that is wrong.
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
    })
}
