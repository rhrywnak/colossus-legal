// Tests for `domain::wording_rehearsal` — task 2.11 B2's forty strings.
//
// This file also OWNS the seeded fixture. It sits here rather than in the module
// for two reasons: the module is at Rule 17's limit with the production shape
// alone, and the fixture belongs beside the test that pins it to the migration —
// a fixture and its proof one file apart can drift; in the same file they cannot.
//
// The one thing that must be proved above all others is that the fixture still
// says what the MIGRATION seeds. A fixture compared only to itself proves
// nothing, and every assertion elsewhere that mentions one of these sentences
// would then be describing a product that says something else.

use super::*;
use crate::domain::wording::tests::seeded_value_in;
use crate::domain::wording_templates::{missing_placeholders, REQUIRED_PLACEHOLDERS};
use std::collections::HashMap;

/// The migrations that seed these rows, in the order they run.
///
/// Two files since task 2.11 C: B2 seeded forty and C added the fifth section
/// state. A single-file constant would have made the new row look unseeded, and
/// the honest fix is to read both rather than to relax the assertion — the whole
/// value of this test is that it reads the SQL a deployment actually applies.
const SEED_MIGRATIONS: &[&str] = &[
    "pipeline_migrations/20260806100704_rehearsal_render_wording.sql",
    "pipeline_migrations/20260806135509_rehearsal_visual_2_11c.sql",
    "pipeline_migrations/20260810094435_r1_390_rehearsal_gate_wording_and_response_uniqueness.sql",
];

/// The seeded values, for TESTS ONLY — one table, feeding both fixtures.
///
/// ## Why `cfg(test)`, exactly like its two siblings
///
/// A production-reachable default would be a compiled-in set of user-facing
/// strings — the precise defect Roman's 2026-08-04 ruling deletes — and it would
/// be reachable by accident through `unwrap_or_default()`. Gating it on
/// `cfg(test)` means it cannot exist in a release binary at all.
///
/// The values match the migration's seed, and `the_fixture_carries_the_values_
/// the_migration_actually_seeds` asserts that against the migration FILE.
const TEST_SEED: &[(&str, &str)] = &[
    (KEY_PAGE_HEADING, "Rehearsal"),
    (
        KEY_PURPOSE_LINE,
        "Your testimony-prep view — what they say, every time they said it, and \
         what we say back. Only scenarios marked Ready appear here.",
    ),
    (KEY_POSITION_TEMPLATE, "Scenario {n} of {total}"),
    (KEY_PREVIOUS_LABEL, "Back"),
    (KEY_NEXT_LABEL, "Next"),
    (
        KEY_NOTHING_READY,
        "Nothing is ready to rehearse yet. A scenario appears here once someone \
         switches it to Ready on its page.",
    ),
    (KEY_PICKER_HEADING, "Choose a scenario to rehearse"),
    (
        KEY_NOT_READY,
        "{code} is not ready to rehearse yet. A scenario appears here once someone \
         switches it to Ready on its page.",
    ),
    (KEY_EXPAND_ALL, "Open everything"),
    (KEY_COLLAPSE_ALL, "Fold everything"),
    (KEY_BLOCK_WHAT, "What this is"),
    (
        KEY_BLOCK_ACCUSATION,
        "The accusation, and every time they made it",
    ),
    (KEY_BLOCK_TIMELINE, "The timeline"),
    (KEY_BLOCK_POINTS, "Your points, in your words"),
    (KEY_BLOCK_WATCH, "Watch for"),
    (KEY_ALWAYS_HEADING, "Always"),
    (
        KEY_ALWAYS_LINES,
        "Tell the truth. · Answer only what's asked. · \"I don't recall\" is fine \
         if it's true. · Don't guess.",
    ),
    (KEY_ACCUSATION_HEADER, "said {times} times · {gaps} gaps"),
    (KEY_TIMELINE_HEADER, "{entries} dated items"),
    (KEY_POINTS_HEADER, "{shown} of {cap}"),
    (KEY_WATCH_HEADER, "{count} to watch for"),
    (
        KEY_WHAT_GAP,
        "Nobody has written what this scenario is about yet.",
    ),
    (
        KEY_ACCUSATION_TEXT_GAP,
        "Nobody has written the accusation in plain words yet.",
    ),
    (
        KEY_NO_INSTANCES,
        "Nobody has marked any instances of this accusation yet.",
    ),
    (
        KEY_GAP_NO_ANSWER,
        "NO ANSWER PREPARED — {who}, {when}, {where}",
    ),
    (
        KEY_GAP_ACCUSATION_REMOVED,
        "An answer is paired to a statement that is no longer part of this scenario.",
    ),
    (
        KEY_GAP_ANSWER_REMOVED,
        "The answer to {who}, {when} is no longer part of this scenario.",
    ),
    (
        KEY_GAP_UNAVAILABLE,
        "One statement marked as an instance could not be loaded from the record.",
    ),
    (KEY_POINTS_GAP, "No talking points yet."),
    (KEY_WATCH_GAP, "Nothing flagged yet."),
    (KEY_INSTANCE_WHEN_GAP, "No date on this statement"),
    (KEY_INSTANCE_WHO_GAP, "Speaker not recorded"),
    (
        KEY_TIMELINE_GAP,
        "Not enough dated items to draw a timeline — {undated} of {total} carry no \
         date.",
    ),
    (KEY_TIMELINE_FILTER_PROMPT, "Show"),
    (KEY_TIMELINE_FILTER_ALL, "Everyone"),
    (KEY_SOURCE_LABEL, "{document}, p. {page}"),
    (KEY_SOURCE_OPEN, "Open"),
    (KEY_WHAT_STATE, "open"),
    (KEY_ACCUSATION_STATE, "open"),
    (KEY_TIMELINE_STATE, "collapsed"),
    (KEY_POINTS_STATE, "open"),
    (KEY_WATCH_STATE, "open"),
];

