// Tests for `domain::wording_scan`.
//
// Same shape, and same single justification, as the sibling wording test files:
// a key declared to the boot loader with no row in the migration makes the
// backend REFUSE TO START. That is a deploy taking DEV down, and reading the
// migration off disk is the only thing that catches it before it happens
// (Rule 21, the disk/code consistency pattern). Nothing here restates the code.

use super::*;
use crate::domain::wording::tests::seeded_value_in;
use std::collections::HashMap;

/// The migration that seeds the first three rows.
const SEED_MIGRATION: &str =
    "pipeline_migrations/20260808084539_theme_scan_tier2_settings_and_scan_wording.sql";
/// The scan → ruling migration: eight more rows, and the one CORRECTION.
///
/// The delete confirmation was seeded by the file above and became false when
/// merge died — it promises that a run's verdicts "support the rulings it
/// produced", where the truth is now that unruled proposals vanish with the run
/// and a cited run cannot be deleted at all. So this file's guarded UPDATE is
/// part of the effective value, exactly as `CORRECTION_MIGRATION` is on the
/// curation surface.
const PROJECTION_MIGRATION: &str = "pipeline_migrations/20260808141052_scan_to_ruling_wording.sql";

/// The seeded values, for TESTS ONLY — kept beside the test that pins them to
/// the migration file, so a fixture and its proof cannot drift apart.
const TEST_SEED: &[(&str, &str)] = &[
    (
        KEY_CONSERVATION_LINE,
        "{pool} gathered · {collapsed} duplicates folded · {excluded} set aside \
         before judging · {judged} judged · {relevant} relevant",
    ),
    (KEY_HISTORY_VIEW_LABEL, "View results"),
    (
        KEY_HISTORY_DELETE_CONFIRM,
        "Remove the scan run from {run}? Any candidate it proposed that you have \
         not ruled disappears with it. Your rulings are untouched — and a run your \
         rulings cite cannot be removed at all.",
    ),
    (
        KEY_CARD_COLLAPSED_SUMMARY,
        "Last scan {when} · {model} · {count} proposed",
    ),
    (
        KEY_REPORT_ADVISORY_NOTE,
        "Advisory only. Nothing here needs your click — the proposed candidates \
         are already in the queue below.",
    ),
    (
        KEY_REPORT_PROPOSED_LINE,
        "{count} proposed and awaiting your ruling — a live count, not part of \
         the run's frozen record above.",
    ),
    (KEY_REPORT_TILE_GATHERED, "gathered"),
    (KEY_REPORT_TILE_FOLDED, "duplicates folded"),
    (KEY_REPORT_TILE_SET_ASIDE, "set aside before judging"),
    (KEY_REPORT_TILE_JUDGED, "judged"),
    (KEY_REPORT_TILE_PROPOSED, "proposed"),
];

impl ScanWording {
    /// The fixture, built through the PRODUCTION builder — so a fixture the real
    /// builder would reject cannot exist.
    pub fn for_test() -> Self {
        build_scan_wording::<String>(|key| {
            TEST_SEED
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
                .ok_or_else(|| format!("{key} is missing from TEST_SEED"))
        })
        .expect("every key in SCAN_WORDING_KEYS is in TEST_SEED")
    }

    /// The fixture as a key→value map, in the shape the store reads.
    pub fn for_test_values() -> HashMap<&'static str, String> {
        TEST_SEED
            .iter()
            .map(|(key, value)| (*key, (*value).to_string()))
            .collect()
    }
}

