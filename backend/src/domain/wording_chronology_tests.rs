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
const SEED_MIGRATIONS: &[&str] = &[
    "pipeline_migrations/20260825150938_chronology_wording_and_phase_window.sql",
    // Phase C's write controls. In the list on the day they were written: this
    // list going stale is exactly the drift the test exists to catch, and a key
    // declared with no migration here makes the backend refuse to start.
    "pipeline_migrations/20260826104928_chronology_write_wording.sql",
    // T1.2: the sixteen words the Subsets surfaces will speak. In this list on
    // the day they were written, for the reason the line above gives.
    "pipeline_migrations/20260830122249_timeline_subsets.sql",
    // Task 2 (2026-08-30): the seven words Screens 2 and 3 speak.
    "pipeline_migrations/20260830153346_timeline_subsets_screen_wording.sql",
];

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
    (KEY_ADD_EVENT_LABEL, "+ Add event"),
    (KEY_EDIT_LABEL, "✎ Edit"),
    (KEY_DELETE_LABEL, "🗑 Delete"),
    (KEY_DELETED_LINE_LABEL, "Deleted —"),
    (KEY_UNDO_LABEL, "Undo"),
    (KEY_FORM_ADD_TITLE, "Add event"),
    (KEY_FORM_EDIT_TITLE, "Edit event"),
    (KEY_FORM_DATE_LABEL, "Date"),
    (KEY_FORM_PRECISION_LABEL, "Precision"),
    (KEY_PRECISION_DAY_LABEL, "Exact day"),
    (KEY_PRECISION_MONTH_LABEL, "Month only"),
    (KEY_PRECISION_YEAR_LABEL, "Year only"),
    (KEY_FORM_APPROXIMATE_LABEL, "Approximate (~)"),
    (KEY_FORM_TITLE_LABEL, "Title"),
    (KEY_FORM_TITLE_PLACEHOLDER, "One short line — this is what the list shows"),
    (KEY_FORM_FACT_LABEL, "What happened (one plain sentence or two)"),
    (KEY_FORM_FACT_PLACEHOLDER, "Write it so anyone can check it against the source."),
    (KEY_FORM_TAGS_LABEL, "Tags"),
    (KEY_FORM_PHASE_LABEL, "Phase"),
    (KEY_FORM_DOCUMENTS_LABEL, "Documents"),
    (KEY_DOCUMENT_SEARCH_PLACEHOLDER, "🔍 Search documents and pick one…"),
    (KEY_DOCUMENT_SEARCH_EMPTY_LABEL, "No documents match that search."),
    (KEY_PINPOINT_PLACEHOLDER, "pinpoint (page / ¶) optional — a link without one is marked"),
    (KEY_SAVE_LABEL, "Save"),
    (KEY_CANCEL_LABEL, "Cancel"),
    (KEY_SAVING_LABEL, "Saving…"),
    (KEY_ADD_NOTE_PLACEHOLDER, "Add a note…"),
    (KEY_ADD_NOTE_BUTTON_LABEL, "Add"),
    (KEY_LINK_DOCUMENT_LABEL, "+ Link a document…"),
    (KEY_REMOVE_LINK_LABEL, "✕ Remove"),
    (KEY_DELETE_NOTE_LABEL, "✕ Delete note"),
    (KEY_HISTORY_LINE_TEMPLATE, "{when} · {who} · {what}"),
    (KEY_HISTORY_CREATED_LABEL, "created"),
    (KEY_HISTORY_UPDATED_LABEL, "edited"),
    (KEY_HISTORY_DELETED_LABEL, "deleted"),
    (KEY_HISTORY_RESTORED_LABEL, "restored"),
    (KEY_HISTORY_UNKNOWN_TEMPLATE, "{action}"),
    (KEY_WRITE_FAILED_TEMPLATE, "That change was not saved — {reason}"),
    (KEY_PICKER_CAPPED_TEMPLATE, "Showing {shown} of {total} matches — narrow the search to see the rest."),
    // Timeline subsets (T1.2), seeded by the third migration named above.
    (KEY_SUBSETS_SECTION_TITLE, "Subsets"),
    (KEY_SUBSETS_SECTION_SUBTITLE, "stories told in dates — references to the events above, never copies"),
    (KEY_SUBSETS_ADD_BUTTON, "+ Add subset"),
    (KEY_SUBSETS_CARRIED_BY_PREFIX, "Carried by"),
    (KEY_SUBSETS_GAP_COUNT_TEMPLATE, "{count} gaps"),
    (KEY_SUBSETS_REMOVED_EVENT_LINE, "removed from the chronology — Undo lives on the timeline"),
    (KEY_SUBSETS_SIZE_LINE_TEMPLATE, "A story a person can hold is 12–20 events — this one is {count}."),
    (KEY_SUBSETS_PICKER_HINT, "Tick an event to add it. Order defaults to date; drag the number to change the story order. The note is optional — one line on why this event is in the story."),
    (KEY_SUBSETS_PICKER_GAP_HINT, "Gaps are not on the chronology — add them with \"+ Add event\" on the timeline first; the picker only lists what exists."),
    (KEY_SCENARIO_VIEW_TIMELINE_BUTTON, "View Timeline"),
    (KEY_SCENARIO_TIMELINE_ROW_LABEL, "Timeline:"),
    (KEY_SCENARIO_ATTACH_LINK, "Attach…"),
    (KEY_SUBSETS_WINDOW_OPEN_TIMELINE, "Open on the timeline"),
    (KEY_SUBSETS_WINDOW_EDIT, "Edit subset"),
    (KEY_SUBSETS_WINDOW_FOOTER_TEMPLATE, "{on_chronology} on the chronology · {gaps} gaps"),
    (KEY_SUBSETS_EMPTY_STATE, "No subsets yet. A subset is a story told in dates — pick events from the phases above."),
    // Timeline subsets, task 2, seeded by the fourth migration named above.
    (KEY_SUBSETS_EVENT_COUNT_TEMPLATE, "{count} events"),
    (KEY_SUBSETS_FORM_ADD_TITLE, "Add subset"),
    (KEY_SUBSETS_PICKED_COUNT_TEMPLATE, "{count} picked"),
    (KEY_SUBSETS_PILL_GAPS_TEMPLATE, "{count} are gaps"),
    (KEY_SUBSETS_FORM_NAME_LABEL, "Name"),
    (KEY_SUBSETS_FORM_DESCRIPTION_LABEL, "Description — what this story proves, one or two sentences"),
    (KEY_SUBSETS_NOTE_PLACEHOLDER, "note"),
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

    // Phase C's three. Added when they were, because the failure they guard
    // against is the quietest one this block has: a CORRECTION migration that
    // edits a placeholder out leaves a sentence that renders with a word missing
    // and nothing anywhere that notices — "That change was not saved — " with no
    // reason, or a history line with no date on it.
    for token in ["{when}", "{who}", "{what}"] {
        assert!(
            w.history_line_template.contains(token),
            "the history line lost {token}"
        );
    }
    assert!(
        w.write_failed_template.contains("{reason}"),
        "a failed write would render with no reason in it, which is a dead \
         button with a sentence over it"
    );
    for token in ["{shown}", "{total}"] {
        assert!(
            w.picker_capped_template.contains(token),
            "the picker's cap line lost {token}, so a truncated list would say \
             it was capped without saying by how much"
        );
    }

    // Task 2's three. Each is a count and nothing else, so losing the
    // placeholder leaves the one word that was never the point — "picked" with
    // no number on a pill whose only job is the number.
    for (name, value) in [
        (
            "subsets_event_count_template",
            &w.subsets_event_count_template,
        ),
        (
            "subsets_picked_count_template",
            &w.subsets_picked_count_template,
        ),
        ("subsets_pill_gaps_template", &w.subsets_pill_gaps_template),
    ] {
        assert!(
            value.contains("{count}"),
            "{name} lost {{count}}, so it would render a label with no number in it"
        );
    }
}

/// Every template the block carries names at least one placeholder.
///
/// ## Why this is derived rather than a second hand-written list
///
/// The test above pins WHICH placeholders each template carries, and it is a
/// list somebody has to remember to extend — the Phase C gap the test-auditor
/// found on 2026-08-26 was exactly that. This one needs no extending: it walks
/// the block's own fields, and a `_template` field seeded with a sentence
/// carrying no `{` at all fails it whether or not anybody added a line above.
#[test]
fn every_field_named_a_template_actually_carries_a_placeholder() {
    let w = ChronologyWording::for_test();
    let mut checked = 0usize;
    for (key, value) in ChronologyWording::for_test_values() {
        if !key.ends_with("_template") {
            continue;
        }
        checked += 1;
        assert!(
            value.contains('{') && value.contains('}'),
            "{key} is named a template and carries no placeholder: '{value}'"
        );
    }
    // Anti-vacuity: a renamed suffix would leave this comparing nothing.
    assert!(
        checked >= 8,
        "only {checked} templates were checked; the naming convention moved"
    );
    // And the block itself is reachable, so a fixture that failed to build
    // could not leave this green.
    assert!(!w.page_title.is_empty());
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