impl RehearsalWording {
    /// The seeded wording, built through the SAME function production uses.
    ///
    /// Not a hand-typed struct literal: routing the fixture through
    /// `build_rehearsal_wording` means every test that touches it also exercises
    /// the builder, and a field wired to the wrong key fails here rather than on
    /// screen.
    pub fn for_test() -> Self {
        build_rehearsal_wording::<String>(|key| {
            TEST_SEED
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
                .ok_or_else(|| format!("{key} is missing from TEST_SEED"))
        })
        .expect("every key in REHEARSAL_WORDING_KEYS is in TEST_SEED")
    }

    /// The fixture as a key→value map, in the shape the store reads.
    pub fn for_test_values() -> HashMap<&'static str, String> {
        TEST_SEED
            .iter()
            .map(|(key, value)| (*key, (*value).to_string()))
            .collect()
    }
}

// ── The fixture is checked against the MIGRATION, not against itself ─────────

#[test]
fn the_fixture_carries_the_values_the_migration_actually_seeds() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    // Concatenated rather than searched one at a time: every key is seeded
    // exactly once across the two files (`no_key_appears_twice…` proves the list
    // has no duplicate, and ON CONFLICT DO NOTHING means a second INSERT of one
    // would be inert anyway), so the first match found is the seeded value.
    let sql: String = SEED_MIGRATIONS
        .iter()
        .map(|file| {
            std::fs::read_to_string(root.join(file))
                .unwrap_or_else(|e| panic!("{file} is on disk: {e}"))
        })
        .collect::<Vec<_>>()
        .join("\n");

    let fixture = RehearsalWording::for_test_values();
    let mut checked = 0usize;

    for key in REHEARSAL_WORDING_KEYS {
        let seeded = seeded_value_in(&sql, key)
            .unwrap_or_else(|| panic!("{key} is not seeded by the migration"));
        let in_fixture = fixture
            .get(*key)
            .unwrap_or_else(|| panic!("{key} is missing from TEST_SEED"));

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
    assert_eq!(checked, 42, "all forty-two stored strings must be compared");
    assert_eq!(REHEARSAL_WORDING_KEYS.len(), 42);
}

#[test]
fn no_key_appears_twice_and_none_collides_with_a_sibling_list() {
    // All four stored-string lists key the SAME `app_settings` table. A collision
    // would mean two struct fields fed by one row — one surface silently editing
    // another's words, with the boot loader perfectly happy.
    let mut seen: Vec<&str> = REHEARSAL_WORDING_KEYS.to_vec();
    seen.sort_unstable();
    let before = seen.len();
    seen.dedup();
    assert_eq!(before, seen.len(), "a key is listed twice");

    for key in REHEARSAL_WORDING_KEYS {
        assert!(
            !crate::domain::wording::WORDING_KEYS.contains(key),
            "{key} is also a curation-surface key"
        );
        assert!(
            !crate::domain::wording_accusation::ACCUSATION_WORDING_KEYS.contains(key),
            "{key} is also a working-view accusation key"
        );
    }
}

