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

/// The migration that seeds all three rows.
const SEED_MIGRATION: &str =
    "pipeline_migrations/20260808084539_theme_scan_tier2_settings_and_scan_wording.sql";

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
        "Remove the scan run from {run}? Its verdicts are deleted with it, and \
         they are what support the rulings it produced.",
    ),
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

/// Every declared key is seeded, with the value this build expects.
#[test]
fn every_declared_key_is_seeded_with_the_value_this_build_expects() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let sql = std::fs::read_to_string(root.join(SEED_MIGRATION))
        .expect("the Tier-2 scan migration is on disk");

    let fixture = ScanWording::for_test_values();

    for key in SCAN_WORDING_KEYS {
        let seeded = seeded_value_in(&sql, key).unwrap_or_else(|| {
            panic!("{key} is declared to the boot loader but the migration seeds no row for it")
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
