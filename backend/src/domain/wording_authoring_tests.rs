// Tests for `domain::wording_authoring` — the twenty-three strings ruling C4b
// moved out of two React components.
//
// The fixture lives here for the same reason as its four siblings: beside the
// test that pins it to the migration FILE, so a fixture and its proof cannot
// drift apart.

use super::*;
use crate::domain::wording::tests::seeded_value_in;
use crate::domain::wording_templates::{missing_placeholders, REQUIRED_PLACEHOLDERS};
use std::collections::HashMap;

/// The migration that seeds all twenty-three rows.
const SEED_MIGRATION: &str = "pipeline_migrations/20260806135509_rehearsal_visual_2_11c.sql";

/// The seeded values, for TESTS ONLY. `cfg(test)` for the reason every sibling
/// fixture is — see `wording_rehearsal_chrome_tests`.
const TEST_SEED: &[(&str, &str)] = &[
    (KEY_POINTS_SECTION_HEADING, "Marie's talking points"),
    (KEY_POINTS_SECTION_META, "her own words · up to {cap}"),
    (
        KEY_POINTS_EMPTY,
        "No talking points yet — these are the sentences Marie says when she is \
         pressed on this scenario.",
    ),
    (KEY_POINTS_NO_EXHIBIT, "No exhibit paired yet"),
    (KEY_POINTS_ADD, "+ Add talking point"),
    (KEY_POINTS_EDIT, "Edit"),
    (KEY_POINTS_SAVE, "Save"),
    (KEY_POINTS_SAVING, "Saving…"),
    (KEY_POINTS_CANCEL, "Cancel"),
    (
        KEY_POINTS_CAP_REACHED,
        "That is already {cap} points — the most a witness can hold.",
    ),
    (KEY_POINTS_FIELD_LABEL, "Talking point {n}"),
    (
        KEY_POINTS_AUTHORING_NOTE,
        "Authored by you and Marie — the system never rewrites these.",
    ),
    (
        KEY_POINTS_SAVE_FAILED,
        "That talking point did not save. Your words are still on screen — try \
         again.",
    ),
    (KEY_WATCH_SECTION_HEADING, "Watch-list"),
    (
        KEY_WATCH_SECTION_META,
        "what the other side will wave around",
    ),
    (KEY_WATCH_FIELD_LABEL, "Flag something to watch for"),
    (KEY_WATCH_ADD, "+ Add watch-list note"),
    (KEY_WATCH_SAVE, "Save"),
    (KEY_WATCH_EDIT, "Edit"),
    (KEY_WATCH_CANCEL, "Cancel"),
    (KEY_WATCH_REMOVE, "Remove"),
    (KEY_WATCH_EDITED_SUFFIX, "edited since written"),
    (
        KEY_WATCH_SAVE_FAILED,
        "That watch item did not save. Your words are still on screen — try again.",
    ),
];

