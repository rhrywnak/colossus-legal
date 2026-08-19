// Tests for `domain::wording_practice`.
//
// Same shape, and the same single justification, as every sibling wording test
// file: a key declared to the boot loader with no row in the migration makes the
// backend REFUSE TO START. That is a deploy taking DEV down, and reading the
// migration off disk is the only thing that catches it before it happens
// (Rule 21, the disk/code consistency pattern). Nothing here restates the code.
//
// This surface has a second reason to be pinned that the others do not: Marie
// reads these sentences alone, the night before she testifies. A wrong one is
// not a cosmetic defect on that screen — it is a witness being coached by a typo.

use super::*;
use crate::domain::wording::tests::seeded_value_in;
use std::collections::HashMap;

/// The migration that seeds every row this module reads.
const SEED_MIGRATION: &str = "pipeline_migrations/20260817213319_practice_session_v0.sql";

/// The seeded values, for TESTS ONLY — kept beside the test that pins them to
/// the migration file, so a fixture and its proof cannot drift apart.
const TEST_SEED: &[(&str, &str)] = &[
    (KEY_KICKER, "Practice session"),
    (
        KEY_INTRO,
        "Twenty minutes, one accusation, no clock, nobody watching. Answer out loud first, then type it in a sentence or two. You'll see your own three points after every answer.",
    ),
    (KEY_WHO_HEADING, "Who's asking?"),
    (KEY_WHO_GEORGE_TITLE, "George's side (cross)"),
    (
        KEY_WHO_GEORGE_DETAIL,
        "Questions built from what they actually said in the record — the attack, turned into a question.",
    ),
    (KEY_WHO_CHUCK_TITLE, "Chuck (direct)"),
    (KEY_WHO_CHUCK_DETAIL, "The questions Chuck asks so you can tell it in your own words."),
    (KEY_WHO_MIXED_TITLE, "Mixed"),
    (KEY_WHO_MIXED_DETAIL, "Both, in no fixed order — closest to the real day."),
    (KEY_HOW_MANY_HEADING, "How many questions?"),
    (KEY_COUNT_ALL_TEMPLATE, "all {n}"),
    (KEY_START_LABEL, "Start"),
    (KEY_ALWAYS_LABEL, "ALWAYS"),
    (
        KEY_ALWAYS_LINE,
        "Tell the truth · Answer only what's asked · \"I don't recall\" is fine if it's true · Don't guess · Pause before every answer — the pause is yours by right.",
    ),
    (KEY_LAST_SESSION_TEMPLATE, "Last session: {when} · {count} questions · {repeat} to repeat"),
    (KEY_NO_LAST_SESSION, "No session on this one yet."),
    (KEY_PROGRESS_TEMPLATE, "Question {n} of {total}"),
    (KEY_PILL_GEORGE, "George's side"),
    (KEY_PILL_CHUCK, "Chuck"),
    (KEY_PILL_BRAID, "George's side · a braid"),
    (KEY_ANSWER_LABEL, "Your answer"),
    (KEY_ANSWER_HINT, "— say it out loud, then type it."),
    (
        KEY_ANSWER_PLACEHOLDER,
        "One or two sentences. Stop when you've answered the question that was asked.",
    ),
    (KEY_ANSWER_BUTTON, "Answer"),
    (KEY_DONT_RECALL_BUTTON, "\"I don't recall.\""),
    (KEY_DONT_RECALL_TEXT, "I don't recall."),
    (KEY_PAUSE_BUTTON, "Pause — take a breath"),
    (
        KEY_PAUSE_NOTE_PREFIX,
        "Good. The pause is yours. Nobody on the stand is timing you. Now: what was the",
    ),
    (KEY_PAUSE_NOTE_EMPHASIS, "question?"),
    (KEY_EMPTY_DECK, "no practice deck yet — seed it"),
    (KEY_LOAD_FAILED, "The practice deck could not be loaded."),
    (KEY_ANSWER_FAILED, "Your answer was not recorded. Nothing was saved — try Answer again."),
    (KEY_TACTIC_BRAID_SUFFIX, "· braid"),
];

