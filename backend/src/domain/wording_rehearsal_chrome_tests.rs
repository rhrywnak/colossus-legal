// Tests for `domain::wording_rehearsal_chrome` — task 2.11 C's eighteen strings.
//
// This file also OWNS the seeded fixture, for the reason its sibling gives: a
// fixture compared only to itself proves nothing, so it lives beside the test
// that pins it to the migration FILE. One `TEST_SEED` table feeds both fixture
// shapes through the same builder production uses, so there is one copy and the
// builder is exercised by every test that touches it.

use super::*;
use crate::domain::wording::tests::seeded_value_in;
use crate::domain::wording_templates::{missing_placeholders, REQUIRED_PLACEHOLDERS};
use std::collections::HashMap;

/// The migration that seeds all eighteen rows.
const SEED_MIGRATION: &str = "pipeline_migrations/20260806135509_rehearsal_visual_2_11c.sql";

/// The seeded values, for TESTS ONLY.
///
/// `cfg(test)` for the reason every sibling fixture is: a production-reachable
/// default would be a compiled-in set of user-facing strings — the defect Roman's
/// 2026-08-04 ruling deletes — and it would be reachable by accident through
/// `unwrap_or_default()`. Gated here, it cannot exist in a release binary.
const TEST_SEED: &[(&str, &str)] = &[
    (KEY_ANSWERED_TAG, "ANSWERED"),
    (KEY_NO_ANSWER_TAG, "NO ANSWER"),
    (KEY_NO_ANSWER_BANNER, "NO ANSWER PREPARED"),
    (KEY_SIDE_THEIRS, "THEY SAY"),
    (KEY_SIDE_OURS, "OUR ANSWER"),
    (KEY_WHAT_ATTRIBUTION, "Written by {who} · {when}"),
    (
        KEY_ACCUSATION_ATTRIBUTION,
        "Written in plain words by {who} · {when}",
    ),
    (
        KEY_ATTRIBUTION_UNKNOWN,
        "Author not recorded — written before authorship was kept.",
    ),
    (KEY_SCENARIO_PAGE, "Scenario page"),
    (KEY_CRUMB_TRIAL_PREP, "Trial Prep"),
    (KEY_GO_TO_ROW, "go to row"),
    (KEY_PREP_LIST_HEADING, "What still needs preparing"),
    (KEY_ROW_OPEN_HINT, "Click a row to open it."),
    (KEY_ADD_POINT, "+ Add talking point"),
    (KEY_ADD_WATCH, "+ Add watch item"),
    (KEY_POINT_NO_EXHIBIT, "No exhibit paired yet"),
    (
        KEY_POINTS_AUTHORING_NOTE,
        "Authored by you and Marie — the system never rewrites these. Spellcheck \
         underlines while you type; what you save is what is stored.",
    ),
    (
        KEY_WHAT_PLACEHOLDER,
        "The fight over whether Marie blocked an amicable division of her \
         father's property.",
    ),
];

impl RehearsalChromeWording {
    /// The fixture, built through the PRODUCTION builder.
    ///
    /// Going through `build_rehearsal_chrome_wording` rather than writing a
    /// struct literal means the key-to-field wiring is exercised here too — a
    /// second hand-typed struct would compile happily while the builder was
    /// wrong.
    pub fn for_test() -> Self {
        build_rehearsal_chrome_wording::<String>(|key| {
            TEST_SEED
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
                .ok_or_else(|| format!("{key} is missing from TEST_SEED"))
        })
        .expect("every key in REHEARSAL_CHROME_KEYS is in TEST_SEED")
    }

    /// The fixture as a key→value map, in the shape the store reads.
    pub fn for_test_values() -> HashMap<&'static str, String> {
        TEST_SEED
            .iter()
            .map(|(key, value)| (*key, (*value).to_string()))
            .collect()
    }
}

#[test]
fn the_fixture_carries_the_values_the_migration_actually_seeds() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let sql = std::fs::read_to_string(root.join(SEED_MIGRATION))
        .expect("the 2.11 C migration is on disk");

    let fixture = RehearsalChromeWording::for_test_values();
    let mut checked = 0usize;

    for key in REHEARSAL_CHROME_KEYS {
        let seeded = seeded_value_in(&sql, key)
            .unwrap_or_else(|| panic!("{key} is not seeded by the migration"));
        let in_fixture = fixture
            .get(*key)
            .unwrap_or_else(|| panic!("{key} is missing from TEST_SEED"));

        assert_eq!(
            in_fixture, &seeded,
            "the fixture has {key} = '{in_fixture}' but the migration seeds \
             '{seeded}'. One moved without the other, and every test asserting on \
             this wording is now describing something the product does not say."
        );
        checked += 1;
    }

    // Anti-vacuity: a parsing change that stopped finding rows would otherwise
    // make this test pass while comparing nothing.
    assert_eq!(checked, 18, "all eighteen strings must be compared");
    assert_eq!(REHEARSAL_CHROME_KEYS.len(), 18);
}