#[test]
fn every_field_is_read_from_its_own_key() {
    // Anti-drift of the copy-paste kind: `next_label: read(KEY_PREVIOUS_LABEL)`
    // compiles, type-checks, and puts "Back" on the Next button. Feeding each key
    // its own name back proves the wiring one field at a time.
    let w = build_rehearsal_wording::<std::convert::Infallible>(|key| Ok(key.to_string()))
        .expect("the identity reader cannot fail");

    assert_eq!(w.page_heading, KEY_PAGE_HEADING);
    assert_eq!(w.purpose_line, KEY_PURPOSE_LINE);
    assert_eq!(w.position_template, KEY_POSITION_TEMPLATE);
    assert_eq!(w.previous_label, KEY_PREVIOUS_LABEL);
    assert_eq!(w.next_label, KEY_NEXT_LABEL);
    assert_eq!(w.nothing_ready_notice, KEY_NOTHING_READY);
    assert_eq!(w.picker_heading, KEY_PICKER_HEADING);
    assert_eq!(w.not_ready_notice, KEY_NOT_READY);
    assert_eq!(w.expand_all_label, KEY_EXPAND_ALL);
    assert_eq!(w.collapse_all_label, KEY_COLLAPSE_ALL);
    assert_eq!(w.block_what_heading, KEY_BLOCK_WHAT);
    assert_eq!(w.block_accusation_heading, KEY_BLOCK_ACCUSATION);
    assert_eq!(w.block_timeline_heading, KEY_BLOCK_TIMELINE);
    assert_eq!(w.block_points_heading, KEY_BLOCK_POINTS);
    assert_eq!(w.block_watch_heading, KEY_BLOCK_WATCH);
    assert_eq!(w.always_heading, KEY_ALWAYS_HEADING);
    assert_eq!(w.always_lines, KEY_ALWAYS_LINES);
    assert_eq!(w.accusation_header_template, KEY_ACCUSATION_HEADER);
    assert_eq!(w.timeline_header_template, KEY_TIMELINE_HEADER);
    assert_eq!(w.points_header_template, KEY_POINTS_HEADER);
    assert_eq!(w.watch_header_template, KEY_WATCH_HEADER);
    assert_eq!(w.what_gap, KEY_WHAT_GAP);
    assert_eq!(w.accusation_text_gap, KEY_ACCUSATION_TEXT_GAP);
    assert_eq!(w.no_instances_notice, KEY_NO_INSTANCES);
    assert_eq!(w.gap_no_answer, KEY_GAP_NO_ANSWER);
    assert_eq!(w.gap_accusation_removed, KEY_GAP_ACCUSATION_REMOVED);
    assert_eq!(w.gap_answer_removed, KEY_GAP_ANSWER_REMOVED);
    assert_eq!(w.gap_instance_unavailable, KEY_GAP_UNAVAILABLE);
    assert_eq!(w.points_gap, KEY_POINTS_GAP);
    assert_eq!(w.watch_gap, KEY_WATCH_GAP);
    assert_eq!(w.instance_when_gap, KEY_INSTANCE_WHEN_GAP);
    assert_eq!(w.instance_who_gap, KEY_INSTANCE_WHO_GAP);
    assert_eq!(w.timeline_gap_template, KEY_TIMELINE_GAP);
    assert_eq!(w.timeline_filter_prompt, KEY_TIMELINE_FILTER_PROMPT);
    assert_eq!(w.timeline_filter_all_label, KEY_TIMELINE_FILTER_ALL);
    assert_eq!(w.source_label_template, KEY_SOURCE_LABEL);
    assert_eq!(w.source_open_label, KEY_SOURCE_OPEN);
    assert_eq!(w.what_default_state, KEY_WHAT_STATE);
    assert_eq!(w.accusation_default_state, KEY_ACCUSATION_STATE);
    assert_eq!(w.timeline_default_state, KEY_TIMELINE_STATE);
    assert_eq!(w.points_default_state, KEY_POINTS_STATE);
    assert_eq!(w.watch_default_state, KEY_WATCH_STATE);
}

