// Tests for `domain::wording_accusation` — task 2.11's twenty-seven stored strings.
//
// One thing needs proving here above all others, and it is the one that would rot
// silently: the test fixture still says what the MIGRATION seeds. A fixture
// compared only to itself proves nothing, and every assertion elsewhere that
// mentions one of these sentences would then be describing a product that says
// something else.
//
// The seed parser is BORROWED from `wording_tests` rather than copied. It is the
// only thing standing between this test and a false green, so there is exactly one
// of it and it has its own test next door.

use super::*;
use crate::domain::wording::tests::seeded_value_in;
use crate::domain::wording_templates::{missing_placeholders, REQUIRED_PLACEHOLDERS};

/// The migration that seeds all twenty-seven rows.
///
/// ONE file, unlike the link-panel wording's five: these arrived together in a
/// single task. If a later task corrects one of these values it must do so in its
/// own migration with a guarded UPDATE — `ON CONFLICT (key) DO NOTHING` means
/// editing this file's VALUES list changes nothing on an environment that has
/// already run it (2.12's lesson, paid for once).
const SEED_MIGRATION: &str = "pipeline_migrations/20260806075620_accusation_authoring_wording.sql";

#[test]
fn the_fixture_carries_the_values_the_migration_actually_seeds() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let sql = std::fs::read_to_string(root.join(SEED_MIGRATION))
        .expect("the accusation wording migration is on disk");

    let fixture = AccusationWording::for_test_values();
    let mut checked = 0usize;

    for key in ACCUSATION_WORDING_KEYS {
        let seeded = seeded_value_in(&sql, key)
            .unwrap_or_else(|| panic!("{key} is not seeded by the migration"));
        let in_fixture = fixture
            .get(*key)
            .unwrap_or_else(|| panic!("{key} is missing from the test fixture"));

        assert_eq!(
            in_fixture, &seeded,
            "the fixture has {key} = '{in_fixture}' but the migration seeds \
             '{seeded}'. One moved without the other, and every test that asserts \
             on this wording is now describing something the product does not say."
        );
        checked += 1;
    }

    // Anti-vacuity: a parsing change that stopped finding rows would otherwise
    // make this test pass while comparing nothing.
    assert_eq!(
        checked, 27,
        "all twenty-seven stored strings must be compared"
    );
    assert_eq!(ACCUSATION_WORDING_KEYS.len(), 27);
}

#[test]
fn no_key_appears_twice_in_the_list() {
    // A duplicate would make the count above pass while one string went unchecked,
    // and `build_accusation_wording` would read the same row into two fields.
    let mut seen: Vec<&str> = ACCUSATION_WORDING_KEYS.to_vec();
    seen.sort_unstable();
    let before = seen.len();
    seen.dedup();
    assert_eq!(before, seen.len(), "a key is listed twice");
}

#[test]
fn no_accusation_key_collides_with_a_link_panel_key() {
    // Both lists key the SAME `app_settings` table. A collision would mean two
    // struct fields fed by one row — one surface silently editing another's words,
    // with the boot loader perfectly happy.
    for key in ACCUSATION_WORDING_KEYS {
        assert!(
            !crate::domain::wording::WORDING_KEYS.contains(key),
            "{key} is claimed by both stored-string lists"
        );
    }
}

#[test]
fn a_missing_row_refuses_the_whole_build_and_names_the_key() {
    // The failure law: a string is a stored parameter with the standing of a
    // threshold. A missing label must not degrade to an empty button — that would
    // be a compiled-in default by omission.
    for missing in ACCUSATION_WORDING_KEYS {
        let values = AccusationWording::for_test_values();
        let result = build_accusation_wording(|key| {
            if key == *missing {
                return Err(format!("no parameter named '{key}' is stored"));
            }
            Ok(values
                .get(key)
                .unwrap_or_else(|| panic!("{key} is seeded"))
                .clone())
        });

        let Err(error) = result else {
            panic!("a store missing {missing} must not produce a wording block");
        };
        assert!(
            error.contains(missing),
            "the refusal must name the missing string: {error}"
        );
    }
}

