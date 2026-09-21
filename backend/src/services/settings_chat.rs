//! Reading the question chat's parameter block out of the settings store.
//!
//! Its own module for the reason `settings_practice` gives for being split from
//! `settings_store`: one more block would take the store past Rule 17. Nothing
//! here does I/O — it turns rows already read into the typed block, or names the
//! first key that is wrong.

use std::collections::HashMap;

use colossus_chat::CacheTtl;

use crate::domain::chat_params::{
    QuestionChatParams, COMPACTION_TRIGGER_MIN, KEY_CHAT_CASE_NARRATIVE_FILE,
    KEY_CHAT_WITNESS_DISPLAY_NAME, KEY_QUESTION_CHAT_ASKER_CROSS, KEY_QUESTION_CHAT_ASKER_DIRECT,
    KEY_QUESTION_CHAT_ASKER_REDIRECT, KEY_QUESTION_CHAT_CACHE_TTL,
    KEY_QUESTION_CHAT_CLIENT_IDLE_TIMEOUT_SECS, KEY_QUESTION_CHAT_COMPACTION_TRIGGER_TOKENS,
    KEY_QUESTION_CHAT_CONTEXT_HEADROOM_TOKENS, KEY_QUESTION_CHAT_EFFORT,
    KEY_QUESTION_CHAT_ERROR_PREVIEW_CHARS, KEY_QUESTION_CHAT_MAX_TOKENS,
    KEY_QUESTION_CHAT_MAX_TOOL_ROUNDS, KEY_QUESTION_CHAT_MAX_TURNS, KEY_QUESTION_CHAT_MODEL,
    KEY_QUESTION_CHAT_PROMPT_FILE,
};
use crate::domain::settings::SettingError;
use crate::repositories::pipeline_repository::AppSettingRecord;
use crate::services::settings_practice::effort_of;
use crate::services::settings_row_readers::token_count_of;
use crate::services::settings_store::{require, text_of};

/// Assemble the chat's parameter block, or name the row that is wrong.
///
/// # Errors
/// [`SettingError`] naming the first key that is missing, mis-kinded, blank, out of
/// bounds — or, for the two values with a vocabulary the store cannot express
/// (the TTL spelling, the compaction floor), outside it.
pub(crate) fn build_question_chat_params(
    rows: &HashMap<String, AppSettingRecord>,
) -> Result<QuestionChatParams, SettingError> {
    let count = |key: &str| -> Result<u32, SettingError> { token_count_of(require(rows, key)?) };
    let text = |key: &str| -> Result<String, SettingError> { text_of(require(rows, key)?) };
    Ok(QuestionChatParams {
        model: text(KEY_QUESTION_CHAT_MODEL)?,
        prompt_file: text(KEY_QUESTION_CHAT_PROMPT_FILE)?,
        narrative_file: text(KEY_CHAT_CASE_NARRATIVE_FILE)?,
        max_tokens: count(KEY_QUESTION_CHAT_MAX_TOKENS)?,
        effort: effort_of(rows, KEY_QUESTION_CHAT_EFFORT)?,
        max_tool_rounds: count(KEY_QUESTION_CHAT_MAX_TOOL_ROUNDS)?,
        context_headroom_tokens: count(KEY_QUESTION_CHAT_CONTEXT_HEADROOM_TOKENS)?,
        cache_ttl: ttl_of(&text(KEY_QUESTION_CHAT_CACHE_TTL)?)?,
        compaction_trigger_tokens: compaction_of(count(
            KEY_QUESTION_CHAT_COMPACTION_TRIGGER_TOKENS,
        )?)?,
        max_turns: count(KEY_QUESTION_CHAT_MAX_TURNS)?,
        client_idle_timeout_secs: count(KEY_QUESTION_CHAT_CLIENT_IDLE_TIMEOUT_SECS)?,
        witness_display_name: text(KEY_CHAT_WITNESS_DISPLAY_NAME)?,
        asker_cross: text(KEY_QUESTION_CHAT_ASKER_CROSS)?,
        asker_direct: text(KEY_QUESTION_CHAT_ASKER_DIRECT)?,
        asker_redirect: text(KEY_QUESTION_CHAT_ASKER_REDIRECT)?,
        error_preview_chars: count(KEY_QUESTION_CHAT_ERROR_PREVIEW_CHARS)?,
    })
}

/// The TTL row as the crate's enum. The crate owns the vocabulary (it is the API's).
fn ttl_of(raw: &str) -> Result<CacheTtl, SettingError> {
    CacheTtl::parse(raw).map_err(|_| SettingError::Unreadable {
        key: KEY_QUESTION_CHAT_CACHE_TTL.to_string(),
        value: raw.to_string(),
        expected: "5m or 1h",
    })
}

/// `0` means off; anything else must meet the API's floor.
///
/// ## Domain note: why 0 and not a separate on/off row
///
/// Two rows that must agree ("on" and "the trigger") are the coupled-row shape
/// the reviewer-bench deadlock taught us to avoid (Law 23(b)). One number, one
/// meaning per range, checked here.
fn compaction_of(stored: u32) -> Result<Option<u32>, SettingError> {
    match stored {
        0 => Ok(None),
        n if n < COMPACTION_TRIGGER_MIN => Err(SettingError::Unreadable {
            key: KEY_QUESTION_CHAT_COMPACTION_TRIGGER_TOKENS.to_string(),
            value: n.to_string(),
            expected: "0 (off) or at least 50000 — the API's minimum compaction trigger",
        }),
        n => Ok(Some(n)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_turns_compaction_off_and_a_small_trigger_is_refused() {
        assert_eq!(compaction_of(0), Ok(None));
        assert_eq!(compaction_of(700_000), Ok(Some(700_000)));
        assert!(compaction_of(49_999).is_err());
    }

    #[test]
    fn ttl_accepts_only_the_api_spellings() {
        assert_eq!(ttl_of("1h"), Ok(CacheTtl::OneHour));
        assert!(ttl_of("forever").is_err());
    }
}