#[test]
fn a_missing_row_refuses_the_whole_build_and_names_the_key() {
    // The failure law: a string is a stored parameter with the standing of a
    // threshold. A missing label must not degrade to an empty control — that is a
    // compiled-in default by omission.
    for missing in REHEARSAL_WORDING_KEYS {
        let values = RehearsalWording::for_test_values();
        let result = build_rehearsal_wording(|key| {
            if key == *missing {
                return Err(format!("no parameter named '{key}' is stored"));
            }
            Ok(values.get(key).expect("seeded").clone())
        });

        let Err(error) = result else {
            panic!("a store missing {missing} must not produce a wording block");
        };
        assert!(error.contains(missing), "the refusal must name it: {error}");
    }
}

// ── The Always card: the one block that must never render empty ──────────────

#[test]
fn the_always_card_splits_into_the_four_doctrine_lines() {
    let lines = RehearsalWording::for_test()
        .always_lines()
        .expect("four lines");
    assert_eq!(
        lines.len(),
        4,
        "ABA 508's substance is four sentences: {lines:?}"
    );
    assert_eq!(lines[0], "Tell the truth.");
    assert!(lines.iter().any(|l| l.contains("I don't recall")));
    assert!(lines.iter().any(|l| l.contains("Don't guess")));
    assert!(lines.iter().any(|l| l.contains("Answer only what's asked")));
}

#[test]
fn the_separator_is_not_left_clinging_to_a_line() {
    // A value typed "a·b" or "a  ·  b" must still yield clean lines, or the card
    // renders punctuation a human never meant to read aloud.
    let mut w = RehearsalWording::for_test();
    w.always_lines = "Tell the truth.  ·   Don't guess.".to_string();
    assert_eq!(
        w.always_lines().expect("two lines"),
        vec!["Tell the truth.".to_string(), "Don't guess.".to_string()]
    );
}

#[test]
fn a_card_edited_to_nothing_is_refused_rather_than_rendered_empty() {
    // §10 makes this the block that is never scrolled away from. A value edited to
    // blank would render a bordered box with a heading over it — worse than an
    // error, because it looks deliberate.
    for blank in ["", "   ", " · ", " ·  · "] {
        let mut w = RehearsalWording::for_test();
        w.always_lines = blank.to_string();
        let Err(error) = w.always_lines() else {
            panic!("a card with no lines must be refused: {blank:?}");
        };
        assert!(
            error.to_string().contains("rehearsal_always_lines"),
            "the refusal must name the row: {error}"
        );
    }
}

#[test]
fn one_line_is_a_legal_card() {
    // Refusing this would be the code deciding editorial policy. Four is the
    // doctrine and the seed; one is a human's deliberate choice, and it renders.
    let mut w = RehearsalWording::for_test();
    w.always_lines = "Tell the truth.".to_string();
    assert_eq!(w.always_lines().expect("one line").len(), 1);
}

// ── The section states ───────────────────────────────────────────────────────

#[test]
fn the_seeded_states_are_the_ones_the_addendum_specifies() {
    // Accusation open (it is the page), timeline collapsed (doctrine: one topic at
    // a time; the timeline is the overview, not the work), the other two open.
    let states = RehearsalWording::for_test()
        .section_states()
        .expect("valid");
    let by_key: HashMap<&str, SectionState> = states.into_iter().collect();

    assert!(by_key[KEY_WHAT_STATE].is_open());
    assert!(by_key[KEY_ACCUSATION_STATE].is_open());
    assert!(!by_key[KEY_TIMELINE_STATE].is_open());
    assert!(by_key[KEY_POINTS_STATE].is_open());
    assert!(by_key[KEY_WATCH_STATE].is_open());
}

#[test]
fn a_typo_in_a_state_row_is_refused_and_names_its_key() {
    // The whole reason this is a parsed enum: `value == "open"` would treat every
    // typo as `collapsed` and quietly fold a section a witness needs, with nothing
    // in the log.
    for token in ["Open", "opne", "true", "1", ""] {
        let mut w = RehearsalWording::for_test();
        w.timeline_default_state = token.to_string();
        let Err(error) = w.section_states() else {
            panic!("{token:?} must be refused, not read as a state");
        };
        assert!(
            error.to_string().contains(KEY_TIMELINE_STATE),
            "the refusal must name the row: {error}"
        );
    }
}

