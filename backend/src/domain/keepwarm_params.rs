//! The automatic keep-loaded ping's five settings (CC_TASK_CACHE_KEEPWARM_v1 R3).
//!
//! Nested inside [`crate::domain::chat_params::QuestionChatParams`] as its
//! `keepwarm` field rather than hung off `Settings` directly, because
//! `domain/settings.rs` is a few lines under Rule 17's limit — and because these
//! ARE the question chat's settings: they decide when its case file is re-sent.
//!
//! ## Domain note: why a window and a cap
//!
//! A ping costs about $0.07 and buys another hour of a case file that costs about
//! $2.95 to reload. It pays only while somebody is going to ask again. The window
//! keeps it to the hours people prepare (nobody asks at 3 am); the daily cap
//! bounds what a day can cost however the rules behave.
//!
//! The seeded values live in
//! `pipeline_migrations/20260925081941_chat_keepwarm_settings.sql`; the test below
//! reads that file so the fixture cannot drift from it (Rule 21).

use chrono::NaiveTime;

/// The block as the pinger and the Admin box read it.
#[derive(Debug, Clone, PartialEq)]
pub struct KeepwarmParams {
    /// `on` / `off`.
    pub enabled: bool,
    /// Local time (in the case's timezone) the window opens.
    pub window_start: NaiveTime,
    /// Local time the window closes; always after `window_start`.
    pub window_end: NaiveTime,
    /// Minutes after the last question or ping before the next ping.
    pub interval_minutes: u32,
    /// The most automatic pings may cost in one day, in dollars.
    pub daily_cap_dollars: f64,
}

// KEYS: the stable identifiers. Renaming one is a migration.
pub const KEY_CHAT_KEEPWARM_ENABLED: &str = "chat_keepwarm_enabled";
pub const KEY_CHAT_KEEPWARM_WINDOW_START: &str = "chat_keepwarm_window_start";
pub const KEY_CHAT_KEEPWARM_WINDOW_END: &str = "chat_keepwarm_window_end";
pub const KEY_CHAT_KEEPWARM_INTERVAL_MINUTES: &str = "chat_keepwarm_interval_minutes";
pub const KEY_CHAT_KEEPWARM_DAILY_CAP_DOLLARS: &str = "chat_keepwarm_daily_cap_dollars";

/// Every key this block reads — counted at boot, bundled on the Settings page,
/// and walked by the store tests.
pub const CHAT_KEEPWARM_PARAM_KEYS: &[&str] = &[
    KEY_CHAT_KEEPWARM_ENABLED,
    KEY_CHAT_KEEPWARM_WINDOW_START,
    KEY_CHAT_KEEPWARM_WINDOW_END,
    KEY_CHAT_KEEPWARM_INTERVAL_MINUTES,
    KEY_CHAT_KEEPWARM_DAILY_CAP_DOLLARS,
];

// STRUCTURAL: the two spellings of the switch — a vocabulary the reader checks,
// not a tunable value (which one is chosen IS the setting).
pub const ENABLED_ON: &str = "on";
// STRUCTURAL: see `ENABLED_ON`.
pub const ENABLED_OFF: &str = "off";

/// One seeded row for tests: `(key, kind, value, min, max)`.
#[cfg(test)]
pub type SeedRow = (
    &'static str,
    &'static str,
    &'static str,
    Option<f64>,
    Option<f64>,
);

#[cfg(test)]
impl KeepwarmParams {
    /// The seeded rows, `(key, kind, value, min, max)`, for tests only.
    pub fn for_test_rows() -> [SeedRow; 5] {
        [
            (KEY_CHAT_KEEPWARM_ENABLED, "text", "on", None, None),
            (KEY_CHAT_KEEPWARM_WINDOW_START, "text", "06:00", None, None),
            (KEY_CHAT_KEEPWARM_WINDOW_END, "text", "23:00", None, None),
            (
                KEY_CHAT_KEEPWARM_INTERVAL_MINUTES,
                "count",
                "48",
                Some(5.0),
                Some(55.0),
            ),
            (
                KEY_CHAT_KEEPWARM_DAILY_CAP_DOLLARS,
                "float",
                "2.00",
                Some(0.0),
                Some(100.0),
            ),
        ]
    }

    /// The block as the seeded store builds it.
    pub fn for_test() -> Self {
        Self {
            enabled: true,
            window_start: NaiveTime::from_hms_opt(6, 0, 0).expect("valid literal"),
            window_end: NaiveTime::from_hms_opt(23, 0, 0).expect("valid literal"),
            interval_minutes: 48,
            daily_cap_dollars: 2.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::wording::tests::seeded_value_in;

    const SEED_MIGRATION: &str = "pipeline_migrations/20260925081941_chat_keepwarm_settings.sql";

    /// Every key is seeded by the migration with the fixture's value.
    #[test]
    fn every_parameter_is_seeded_with_the_fixture_value() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let sql = std::fs::read_to_string(root.join(SEED_MIGRATION))
            .expect("the keep-warm migration is on disk");
        let rows = KeepwarmParams::for_test_rows();
        assert_eq!(rows.len(), CHAT_KEEPWARM_PARAM_KEYS.len());
        for key in CHAT_KEEPWARM_PARAM_KEYS {
            let seeded =
                seeded_value_in(&sql, key).unwrap_or_else(|| panic!("{key} is not seeded"));
            let fixture = rows
                .iter()
                .find(|r| r.0 == *key)
                .unwrap_or_else(|| panic!("{key} is in no fixture"));
            assert_eq!(seeded, fixture.2, "{key}: migration and fixture disagree");
        }
    }
}
