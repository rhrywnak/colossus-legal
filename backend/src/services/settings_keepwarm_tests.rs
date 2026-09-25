//! The keep-warm reader: the seeded rows build the fixture, and every bad value
//! is refused naming its key.

use super::*;
use chrono::Utc;

/// The seeded rows as the store holds them, with `edits` applied over them.
fn rows_with(edits: &[(&str, &str)]) -> HashMap<String, AppSettingRecord> {
    KeepwarmParams::for_test_rows()
        .into_iter()
        .map(|(key, kind, value, min, max)| {
            let value = edits
                .iter()
                .find(|(k, _)| *k == key)
                .map_or(value, |(_, v)| *v);
            let record = AppSettingRecord {
                key: key.to_string(),
                value: value.to_string(),
                value_kind: kind.to_string(),
                default_value: value.to_string(),
                min_value: min,
                max_value: max,
                meaning: "a parameter".to_string(),
                consumed_by: None,
                updated_at: Utc::now(),
                updated_by: "test".to_string(),
            };
            (key.to_string(), record)
        })
        .collect()
}

/// The key a refusal names.
fn refused_key(edits: &[(&str, &str)]) -> String {
    match build_keepwarm_params(&rows_with(edits)) {
        Ok(p) => panic!("{edits:?} was accepted as {p:?}"),
        Err(
            SettingError::Unreadable { key, .. }
            | SettingError::BelowMinimum { key, .. }
            | SettingError::AboveMaximum { key, .. },
        ) => key,
        Err(other) => panic!("{edits:?} refused with an unexpected error: {other}"),
    }
}

#[test]
fn the_seeded_rows_build_the_fixture() {
    assert_eq!(
        build_keepwarm_params(&rows_with(&[])).expect("the seed is valid"),
        KeepwarmParams::for_test()
    );
}

#[test]
fn off_reads_as_off() {
    let p = build_keepwarm_params(&rows_with(&[(KEY_CHAT_KEEPWARM_ENABLED, "off")]))
        .expect("off is valid");
    assert!(!p.enabled);
}

#[test]
fn only_on_and_off_are_accepted() {
    // Stored text is trimmed on read (`parse_text`), so `"on "` IS `on`; case is
    // not folded, so `On` is refused rather than guessed at.
    for bad in ["yes", "On", "true", "1"] {
        assert_eq!(
            refused_key(&[(KEY_CHAT_KEEPWARM_ENABLED, bad)]),
            KEY_CHAT_KEEPWARM_ENABLED,
            "{bad:?}"
        );
    }
}

#[test]
fn only_hh_mm_times_are_accepted() {
    for bad in ["25:00", "6:00", "6:00pm", "06:00:30", "0600", "noon"] {
        assert_eq!(
            refused_key(&[(KEY_CHAT_KEEPWARM_WINDOW_START, bad)]),
            KEY_CHAT_KEEPWARM_WINDOW_START,
            "{bad:?}"
        );
    }
}

#[test]
fn a_window_whose_end_is_not_after_its_start_is_refused() {
    for end in ["06:00", "05:59"] {
        assert_eq!(
            refused_key(&[(KEY_CHAT_KEEPWARM_WINDOW_END, end)]),
            KEY_CHAT_KEEPWARM_WINDOW_END,
            "{end}"
        );
    }
}

#[test]
fn the_interval_and_the_cap_keep_their_bounds() {
    assert_eq!(
        refused_key(&[(KEY_CHAT_KEEPWARM_INTERVAL_MINUTES, "4")]),
        KEY_CHAT_KEEPWARM_INTERVAL_MINUTES
    );
    assert_eq!(
        refused_key(&[(KEY_CHAT_KEEPWARM_INTERVAL_MINUTES, "56")]),
        KEY_CHAT_KEEPWARM_INTERVAL_MINUTES
    );
    assert_eq!(
        refused_key(&[(KEY_CHAT_KEEPWARM_DAILY_CAP_DOLLARS, "-0.01")]),
        KEY_CHAT_KEEPWARM_DAILY_CAP_DOLLARS
    );
}

#[test]
fn a_missing_row_is_named() {
    let mut rows = rows_with(&[]);
    rows.remove(KEY_CHAT_KEEPWARM_WINDOW_END);
    assert!(matches!(
        build_keepwarm_params(&rows),
        Err(SettingError::Missing { key }) if key == KEY_CHAT_KEEPWARM_WINDOW_END
    ));
}
