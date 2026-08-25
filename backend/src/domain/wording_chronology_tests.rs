//! Tests for `domain::wording_chronology`.
//!
//! Same shape and the same single justification as every sibling wording test
//! file: a key declared to the boot loader with no row in the migration makes
//! the backend REFUSE TO START. That is a deploy taking DEV down, and reading
//! the migration off disk is the only thing that catches it beforehand
//! (Rule 21, the disk/code consistency pattern).

use super::*;
use crate::domain::wording::tests::{corrected_value_in, seeded_value_in};
use std::collections::HashMap;

/// The migration that seeds every row this module reads.
// STRUCTURAL: a repo-internal pointer to one immutable, version-controlled
// migration. Identical in every environment; nothing here can vary by deployment.
const SEED_MIGRATIONS: &[&str] =
    &["pipeline_migrations/20260825150938_chronology_wording_and_phase_window.sql"];

/// Migrations that CORRECT a value the seed already wrote.
///
/// Empty today and present anyway: the block this file guards is one day old,
/// and `wording_practice_list` had to grow this list in a hurry the first time
/// one of its rows was corrected. The lookup below already searches it.
// STRUCTURAL: same judgement as SEED_MIGRATIONS above.
const CORRECTION_MIGRATIONS: &[&str] = &[];

/// The seeded values, for TESTS ONLY — kept beside the test that pins them to
/// the migration file, so a fixture and its proof cannot drift apart.
const TEST_SEED: &[(&str, &str)] = &[
    (KEY_PAGE_TITLE, "Case Timeline"),
    (KEY_COUNT_TEMPLATE, "{events} events across {phases} phases"),
    (KEY_FILTERED_COUNT_TEMPLATE, "Showing {phase} · {shown} of {total} events"),
    (KEY_SEARCH_PLACEHOLDER, "Search events, facts, notes…"),
    (KEY_ALL_TAGS_LABEL, "All"),
    (KEY_DATES_LABEL, "Dates"),
    (KEY_DATE_FROM_LABEL, "From"),
    (KEY_DATE_TO_LABEL, "To"),
    (KEY_EXPAND_LABEL, "⤢ Expand"),
    (KEY_SHOW_ALL_PHASES_LABEL, "⇲ Show all phases"),
    (KEY_SCROLL_HINT_TEMPLATE, "↕ scroll window — shows {count} at a time (size configurable in settings)"),
    (KEY_PHASE_COUNT_TEMPLATE, "{range} · {count} events"),
    (KEY_NO_DOCUMENT_LABEL, "⚠ no document yet"),
    (KEY_LINK_UNCHECKED_LABEL, "◌ not checked"),
    (KEY_NOTE_COUNT_TEMPLATE, "💬 {count} notes"),
    (KEY_NOTE_COUNT_ONE, "💬 1 note"),
    (KEY_NO_PINPOINT_LABEL, "no pinpoint"),
    (KEY_EMPTY_LABEL, "No events in this case yet."),
    (KEY_NO_MATCHES_LABEL, "No events match these filters."),
    (KEY_UNKNOWN_PHASE_TEMPLATE, "Event {id} names a phase this build does not know ({phase}). It is shown here so it can be corrected."),
    (KEY_BACK_LABEL, "← Case Timeline"),
    (KEY_DOCUMENTS_HEADING, "Documents"),
    (KEY_NOTES_HEADING, "Notes"),
    (KEY_HISTORY_HEADING, "History"),
    (KEY_NO_HISTORY_LABEL, "No changes recorded yet"),
    (KEY_NO_NOTES_LABEL, "No notes yet"),
    (KEY_BAND_MISMATCH_TEMPLATE, "{shown} of {total} events are in a phase this page can show."),
];

impl ChronologyWording {
    /// The fixture, built through the PRODUCTION builder — so a fixture the real
    /// builder would reject cannot exist.
    pub fn for_test() -> Self {
        build_chronology_wording::<String>(|key| {
            TEST_SEED
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
                .ok_or_else(|| {
                    format!(
                        "{key} is declared to the boot loader but missing from this file's \
                         TEST_SEED fixture — add the value the migration seeds"
                    )
                })
        })
        .expect("every key in CHRONOLOGY_WORDING_KEYS is in TEST_SEED")
    }