impl AuthoringWording {
    /// The fixture, built through the PRODUCTION builder.
    pub fn for_test() -> Self {
        build_authoring_wording::<String>(|key| {
            TEST_SEED
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
                .ok_or_else(|| format!("{key} is missing from TEST_SEED"))
        })
        .expect("every key in AUTHORING_WORDING_KEYS is in TEST_SEED")
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

    let fixture = AuthoringWording::for_test_values();
    let mut checked = 0usize;

    for key in AUTHORING_WORDING_KEYS {
        let seeded = seeded_value_in(&sql, key)
            .unwrap_or_else(|| panic!("{key} is not seeded by the migration"));
        let in_fixture = fixture
            .get(*key)
            .unwrap_or_else(|| panic!("{key} is missing from TEST_SEED"));

        assert_eq!(
            in_fixture, &seeded,
            "the fixture has {key} = '{in_fixture}' but the migration seeds \
             '{seeded}'."
        );
        checked += 1;
    }

    assert_eq!(checked, 23, "all twenty-three strings must be compared");
    assert_eq!(AUTHORING_WORDING_KEYS.len(), 23);
}

#[test]
fn no_key_appears_twice_and_none_collides_with_a_sibling_list() {
    let mut seen: Vec<&str> = AUTHORING_WORDING_KEYS.to_vec();
    seen.sort_unstable();
    let before = seen.len();
    seen.dedup();
    assert_eq!(before, seen.len(), "a key is listed twice");

    for key in AUTHORING_WORDING_KEYS {
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
    }
}

#[test]
fn every_field_is_read_from_its_own_key() {
    let w = build_authoring_wording::<std::convert::Infallible>(|key| Ok(key.to_string()))
        .expect("the identity reader cannot fail");

    assert_eq!(w.points_section_heading, KEY_POINTS_SECTION_HEADING);
    assert_eq!(w.points_section_meta_template, KEY_POINTS_SECTION_META);
    assert_eq!(w.points_empty_notice, KEY_POINTS_EMPTY);
    assert_eq!(w.points_no_exhibit_notice, KEY_POINTS_NO_EXHIBIT);
    assert_eq!(w.points_add_label, KEY_POINTS_ADD);
    assert_eq!(w.points_edit_label, KEY_POINTS_EDIT);
    assert_eq!(w.points_save_label, KEY_POINTS_SAVE);
    assert_eq!(w.points_saving_label, KEY_POINTS_SAVING);
    assert_eq!(w.points_cancel_label, KEY_POINTS_CANCEL);
    assert_eq!(w.points_cap_reached_notice, KEY_POINTS_CAP_REACHED);
    assert_eq!(w.points_field_label_template, KEY_POINTS_FIELD_LABEL);
    assert_eq!(w.points_authoring_note, KEY_POINTS_AUTHORING_NOTE);
    assert_eq!(w.points_save_failed_notice, KEY_POINTS_SAVE_FAILED);
    assert_eq!(w.watch_section_heading, KEY_WATCH_SECTION_HEADING);
    assert_eq!(w.watch_section_meta, KEY_WATCH_SECTION_META);
    assert_eq!(w.watch_field_label, KEY_WATCH_FIELD_LABEL);
    assert_eq!(w.watch_add_label, KEY_WATCH_ADD);
    assert_eq!(w.watch_save_label, KEY_WATCH_SAVE);
    assert_eq!(w.watch_edit_label, KEY_WATCH_EDIT);
    assert_eq!(w.watch_cancel_label, KEY_WATCH_CANCEL);
    assert_eq!(w.watch_remove_label, KEY_WATCH_REMOVE);
    assert_eq!(w.watch_edited_suffix, KEY_WATCH_EDITED_SUFFIX);
    assert_eq!(w.watch_save_failed_notice, KEY_WATCH_SAVE_FAILED);
}

#[test]
fn a_missing_row_refuses_the_whole_build_and_names_the_key() {
    for missing in AUTHORING_WORDING_KEYS {
        let values = AuthoringWording::for_test_values();
        let result = build_authoring_wording(|key| {
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
fn the_three_templates_keep_their_facts() {
    // "her own words · up to " states a limit and withholds it; "That is already
    // points" refuses without naming the ceiling; a field label with no {n}
    // announces every box in the list identically to a screen reader.
    let fixture = AuthoringWording::for_test_values();

    for key in [
        KEY_POINTS_SECTION_META,
        KEY_POINTS_CAP_REACHED,
        KEY_POINTS_FIELD_LABEL,
    ] {
        let value = fixture.get(key).expect("the fixture carries it");
        assert!(
            missing_placeholders(key, value).is_empty(),
            "{key} has lost a placeholder"
        );
        assert!(
            REQUIRED_PLACEHOLDERS.iter().any(|(k, _)| *k == key),
            "{key} is not in REQUIRED_PLACEHOLDERS, so the write path would \
             accept a value with its facts removed"
        );
    }

    assert_eq!(
        missing_placeholders(KEY_POINTS_CAP_REACHED, "That is already the most"),
        vec!["{cap}"]
    );
}
