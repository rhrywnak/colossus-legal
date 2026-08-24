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
use crate::domain::wording::tests::{corrected_value_in, seeded_value_in};
use std::collections::HashMap;

/// The migration that seeds every row this module reads.
const SEED_MIGRATION: &str = "pipeline_migrations/20260817213319_practice_session_v0.sql";

/// Migrations that CORRECT a value the seed already wrote, or add a row to this
/// block later.
///
/// Searched BEFORE the seed, so the newest word wins. Without this the fixture
/// pins whatever the original INSERT said and goes green forever while the store
/// holds something else — the trap `scenario_practice_link_label` fell into on
/// 08-19, where fixture and code agreed with each other and disagreed with every
/// screen in the product.
const CORRECTION_MIGRATIONS: &[&str] = &[
    "pipeline_migrations/20260819202208_build_403_labels_and_chuck_view.sql",
    // The seed-question warning (2026-08-22): `practice_intro` stopped inviting a
    // witness to rehearse and started saying the deck is unreviewed.
    "pipeline_migrations/20260823101322_practice_seed_question_warning.sql",
    // The side picker (2026-08-23): `practice_redirects_subheader` stopped
    // naming an internal database value on Marie's screen.
    "pipeline_migrations/20260823231335_practice_list_side_picker.sql",
];

/// The seeded values, for TESTS ONLY — kept beside the test that pins them to
/// the migration file, so a fixture and its proof cannot drift apart.
const TEST_SEED: &[(&str, &str)] = &[
    (KEY_KICKER, "Practice session"),
    // A WARNING, not an invitation — Roman, 2026-08-22. Every deck on this system
    // is seeded from the record and unreviewed, and the line this replaced told a
    // witness to rehearse answers to questions no attorney had read.
    (
        KEY_INTRO,
        "These are seed questions, drafted from the record. An attorney must review them before anyone practises answering.",
    ),
    (KEY_WHO_HEADING, "Who's asking?"),
    (KEY_WHO_GEORGE_TITLE, "The defense asks"),
    (
        KEY_WHO_GEORGE_DETAIL,
        "Built from what they actually said in the record — the attack, turned into a question.",
    ),
    (KEY_WHO_CHUCK_TITLE, "Chuck asks"),
    (KEY_WHO_CHUCK_DETAIL, "The questions Chuck asks so you can tell it in your own words."),
    (KEY_WHO_MIXED_TITLE, "Mixed"),
    (KEY_WHO_GEORGE_TERM, "cross"),
    (KEY_WHO_CHUCK_TERM, "direct"),
    (KEY_WHO_REDIRECT_TERM, "redirect"),
    // "(dealt in Mixed)" was removed on 2026-08-23. `mixed` was a value in the
    // sessions table's `who` column, printed on screen as though it named a
    // place a person could go, and it described an interleaved list that the
    // side picker replaced. See `sideSections`.
    (KEY_REDIRECTS_SUBHEADER, "The redirects — after the defense's questions"),
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
    (KEY_PILL_GEORGE, "the defense"),
    (KEY_PILL_CHUCK, "Chuck"),
    (KEY_PILL_BRAID, "the defense · a braid"),
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
        let editor =
            crate::domain::wording_practice_editor::PracticeEditorWording::for_test_values();
        let print = crate::domain::wording_practice_print::PracticePrintWording::for_test_values();
        let list = crate::domain::wording_practice_list::PracticeListWording::for_test_values();
        build_practice_wording::<String>(|key| {
            TEST_SEED
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
                .or_else(|| flow.get(key).cloned())
                .or_else(|| row.get(key).cloned())
                .or_else(|| editor.get(key).cloned())
                .or_else(|| print.get(key).cloned())
                .or_else(|| list.get(key).cloned())
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
    let corrections: String = CORRECTION_MIGRATIONS
        .iter()
        .map(|relative| {
            std::fs::read_to_string(root.join(relative))
                .unwrap_or_else(|_| panic!("{relative} is on disk"))
        })
        .collect::<Vec<_>>()
        .join("\n");

    for key in PRACTICE_WORDING_KEYS {
        // Corrections first, then later INSERTs, then the original seed. A value
        // UPDATEd after its INSERT is the one the store actually holds, and
        // searching the seed first would pin the superseded string.
        let seeded = corrected_value_in(&corrections, key)
            .or_else(|| seeded_value_in(&corrections, key))
            .or_else(|| seeded_value_in(&sql, key))
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