    /// The fixture as a key→value map, in the shape the store reads.
    pub fn for_test_values() -> HashMap<&'static str, String> {
        TEST_SEED
            .iter()
            .map(|(key, value)| (*key, (*value).to_string()))
            .collect()
    }
}

/// Read every named migration off disk, failing loudly if one has moved.
fn read_all(files: &[&str]) -> Vec<String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    files
        .iter()
        .map(|file| {
            std::fs::read_to_string(root.join(file))
                .unwrap_or_else(|cause| panic!("{file} is not on disk: {cause}"))
        })
        .collect()
}

/// The value the store ends up holding: the newest correction, else the seed.
fn effective_value(corrections: &[String], seeds: &[String], key: &str) -> Option<String> {
    corrections
        .iter()
        .find_map(|sql| corrected_value_in(sql, key))
        .or_else(|| seeds.iter().find_map(|sql| seeded_value_in(sql, key)))
}

#[test]
fn every_declared_key_is_seeded_with_the_value_this_build_expects() {
    let seeds = read_all(SEED_MIGRATIONS);
    let corrections = read_all(CORRECTION_MIGRATIONS);

    for key in CHRONOLOGY_WORDING_KEYS {
        let seeded = effective_value(&corrections, &seeds, key).unwrap_or_else(|| {
            panic!(
                "{key} is declared to the boot loader but seeded by no migration \
                 — the backend would refuse to start"
            )
        });
        let expected = TEST_SEED
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| (*v).to_string())
            .unwrap_or_else(|| panic!("{key} is missing from TEST_SEED"));
        assert_eq!(
            seeded, expected,
            "the migration and the fixture disagree about {key}"
        );
    }
}

#[test]
fn the_fixture_declares_no_key_the_build_does_not_read() {
    for (key, _) in TEST_SEED {
        assert!(
            CHRONOLOGY_WORDING_KEYS.contains(key),
            "{key} is in TEST_SEED but declared to no block — it would be a row nothing reads"
        );
    }
    assert_eq!(TEST_SEED.len(), CHRONOLOGY_WORDING_KEYS.len());
}

#[test]
fn a_missing_row_refuses_the_block_and_names_the_key() {
    // The boot refusal, exercised: the builder must fail on the FIRST missing
    // key and say which one, because that message is all an operator gets when
    // the backend will not start.
    let refusal = build_chronology_wording::<String>(|key| {
        if key == KEY_NO_DOCUMENT_LABEL {
            return Err(format!("no row for {key}"));
        }
        Ok("x".to_string())
    })
    .expect_err("a missing row must refuse the block");
    assert!(refusal.contains(KEY_NO_DOCUMENT_LABEL), "got: {refusal}");
}

#[test]
fn the_templates_carry_the_placeholders_their_callers_fill() {
    let w = ChronologyWording::for_test();
    // A template whose placeholder was edited out renders a sentence with a
    // number missing from it, which is worse than a compile error and quieter.
    assert!(w.count_template.contains("{events}") && w.count_template.contains("{phases}"));
    for token in ["{phase}", "{shown}", "{total}"] {
        assert!(w.filtered_count_template.contains(token), "missing {token}");
    }
    assert!(w.scroll_hint_template.contains("{count}"));
    assert!(
        w.phase_count_template.contains("{range}") && w.phase_count_template.contains("{count}")
    );
    assert!(w.note_count_template.contains("{count}"));
    assert!(
        w.unknown_phase_template.contains("{id}") && w.unknown_phase_template.contains("{phase}")
    );
    assert!(
        w.band_mismatch_template.contains("{shown}")
            && w.band_mismatch_template.contains("{total}")
    );
}

#[test]
fn missing_and_unchecked_are_different_sentences() {
    // The whole point of the three-state resolution: a reader must be able to
    // tell "looked for and not there" from "nobody looked".
    let w = ChronologyWording::for_test();
    assert_ne!(w.no_document_label, w.link_unchecked_label);
    assert!(w.no_document_label.contains("no document"));
    assert!(w.link_unchecked_label.contains("not checked"));
}

#[test]
fn the_empty_case_and_the_over_filtered_case_say_different_things() {
    let w = ChronologyWording::for_test();
    assert_ne!(w.empty_label, w.no_matches_label);
}