/// The value a guarded UPDATE leaves behind for one key, if it touches it.
///
/// Deliberately crude, exactly like its INSERT sibling: it finds the statement
/// whose `WHERE key = '…'` names this key and reads the `SET value = '…'` literal
/// that precedes it. A migration that corrected a row some other way would not be
/// found — and the equality assertion is what would catch that, by leaving the
/// fixture disagreeing with the product.
///
/// Doubled quotes are one apostrophe (SQL's escape), so the walk below treats
/// `''` as a single character rather than as the end of the literal.
fn corrected_value_in(sql: &str, key: &str) -> Option<String> {
    let at = sql.find(&format!("WHERE key = '{key}'"))?;
    let set = sql[..at].rfind("SET value = '")?;
    let mut out = String::new();
    let mut chars = sql[set + "SET value = '".len()..].chars();
    while let Some(c) = chars.next() {
        if c != '\'' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('\'') => out.push('\''),
            _ => return Some(out),
        }
    }
    None
}

/// Every declared key is seeded, with the value this build expects.
#[test]
fn every_declared_key_is_seeded_with_the_value_this_build_expects() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let sql = std::fs::read_to_string(root.join(SEED_MIGRATION))
        .expect("the Tier-2 scan migration is on disk");
    let projection = std::fs::read_to_string(root.join(PROJECTION_MIGRATION))
        .expect("the scan-to-ruling wording migration is on disk");

    let fixture = ScanWording::for_test_values();

    for key in SCAN_WORDING_KEYS {
        // The CORRECTION is consulted first: a key both files touch ends up with
        // the later file's value, and a fixture pinned to the INSERT alone would
        // assert the product says something it stopped saying.
        let seeded = corrected_value_in(&projection, key)
            .or_else(|| seeded_value_in(&projection, key))
            .or_else(|| seeded_value_in(&sql, key))
            .unwrap_or_else(|| {
                panic!("{key} is declared to the boot loader but no migration seeds a row for it")
            });
        let in_fixture = fixture
            .get(*key)
            .unwrap_or_else(|| panic!("{key} is missing from TEST_SEED"));

        assert_eq!(
            in_fixture, &seeded,
            "the fixture has {key} = '{in_fixture}' but the migration seeds '{seeded}'."
        );
    }
}

/// No key collides with a sibling block's.
///
/// `app_settings` is keyed by `key` alone, so a collision would make two surfaces
/// read ONE row — editing the scan's delete confirmation would silently re-word
/// something on the rehearsal page.
#[test]
fn no_key_collides_with_another_surface_s_key() {
    for key in SCAN_WORDING_KEYS {
        assert!(
            !crate::domain::wording::WORDING_KEYS.contains(key),
            "{key} is also a curation-surface key"
        );
        assert!(
            !crate::domain::wording_accusation::ACCUSATION_WORDING_KEYS.contains(key),
            "{key} is also a working-view accusation key"
        );
        assert!(
            !crate::domain::wording_rehearsal::REHEARSAL_WORDING_KEYS.contains(key),
            "{key} is also a rehearsal prose key"
        );
        assert!(
            !crate::domain::wording_rehearsal_chrome::REHEARSAL_CHROME_KEYS.contains(key),
            "{key} is also a rehearsal chrome key"
        );
        assert!(
            !crate::domain::wording_authoring::AUTHORING_WORDING_KEYS.contains(key),
            "{key} is also a shared authoring-section key"
        );
        assert!(
            !crate::domain::wording_scenario_authoring::SCENARIO_AUTHORING_WORDING_KEYS
                .contains(key),
            "{key} is also a scenario-authoring key"
        );
    }
}

/// The conservation template keeps every number it promises to reconcile.
///
/// Not a restatement of the placeholder table: this asserts that the SEEDED value
/// — the one that ships — satisfies the rule the write path enforces. A seed that
/// the write path would refuse is a row nobody could ever edit back to default.
#[test]
fn the_seeded_conservation_line_carries_all_five_numbers() {
    let seeded = ScanWording::for_test().conservation_line_template;
    for token in [
        "{pool}",
        "{collapsed}",
        "{excluded}",
        "{judged}",
        "{relevant}",
    ] {
        assert!(
            seeded.contains(token),
            "the shipped conservation line must contain {token}, or the sentence \
             reconciles with a term missing and still looks reconciled"
        );
    }
}
