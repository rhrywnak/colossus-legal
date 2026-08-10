//! Reading ONE stored row as a typed value — the kind check, the bounds, and the
//! narrowing.
//!
//! Split from [`super::settings_store`] on 2026-08-09, when the judge's token cap
//! became a row and pushed that module past the 300-line limit. The boundary is a
//! real one and not just a size cut: everything here answers "what does this ONE
//! row say?", while the parent answers "does the whole store make a usable
//! snapshot?" — the cross-row invariants (the confidence bands' ordering) and the
//! key lists stay there.
//!
//! Every function is `pub(super)` or narrower: these are the parent's internals,
//! and a caller reaching past `build_settings` to parse a row by hand would be
//! bypassing the boot assertion that gives the configuration law its teeth.

use std::collections::HashMap;

use crate::domain::settings::{
    parse_count, parse_float, parse_ratio, parse_text, parse_token_list, Bounds, Ratio,
    SettingError, ValueKind,
};
use crate::repositories::pipeline_repository::AppSettingRecord;

/// Bounds as declared on one row.
pub(super) fn bounds_of(record: &AppSettingRecord) -> Bounds {
    Bounds {
        min: record.min_value,
        max: record.max_value,
    }
}

/// Find a required row, or report it missing by name.
pub(crate) fn require<'a>(
    rows: &'a HashMap<String, AppSettingRecord>,
    key: &str,
) -> Result<&'a AppSettingRecord, SettingError> {
    rows.get(key).ok_or_else(|| SettingError::Missing {
        key: key.to_string(),
    })
}

/// Read one row as a float, checking that its declared kind agrees.
///
/// The kind check is not redundant with the parse: a row whose `value_kind` says
/// `ratio` while this build reads it as a float is a store that has drifted from
/// the code, and it must be reported as that rather than as a parse failure that
/// sends a human hunting through the value.
pub(super) fn float_of(record: &AppSettingRecord) -> Result<f32, SettingError> {
    expect_kind(record, ValueKind::Float)?;
    parse_float(&record.key, &record.value, bounds_of(record))
}

/// Read one row as a count, checking its declared kind.
pub(super) fn count_of(record: &AppSettingRecord) -> Result<usize, SettingError> {
    expect_kind(record, ValueKind::Count)?;
    parse_count(&record.key, &record.value, bounds_of(record))
}

/// Read one `count` row as a TOKEN budget — a `u32`, the width the LLM seam speaks.
///
/// ## Rust Learning: `try_from` instead of `as`
///
/// `count_of` yields a `usize`, which is 64 bits on this hardware, and every
/// `max_tokens` on the provider seam is a `u32`. Written `value as u32`, a stored
/// 5_000_000_000 would arrive at the wire as 705_032_704 — a different, plausible,
/// silently wrong budget. `u32::try_from` returns a `Result` instead, so the
/// impossible value is a named refusal at BOOT rather than a strange scan later.
///
/// The row's own `max_value` (64000) makes the overflow unreachable in practice.
/// It is handled anyway because the bound is DATA — a `psql` edit or a future
/// migration can widen it — while this cast is code, and Standing Rule 1 does not
/// have an "unreachable in practice" clause.
///
/// Reported as [`SettingError::AboveMaximum`] rather than a new variant: a number
/// too large for the field it feeds is exactly what that error says, and the
/// operator's remedy (type a smaller number) is the same one.
///
/// The `max` it names is `u32::MAX`, not the row's own `max_value` — deliberately.
/// By the time this line runs the row's ceiling has already been checked and
/// passed (that is what `count_of` did), so the limit actually being hit is the
/// width of the field, and naming the row's ceiling would point the operator at a
/// bound that is not the one refusing them.
pub(super) fn token_count_of(record: &AppSettingRecord) -> Result<u32, SettingError> {
    let count = count_of(record)?;
    u32::try_from(count).map_err(|_| SettingError::AboveMaximum {
        key: record.key.clone(),
        value: record.value.clone(),
        max: f64::from(u32::MAX),
    })
}

/// Read one row as a ratio, checking its declared kind.
pub(super) fn ratio_of(record: &AppSettingRecord) -> Result<Ratio, SettingError> {
    expect_kind(record, ValueKind::Ratio)?;
    parse_ratio(&record.key, &record.value)
}

/// Read one row as stored text, checking its declared kind (task 2.10).
pub(crate) fn text_of(record: &AppSettingRecord) -> Result<String, SettingError> {
    expect_kind(record, ValueKind::Text)?;
    parse_text(&record.key, &record.value)
}

/// Read one `text` row as a comma-separated list of tokens (task 2.15 Tier 2).
///
/// The declared kind is still `text` — the store has no list kind, and inventing
/// one would be a migration for a single row. What makes the list a list is the
/// PARSE, which is where every other stored shape is decided too.
pub(super) fn token_list_of(record: &AppSettingRecord) -> Result<Vec<String>, SettingError> {
    expect_kind(record, ValueKind::Text)?;
    parse_token_list(&record.key, &record.value)
}

/// Confirm a row's stored kind is the one this build expects to read.
pub(super) fn expect_kind(
    record: &AppSettingRecord,
    expected: ValueKind,
) -> Result<(), SettingError> {
    let stored = ValueKind::try_from(record.value_kind.as_str())?;
    if stored != expected {
        return Err(SettingError::Unreadable {
            key: record.key.clone(),
            value: record.value.clone(),
            expected: match expected {
                ValueKind::Float => "a number (the store declares a different kind)",
                ValueKind::Count => "a whole number (the store declares a different kind)",
                ValueKind::Ratio => "a ratio (the store declares a different kind)",
                ValueKind::Text => "words (the store declares a different kind)",
            },
        });
    }
    Ok(())
}