#[test]
fn no_key_appears_twice_and_none_collides_with_a_sibling_list() {
    // All five stored-string lists key the SAME `app_settings` table. A collision
    // would mean two struct fields fed by one row — one surface silently editing
    // another's words, with the boot loader perfectly happy.
    let mut seen: Vec<&str> = REHEARSAL_CHROME_KEYS.to_vec();
    seen.sort_unstable();
    let before = seen.len();
    seen.dedup();
    assert_eq!(before, seen.len(), "a key is listed twice");

    for key in REHEARSAL_CHROME_KEYS {
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
            !crate::domain::wording_authoring::AUTHORING_WORDING_KEYS.contains(key),
            "{key} is also an authoring-section key"
        );
    }
}

#[test]
fn every_field_is_read_from_its_own_key() {
    // Anti-drift of the copy-paste kind: `no_answer_tag: read(KEY_ANSWERED_TAG)`
    // compiles, type-checks, and puts a green "ANSWERED" on a row nobody has
    // answered — the single most dangerous mistake this file could contain.
    // Feeding each key its own name back proves the wiring one field at a time.
    let w = build_rehearsal_chrome_wording::<std::convert::Infallible>(|key| Ok(key.to_string()))
        .expect("the identity reader cannot fail");

    assert_eq!(w.answered_tag, KEY_ANSWERED_TAG);
    assert_eq!(w.no_answer_tag, KEY_NO_ANSWER_TAG);
    assert_eq!(w.no_answer_banner, KEY_NO_ANSWER_BANNER);
    assert_eq!(w.timeline_side_theirs_label, KEY_SIDE_THEIRS);
    assert_eq!(w.timeline_side_ours_label, KEY_SIDE_OURS);
    assert_eq!(w.what_attribution_template, KEY_WHAT_ATTRIBUTION);
    assert_eq!(
        w.accusation_attribution_template,
        KEY_ACCUSATION_ATTRIBUTION
    );
    assert_eq!(w.attribution_unknown_notice, KEY_ATTRIBUTION_UNKNOWN);
    assert_eq!(w.scenario_page_label, KEY_SCENARIO_PAGE);
    assert_eq!(w.crumb_trial_prep_label, KEY_CRUMB_TRIAL_PREP);
    assert_eq!(w.go_to_row_label, KEY_GO_TO_ROW);
    assert_eq!(w.prep_list_heading, KEY_PREP_LIST_HEADING);
    assert_eq!(w.row_open_hint, KEY_ROW_OPEN_HINT);
    assert_eq!(w.add_point_label, KEY_ADD_POINT);
    assert_eq!(w.add_watch_label, KEY_ADD_WATCH);
    assert_eq!(w.point_no_exhibit_notice, KEY_POINT_NO_EXHIBIT);
    assert_eq!(w.points_authoring_note, KEY_POINTS_AUTHORING_NOTE);
    assert_eq!(w.what_placeholder, KEY_WHAT_PLACEHOLDER);
}

#[test]
fn a_missing_row_refuses_the_whole_build_and_names_the_key() {
    // The failure law: a string is a stored parameter with the standing of a
    // threshold. A missing label must not degrade to an empty control — that is a
    // compiled-in default by omission.
    for missing in REHEARSAL_CHROME_KEYS {
        let values = RehearsalChromeWording::for_test_values();
        let result = build_rehearsal_chrome_wording(|key| {
            if key == *missing {
                return Err(format!("{key} is missing"));
            }
            values
                .get(key)
                .cloned()
                .ok_or_else(|| format!("{key} is missing"))
        });

        let error = result.expect_err("a missing row must refuse the build");
        assert!(
            error.contains(missing),
            "the refusal must name the key; got '{error}'"
        );
    }
}

#[test]
fn the_two_attribution_templates_keep_their_facts() {
    // The provenance line is the one place the page claims a HUMAN wrote a
    // sentence. "Written in plain words by  · " makes that claim and then names
    // neither the human nor the day — an attribution that attributes nothing,
    // which is worse than no line at all because it looks like one.
    let fixture = RehearsalChromeWording::for_test_values();

    for key in [KEY_WHAT_ATTRIBUTION, KEY_ACCUSATION_ATTRIBUTION] {
        let value = fixture.get(key).expect("the fixture carries it");
        assert!(
            missing_placeholders(key, value).is_empty(),
            "{key} has lost a placeholder"
        );

        // And the table itself must KNOW about the key — an entry absent from it
        // is silently unconstrained, which is the failure it exists to prevent.
        assert!(
            REQUIRED_PLACEHOLDERS.iter().any(|(k, _)| *k == key),
            "{key} is not in REQUIRED_PLACEHOLDERS, so the write path would \
             accept a value with its facts removed"
        );
    }

    // A template stripped of one placeholder is REFUSED, and the refusal names
    // what is missing rather than saying "invalid".
    let stripped = "Written in plain words by {who}";
    let missing = missing_placeholders(KEY_ACCUSATION_ATTRIBUTION, stripped);
    assert_eq!(missing, vec!["{when}"]);
}
