// Tests for `domain::wording_practice_list`.
//
// Same law as every sibling wording test file: a key declared to the boot loader
// with no row in the migration makes the backend REFUSE TO START, and reading the
// migration off disk is the only thing that catches it before a deploy takes DEV
// down (Rule 21).
//
// This block carries a reason of its own. Two of its four rows describe a surface
// that WRITES NOTHING — practice mode makes no model call and no database write,
// and the hint beside its button is the only place a person is told what pressing
// it will do. It replaces a button called "Start" that opened a sitting and wrote
// rows, in the same position on the same page. If those two strings ever go blank
// the screen does not look broken; it looks like the old one.

use super::*;
use crate::domain::wording::tests::seeded_value_in;
use std::collections::HashMap;

/// The migration that seeds every row this module reads.
const SEED_MIGRATIONS: &[&str] = &[
    "pipeline_migrations/20260823134349_practice_one_page_l2_list_and_print_answers.sql",
    // L3's one row: the line under a one-sentence critique.
    "pipeline_migrations/20260823163653_practice_one_page_l3_plain_read_line.sql",
    // L3's question page, critique and practice walk.
    "pipeline_migrations/20260823164454_practice_one_page_l3_question_page_and_walk.sql",
];

/// The seeded values, for TESTS ONLY — kept beside the test that pins them to the
/// migration file, so a fixture and its proof cannot drift apart.
const TEST_SEED: &[(&str, &str)] = &[
    (KEY_PRACTICE_MODE_LABEL, "Practice mode"),
    (KEY_START_PRACTISING_LABEL, "Start practising"),
    (
        KEY_PRACTICE_HINT,
        "One question at a time, your answer hidden until you ask for it.",
    ),
    (
        KEY_STATUS_FOOTNOTE,
        "No date means not answered yet. That is the only status a row carries.",
    ),
    (
        KEY_READ_PLAIN_HINT,
        "This is an older read. Press Answer again for the fuller one.",
    ),
    (KEY_READ_WORKING_LABEL, "Reading your answer"),
    (KEY_READ_USUALLY_QUICK, "Usually a few seconds."),
    (KEY_READ_STILL_WORKING, "Still working — your answer is already saved either way."),
    (KEY_READ_STOP_WAITING, "Stop waiting"),
    (KEY_READ_WHY_LABEL, "Why"),
    (KEY_READ_POINTERS_LABEL, "What to do instead"),
    (KEY_READ_SOURCE_MISSING, "this source was not sent — report it"),
    (KEY_READ_UNREVIEWED, "Chuck has not reviewed this."),
    (KEY_READ_WRONG_LABEL, "This is wrong →"),
    (KEY_EARLIER_VERSIONS_TEMPLATE, "▸ {n} earlier versions"),
    (KEY_EARLIER_VERSION_ONE, "▸ 1 earlier version"),
    (KEY_YOUR_ANSWER_DATED_TEMPLATE, "Your answer · {when}"),
    (KEY_SHOW_ANSWER_LABEL, "Show my answer"),
    (KEY_NEXT_QUESTION_LABEL, "Next question ▸"),
    (KEY_CHANGE_ANSWER_LABEL, "Change this answer"),
    (KEY_PRACTICE_COUNTER_TEMPLATE, "PRACTICE · {side} · {n} OF {m}"),
    (KEY_PRACTICE_SAY_ALOUD, "Say your answer out loud."),
    (KEY_PRACTICE_THEN_PRESS_TEMPLATE, "Then press {label} to see what you wrote."),
    (KEY_PRACTICE_SKIP_HINT, "To skip it, just press Next question."),
    (KEY_PRACTICE_END_TITLE, "That's all of them."),
    (KEY_PRACTICE_END_COUNT_TEMPLATE, "{n} questions from {side}."),
    (KEY_PRACTISE_AGAIN_LABEL, "Practise them again"),
    (KEY_PRACTICE_NONE_ANSWERED, "There is nothing to practise yet — practice walks the questions you have already answered."),
    (KEY_DECK_QUESTION_MISSING, "That question is no longer in this deck."),
];

impl PracticeListWording {
    /// The fixture, built through the PRODUCTION builder — so a fixture the real
    /// builder would reject cannot exist.
    pub fn for_test() -> Self {
        build_practice_list_wording::<String>(|key| {
            TEST_SEED
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
                .ok_or_else(|| format!("{key} is missing from TEST_SEED"))
        })
        .expect("every key in PRACTICE_LIST_WORDING_KEYS is in TEST_SEED")
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
fn every_declared_key_is_seeded_with_the_value_this_build_expects() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let sources: Vec<String> = SEED_MIGRATIONS
        .iter()
        .map(|file| {
            std::fs::read_to_string(root.join(file))
                .unwrap_or_else(|cause| panic!("{file} is not on disk: {cause}"))
        })
        .collect();

    for key in PRACTICE_LIST_WORDING_KEYS {
        let seeded = sources
            .iter()
            .find_map(|sql| seeded_value_in(sql, key))
            .unwrap_or_else(|| {
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

/// The fixture holds nothing the boot loader does not read.
///
/// ANTI-VACUITY, and not a formality: the test above walks
/// `PRACTICE_LIST_WORDING_KEYS`, so a fixture entry for a key nobody declares
/// would never be visited. Without this, a key removed from the struct but left
/// in the fixture would look tested forever.
#[test]
fn the_fixture_declares_no_key_the_build_does_not_read() {
    for (key, _) in TEST_SEED {
        assert!(
            PRACTICE_LIST_WORDING_KEYS.contains(key),
            "{key} is in TEST_SEED but declared nowhere — it is tested by nothing"
        );
    }
}

/// The hint says what the control does, and does not merely name it again.
///
/// Standing rule of 2026-08-19: no control on a practice page is dim and silent.
/// This one replaces a button in the same position that used to open a sitting
/// and write rows, so a hint that read "Start practising" a second time would
/// leave a person with no way to tell the two apart.
#[test]
fn the_hint_tells_a_person_something_the_button_does_not() {
    let wording = PracticeListWording::for_test();

    assert_ne!(wording.practice_hint, wording.start_practising_label);
    assert!(
        wording.practice_hint.len() > wording.start_practising_label.len(),
        "a hint no longer than its button explains nothing"
    );
}

/// The footnote explains the ABSENCE of the marks, not their presence.
///
/// It exists only because `answered today · repeat · attempt 2` was removed. If
/// it ever stops naming the absent case it stops doing the one job it has.
#[test]
fn the_footnote_names_the_case_that_shows_nothing() {
    let wording = PracticeListWording::for_test();

    assert!(
        wording.status_footnote.to_lowercase().contains("no date"),
        "the footnote must name the empty case: {:?}",
        wording.status_footnote
    );
}

/// A missing row refuses the block and says which key.
///
/// Standing Rule 1: the boot failure names what is wrong. "Settings failed to
/// load" sends an operator to the wrong place.
#[test]
fn a_missing_row_refuses_the_block_and_names_the_key() {
    let refused = build_practice_list_wording::<String>(|key| Err(format!("no row for {key}")));
    let reason = refused.expect_err("a reader that supplies nothing cannot build the block");

    assert!(
        PRACTICE_LIST_WORDING_KEYS
            .iter()
            .any(|key| reason.contains(key)),
        "the refusal must name a declared key, got {reason:?}"
    );
}