#[test]
fn every_field_is_read_from_its_own_key() {
    // Anti-drift of the copy-paste kind: `text_save_label: read(KEY_TEXT_EDIT_
    // LABEL)` compiles, type-checks, and puts "Edit" on the Save button. Feeding
    // each key its own name back proves the wiring one field at a time.
    let built = build_accusation_wording::<std::convert::Infallible>(|key| Ok(key.to_string()))
        .expect("the identity reader cannot fail");

    assert_eq!(built.section_heading, KEY_SECTION_HEADING);
    assert_eq!(built.section_hint, KEY_SECTION_HINT);
    assert_eq!(built.text_label, KEY_TEXT_LABEL);
    assert_eq!(built.text_placeholder, KEY_TEXT_PLACEHOLDER);
    assert_eq!(built.text_missing_notice, KEY_TEXT_MISSING_NOTICE);
    assert_eq!(built.text_edit_label, KEY_TEXT_EDIT_LABEL);
    assert_eq!(built.text_save_label, KEY_TEXT_SAVE_LABEL);
    assert_eq!(built.text_clear_label, KEY_TEXT_CLEAR_LABEL);
    assert_eq!(built.text_cancel_label, KEY_TEXT_CANCEL_LABEL);
    assert_eq!(built.count_template, KEY_COUNT_TEMPLATE);
    assert_eq!(built.no_instances_notice, KEY_NO_INSTANCES_NOTICE);
    assert_eq!(built.mark_label, KEY_MARK_LABEL);
    assert_eq!(built.unmark_label, KEY_UNMARK_LABEL);
    assert_eq!(built.pair_label, KEY_PAIR_LABEL);
    assert_eq!(built.repair_label, KEY_REPAIR_LABEL);
    assert_eq!(built.unpair_label, KEY_UNPAIR_LABEL);
    assert_eq!(built.answer_label, KEY_ANSWER_LABEL);
    assert_eq!(built.picker_prompt, KEY_PICKER_PROMPT);
    assert_eq!(built.picker_cancel_label, KEY_PICKER_CANCEL_LABEL);
    assert_eq!(built.picker_no_match_notice, KEY_PICKER_NO_MATCH);
    assert_eq!(built.picker_empty_notice, KEY_PICKER_EMPTY);
    assert_eq!(built.gaps_heading, KEY_GAPS_HEADING);
    assert_eq!(built.no_gaps_notice, KEY_NO_GAPS_NOTICE);
    assert_eq!(built.gap_no_answer, KEY_GAP_NO_ANSWER);
    assert_eq!(built.gap_accusation_removed, KEY_GAP_ACCUSATION_REMOVED);
    assert_eq!(built.gap_answer_removed, KEY_GAP_ANSWER_REMOVED);
    assert_eq!(built.save_failed_template, KEY_SAVE_FAILED_TEMPLATE);
}

#[test]
fn the_six_templates_are_guarded_by_the_placeholder_table() {
    // The law that a template must keep its facts, applied to THIS module's keys.
    // Without an entry in `REQUIRED_PLACEHOLDERS` a key is silently unconstrained,
    // and an edit could ship "Said  times, in  documents." — grammatical, renders
    // perfectly, and states nothing.
    let guarded: Vec<&str> = REQUIRED_PLACEHOLDERS
        .iter()
        .map(|(key, _)| *key)
        .filter(|key| ACCUSATION_WORDING_KEYS.contains(key))
        .collect();

    assert_eq!(
        guarded.len(),
        6,
        "the count line, the no-instances notice, the three gap messages and the \
         failure template all carry facts an edit must not drop: {guarded:?}"
    );

    assert_eq!(
        missing_placeholders(KEY_COUNT_TEMPLATE, "Said them a lot."),
        vec!["{times}", "{documents}"]
    );
    assert_eq!(
        missing_placeholders(KEY_GAP_NO_ANSWER, "NO ANSWER PREPARED"),
        vec!["{code}"]
    );
    assert!(missing_placeholders(KEY_GAP_NO_ANSWER, "{code} needs an answer").is_empty());
}

#[test]
fn no_seeded_string_is_blank() {
    // A blank label is an invisible control. The store refuses one on write; this
    // pins the DEFAULTS, which the write path never sees.
    for (key, value) in AccusationWording::for_test_values() {
        assert!(!value.trim().is_empty(), "{key} seeds a blank string");
    }
}

#[test]
fn the_gap_messages_are_three_different_sentences() {
    // The honest-gap law names three genuinely different absences, and the whole
    // point is that a human can tell them apart and knows which remedy each needs.
    // Seeding two of them identically would satisfy every other test here.
    let w = AccusationWording::for_test();
    assert_ne!(w.gap_no_answer, w.gap_accusation_removed);
    assert_ne!(w.gap_no_answer, w.gap_answer_removed);
    assert_ne!(w.gap_accusation_removed, w.gap_answer_removed);
}