#[test]
fn a_state_token_survives_surrounding_whitespace() {
    // Trailing whitespace is invisible in psql, and refusing it would be a refusal
    // a human cannot see the cause of.
    assert!(SectionState::parse(KEY_POINTS_STATE, "  open  ")
        .expect("trimmed")
        .is_open());
}

// ── The templates that must keep their facts ─────────────────────────────────

#[test]
fn every_template_that_carries_a_fact_is_guarded() {
    // Without an entry in `REQUIRED_PLACEHOLDERS` a key is silently unconstrained,
    // and an edit could ship "said  times ·  gaps" — grammatical, renders
    // perfectly, and states nothing. On the collapsed accusation header that
    // sentence would be a prep list reporting no work.
    let guarded: Vec<&str> = REQUIRED_PLACEHOLDERS
        .iter()
        .map(|(key, _)| *key)
        .filter(|key| REHEARSAL_WORDING_KEYS.contains(key))
        .collect();

    assert_eq!(
        guarded.len(),
        10,
        "position · not-ready · four headers · two gaps · timeline gap · source: {guarded:?}"
    );

    assert_eq!(
        missing_placeholders(KEY_ACCUSATION_HEADER, "said a lot"),
        vec!["{times}", "{gaps}"]
    );
    assert_eq!(
        missing_placeholders(KEY_GAP_NO_ANSWER, "NO ANSWER PREPARED"),
        vec!["{who}", "{when}", "{where}"]
    );
    assert!(missing_placeholders(KEY_SOURCE_LABEL, "{document}, p. {page}").is_empty());
}

#[test]
fn the_seeded_defaults_satisfy_their_own_placeholder_rules() {
    // If they did not, the migration would seed a store the write path refuses —
    // a value nobody could edit back to its own default.
    let values = RehearsalWording::for_test_values();
    for (key, _) in REQUIRED_PLACEHOLDERS
        .iter()
        .filter(|(k, _)| REHEARSAL_WORDING_KEYS.contains(k))
    {
        let seeded = values.get(key).expect("a seeded value");
        assert!(
            missing_placeholders(key, seeded).is_empty(),
            "the seeded default for {key} does not satisfy its own rule: {seeded}"
        );
    }
}

#[test]
fn no_seeded_string_is_blank() {
    // A blank label is an invisible control. The store refuses one on write; this
    // pins the DEFAULTS, which the write path never sees.
    for (key, value) in RehearsalWording::for_test_values() {
        assert!(!value.trim().is_empty(), "{key} seeds a blank string");
    }
}

#[test]
fn this_surface_never_names_a_fact_by_its_working_view_handle() {
    // §10 keeps internal identifiers off the witness surface. B1's gap rows name
    // facts "C-14"; these name who, when and where instead. A future edit that
    // reached for the handle would be reaching for working-view vocabulary.
    for (key, value) in RehearsalWording::for_test_values() {
        assert!(
            !value.contains("{code}") || key == KEY_NOT_READY,
            "{key} carries a {{code}} placeholder; only the not-ready notice may, \
             and it means the SCENARIO handle S-2, not a candidate's C-14"
        );
    }
}

#[test]
fn the_five_states_arrive_in_the_order_the_page_renders_them() {
    // The caller reads this array POSITIONALLY into `RehearsalCollapse`. Task
    // 2.11 C put "What this is" at the front, which shifted every other index by
    // one — and an index read out of step opens the wrong section for a witness,
    // silently, with the page looking perfectly fine.
    let states = RehearsalWording::for_test()
        .section_states()
        .expect("valid");
    let keys: Vec<&str> = states.iter().map(|(key, _)| *key).collect();

    assert_eq!(
        keys,
        vec![
            KEY_WHAT_STATE,
            KEY_ACCUSATION_STATE,
            KEY_TIMELINE_STATE,
            KEY_POINTS_STATE,
            KEY_WATCH_STATE,
        ],
        "page order: what → accusation → timeline → points → watch"
    );
}
