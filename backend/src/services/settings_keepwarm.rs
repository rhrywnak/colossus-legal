//! Reading the keep-warm block out of the settings store.
//!
//! Its own module for the reason `settings_chat` gives for being split from
//! `settings_store`: Rule 17. Nothing here does I/O — it turns rows already read
//! into [`KeepwarmParams`], or names the first key that is wrong. Because the
//! Settings page rebuilds the whole snapshot before a save is accepted, every
//! refusal below is ALSO a refusal of the edit that would have caused it.

use std::collections::HashMap;

use chrono::NaiveTime;

use crate::domain::keepwarm_params::{
    KeepwarmParams, ENABLED_OFF, ENABLED_ON, KEY_CHAT_KEEPWARM_DAILY_CAP_DOLLARS,
    KEY_CHAT_KEEPWARM_ENABLED, KEY_CHAT_KEEPWARM_INTERVAL_MINUTES, KEY_CHAT_KEEPWARM_WINDOW_END,
    KEY_CHAT_KEEPWARM_WINDOW_START,
};
use crate::domain::settings::SettingError;
use crate::repositories::pipeline_repository::AppSettingRecord;
use crate::services::settings_row_readers::{float_of, require, text_of, token_count_of};

// STRUCTURAL: the one clock spelling the window rows accept (24-hour `HH:MM`).
// A format, not a per-deployment value: the rows' help text names it.
const WINDOW_TIME_FORMAT: &str = "%H:%M";

/// Assemble the keep-warm block, or name the row that is wrong.
///
/// # Errors
/// [`SettingError`] naming the first key that is missing, mis-kinded, out of
/// bounds, not `on`/`off`, not an `HH:MM` time — or a window end that is not
/// after its start.
pub(crate) fn build_keepwarm_params(
    rows: &HashMap<String, AppSettingRecord>,
) -> Result<KeepwarmParams, SettingError> {
    let text = |key: &str| -> Result<String, SettingError> { text_of(require(rows, key)?) };
    let window_start = time_of(
        KEY_CHAT_KEEPWARM_WINDOW_START,
        &text(KEY_CHAT_KEEPWARM_WINDOW_START)?,
    )?;
    let window_end = time_of(
        KEY_CHAT_KEEPWARM_WINDOW_END,
        &text(KEY_CHAT_KEEPWARM_WINDOW_END)?,
    )?;
    if window_end <= window_start {
        return Err(SettingError::Unreadable {
            key: KEY_CHAT_KEEPWARM_WINDOW_END.to_string(),
            value: window_end.format(WINDOW_TIME_FORMAT).to_string(),
            expected: "a time after chat_keepwarm_window_start",
        });
    }
    Ok(KeepwarmParams {
        enabled: switch_of(&text(KEY_CHAT_KEEPWARM_ENABLED)?)?,
        window_start,
        window_end,
        interval_minutes: token_count_of(require(rows, KEY_CHAT_KEEPWARM_INTERVAL_MINUTES)?)?,
        daily_cap_dollars: f64::from(float_of(require(
            rows,
            KEY_CHAT_KEEPWARM_DAILY_CAP_DOLLARS,
        )?)?),
    })
}

/// `on` → true, `off` → false, anything else refused by name.
fn switch_of(raw: &str) -> Result<bool, SettingError> {
    match raw {
        ENABLED_ON => Ok(true),
        ENABLED_OFF => Ok(false),
        other => Err(SettingError::Unreadable {
            key: KEY_CHAT_KEEPWARM_ENABLED.to_string(),
            value: other.to_string(),
            expected: "on or off",
        }),
    }
}

/// A window row as a time of day.
///
/// ## Rust Learning: `NaiveTime::parse_from_str`
///
/// "Naive" in chrono means "with no timezone attached": `06:00` is a wall-clock
/// reading, and which instant it names depends on the zone and the date it is
/// read on — which is exactly right for a window that must mean 6 am in Detroit
/// on both sides of a daylight-saving change. The zone is applied later, per day.
fn time_of(key: &str, raw: &str) -> Result<NaiveTime, SettingError> {
    // `%H:%M` alone accepts `6:00`; requiring five characters keeps the one
    // spelling the help text promises, so `6:00pm` or `06:00:30` are refused.
    let parsed = (raw.len() == 5)
        // best-effort: chrono's parse error is replaced, not lost — the
        // `ok_or_else` below names the key, the value and the one spelling.
        .then(|| NaiveTime::parse_from_str(raw, WINDOW_TIME_FORMAT).ok())
        .flatten();
    parsed.ok_or_else(|| SettingError::Unreadable {
        key: key.to_string(),
        value: raw.to_string(),
        expected: "a 24-hour time written HH:MM, such as 06:00 or 23:00",
    })
}

#[cfg(test)]
#[path = "settings_keepwarm_tests.rs"]
mod tests;