impl PracticeWording {
    /// The fixture, built through the PRODUCTION builder — so a fixture the real
    /// builder would reject cannot exist.
    pub fn for_test() -> Self {
        // The nested flow block is read by the SAME closure, so its fixture is
        // consulted here too — one builder, one rule, two tables.
        let flow = crate::domain::wording_practice_flow::PracticeFlowWording::for_test_values();
        let row = crate::domain::wording_practice_row::PracticeRowWording::for_test_values();
        build_practice_wording::<String>(|key| {
            TEST_SEED
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
                .or_else(|| flow.get(key).cloned())
                .or_else(|| row.get(key).cloned())
                .ok_or_else(|| format!("{key} is missing from TEST_SEED"))
        })
        .expect("every key in PRACTICE_WORDING_KEYS is in TEST_SEED")
    }

    /// The fixture as a key→value map, in the shape the store reads.
    pub fn for_test_values() -> HashMap<&'static str, String> {
        TEST_SEED
            .iter()
            .map(|(key, value)| (*key, (*value).to_string()))
            .collect()
    }
}

/// Every declared key is seeded by the migration, with the value this build
/// expects.
///
/// The equality half is what makes this more than an existence check: a row
/// whose wording someone edited in the migration without editing the fixture
/// would pass a "the key is present" test and ship a screen saying something
/// this build never read.
#[test]
fn every_declared_key_is_seeded_with_the_value_this_build_expects() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let sql = std::fs::read_to_string(root.join(SEED_MIGRATION))
        .expect("the practice migration is on disk");

    for key in PRACTICE_WORDING_KEYS {
        let seeded = seeded_value_in(&sql, key).unwrap_or_else(|| {
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
/// ANTI-VACUITY, and not a formality: the test above walks PRACTICE_WORDING_KEYS, so a
/// fixture entry for a key nobody declares would never be visited. Without this,
/// a key removed from the struct but left in the fixture would look tested
/// forever.
#[test]
fn the_fixture_declares_no_key_the_build_does_not_read() {
    for (key, _) in TEST_SEED {
        assert!(
            PRACTICE_WORDING_KEYS.contains(key),
            "{key} is in the fixture but no field reads it"
        );
    }
    assert_eq!(
        PRACTICE_WORDING_KEYS.len(),
        TEST_SEED.len(),
        "one key, one fixture row"
    );
}

/// A missing row is a NAMED refusal, not a blank sentence.
///
/// This is the behaviour the whole wording law rests on. It is asserted here
/// rather than assumed because the failure it prevents is silent by nature: a
/// builder that returned `String::new()` for an absent key would compile, boot,
/// and put an empty ALWAYS card under Marie's first question.
#[test]
fn a_missing_row_names_itself_rather_than_yielding_a_blank() {
    let error = build_practice_wording::<String>(|key| {
        if key == KEY_ALWAYS_LINE {
            Err("no such row".to_string())
        } else {
            Ok("x".to_string())
        }
    })
    .expect_err("the ALWAYS card's row was withheld");

    assert_eq!(error, "no such row");
}

/// The pause note carries no markup, and its halves join into the mockup's line.
///
/// ## Why this is a test and not a comment
///
/// The split exists so the store holds no `<i>`. A later editor "simplifying"
/// the row back into one sentence with a tag in it would break the law quietly —
/// the screen would still render, because React escapes the tag and shows it as
/// text to a witness.
///
/// The join assertion pins the OTHER half: the store trims, so a template cannot
/// carry a leading space and the renderer supplies the joining one.
#[test]
fn the_pause_note_carries_no_markup_and_joins_into_the_mockups_line() {
    let w = PracticeWording::for_test();

    for part in [&w.pause_note_prefix, &w.pause_note_emphasis] {
        assert!(
            !part.contains('<'),
            "a wording row must carry no markup: {part}"
        );
    }
    assert_eq!(
        format!("{} {}", w.pause_note_prefix, w.pause_note_emphasis),
        "Good. The pause is yours. Nobody on the stand is timing you. Now: what was the question?"
    );
}

/// The ALWAYS card names all five rules.
///
/// It is the floor under every read (the model is given it verbatim) and the
/// standing card on screen. A rule dropped from this row would quietly stop being
/// something the system judges by — the one wording edit with a behavioural
/// consequence, which is why it is pinned rather than left to review.
#[test]
fn the_always_card_still_carries_all_five_rules() {
    let w = PracticeWording::for_test();
    for rule in [
        "Tell the truth",
        "Answer only what's asked",
        "I don't recall",
        "Don't guess",
        "Pause before every answer",
    ] {
        assert!(w.always_line.contains(rule), "the ALWAYS card lost: {rule}");
    }
}
